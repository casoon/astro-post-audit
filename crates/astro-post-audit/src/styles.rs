//! Die Stylesheets einer gebauten Seite, von der Platte.
//!
//! Im Build liegt alles Nötige vor: `<link rel=stylesheet href>` zeigt auf eine
//! Datei in `dist/`, `<style>` steht im HTML, ein `@import` auf eine lokale
//! Datei lässt sich auflösen. Dieses Modul sucht die Blätter einer Seite,
//! liest sie (einmal je Datei für den ganzen Lauf, viele Seiten teilen sich
//! dasselbe CSS) und gibt die geparsten Regeln in Dokumentreihenfolge zurück.
//!
//! Was nicht auf der Platte liegt -- Fernadressen, fehlende Dateien, Pfade
//! außerhalb von `dist/` -- wird **nicht** geraten: Es ergibt einen Vermerk
//! (`InputNote`) und ein unvollständiges Blatt. Mit einem unvollständigen
//! Blatt behauptet der Host keine Farbe, der Kontrast bleibt `untested`.
//! `a11y-static` löst keine URLs auf und liest keine Dateien; das ist hier
//! die Aufgabe des Hosts.

use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use a11y_report::{InputKind, InputNote};
use scraper::{Html, Selector};
use style_cascade::Import;
use stylesheet_parse::{parse_stylesheet, MediaRule, Rule, Stylesheet};

use crate::discovery::{PageInfo, SiteIndex};

/// Tiefe, ab der verschachtelte `@import`-Ketten abbrechen (wie in `a11y-static`).
const MAX_IMPORT_TIEFE: usize = 16;

/// Ein gelesenes Blatt mit aufgelösten lokalen `@import`s.
#[derive(Debug)]
pub struct Geladen {
    rules: Vec<Rule>,
    /// Vorspann des `@import` und Grund, warum er offen blieb.
    offen: Vec<(String, String)>,
    /// Ob das Blatt (oder ein Import) nach `prefers-color-scheme` fragt.
    dunkel: bool,
}

type Gelesen = Result<Arc<Geladen>, String>;

/// Zwischenspeicher und gesammelte Vermerke für einen Lauf.
///
/// Gehört zum `SiteIndex`: Die Checks bekommen nur den Index, und die Blätter
/// sind eine Eigenschaft der Site, nicht einer Seite.
#[derive(Debug, Default)]
pub struct StyleStore {
    dateien: Mutex<HashMap<PathBuf, Gelesen>>,
    /// (Art, Gegenstand, Grund) -> Zahl der Seiten.
    vermerke: Mutex<BTreeMap<(u8, String, String), usize>>,
}

/// Die Blätter einer Seite, bereit für die Kaskade.
pub struct SeitenStile {
    pub blaetter: Vec<Stylesheet>,
    /// Mindestens ein `<link>` ohne nutzbares Blatt.
    pub fehlende: bool,
    /// Ein Blatt fragt nach `prefers-color-scheme`: ein zweiter Lauf im dunklen
    /// Schema kann andere Farben ergeben.
    pub dunkel: bool,
    /// Vorspänne offen gebliebener `@import`s mit Grund.
    pub offene_imports: Vec<(String, String)>,
}

enum Ref {
    Link { href: String, media: Option<String> },
    Inline { css: String, media: Option<String> },
    LeererLink,
}

fn nicht_leer(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Die Blätter der Seite in Dokumentreihenfolge. Die Auswahl folgt den
/// HTML-Regeln wie in `a11y-static`: `<link>` mit dem `rel`-Token
/// `stylesheet`, ohne `alternate`, ohne `disabled`; jedes `<style>`; nichts
/// innerhalb von `<template>`.
fn refs(html: &Html) -> Vec<Ref> {
    let sel = Selector::parse("link, style").expect("static selector");
    html.select(&sel)
        .filter(|e| {
            !e.ancestors().any(|a| {
                a.value()
                    .as_element()
                    .is_some_and(|el| el.name() == "template")
            })
        })
        .filter_map(|e| {
            let el = e.value();
            let media = nicht_leer(el.attr("media"));
            if el.name() == "style" {
                return Some(Ref::Inline {
                    css: e.text().collect(),
                    media,
                });
            }
            let rel = el.attr("rel").unwrap_or("");
            let hat = |t: &str| rel.split_whitespace().any(|r| r.eq_ignore_ascii_case(t));
            if hat("stylesheet") && !hat("alternate") && el.attr("disabled").is_none() {
                return Some(match nicht_leer(el.attr("href")) {
                    Some(href) => Ref::Link { href, media },
                    None => Ref::LeererLink,
                });
            }
            None
        })
        .collect()
}

/// `@media`-Bedingung des Links um die Regeln; `@import` bleibt oben, dort
/// sucht es die Kaskade.
fn umhuellen(rules: Vec<Rule>, media: Option<&str>) -> Stylesheet {
    let Some(bedingung) = media else {
        return Stylesheet { rules };
    };
    let (mut oben, innen): (Vec<_>, Vec<_>) = rules
        .into_iter()
        .partition(|r| matches!(r, Rule::Other(a) if a.name == "import"));
    oben.push(Rule::Media(MediaRule {
        condition: bedingung.to_string(),
        rules: innen,
    }));
    Stylesheet { rules: oben }
}

/// Sinngemäß `Url::join` für die zwei Fälle im Build: relativ zur Seite und
/// wurzelrelativ. Alles mit Schema oder `//` ist eine Fernadresse.
fn aufloesen(index: &SiteIndex, von_datei: &Path, url: &str) -> Result<PathBuf, String> {
    let url = url.trim();
    let ohne = url.split(['#', '?']).next().unwrap_or("");
    let schema = ohne.split('/').next().is_some_and(|s| s.contains(':'));
    if ohne.starts_with("//") || schema {
        return Err(format!("{url}: remote or non-file URL, not on disk"));
    }
    if ohne.is_empty() {
        return Err("empty URL".into());
    }
    let dekodiert = percent_encoding::percent_decode_str(ohne)
        .decode_utf8()
        .map_err(|_| format!("{url}: not valid UTF-8 after percent-decoding"))?;
    let roh = if dekodiert.starts_with('/') {
        index
            .dist_path_for(&dekodiert)
            .ok_or_else(|| format!("{url}: outside the Astro base, not served by this build"))?
    } else {
        von_datei
            .parent()
            .unwrap_or(&index.dist_path)
            .join(dekodiert.as_ref())
    };
    // `..` lexikalisch auflösen; die Datei muss unter `dist/` bleiben.
    let mut out = PathBuf::new();
    for c in roh.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            c => out.push(c),
        }
    }
    if !out.starts_with(&index.dist_path) {
        return Err(format!("{url}: resolves outside the dist directory"));
    }
    Ok(out)
}

impl StyleStore {
    fn vermerke_zu(&self, art: InputKind, gegenstand: &str, grund: &str) {
        let k = match art {
            InputKind::StylesheetUnavailable => 0,
            _ => 1,
        };
        *self
            .vermerke
            .lock()
            .unwrap()
            .entry((k, gegenstand.to_string(), grund.to_string()))
            .or_default() += 1;
    }

    /// Ein `@import`, den die Kaskade nicht auflösen konnte, den aber niemand
    /// vermerkt hat (etwa weil der Vorspann nicht zu lesen war).
    pub fn unaufgeloest_nachtragen(&self, vorspann: &str, bekannt: &[(String, String)]) {
        if !bekannt.iter().any(|(v, _)| v == vorspann) {
            self.vermerke_zu(
                InputKind::StylesheetUnresolved,
                vorspann,
                "@import is not resolved; styles that depend on it are undetermined",
            );
        }
    }

    /// Die Vermerke des Laufs, je Blatt und Grund einmal, mit Seitenzahl.
    pub fn vermerke(&self) -> Vec<InputNote> {
        self.vermerke
            .lock()
            .unwrap()
            .iter()
            .map(|((k, gegenstand, grund), seiten)| {
                let art = if *k == 0 {
                    InputKind::StylesheetUnavailable
                } else {
                    InputKind::StylesheetUnresolved
                };
                InputNote::new(art)
                    .with_subject(gegenstand.clone())
                    .with_detail(format!("{grund} ({seiten} page(s))"))
            })
            .collect()
    }

    /// Liest und parst eine Datei einmal; spätere Seiten bekommen dieselben Regeln.
    fn lade(&self, index: &SiteIndex, pfad: &Path, kette: &mut Vec<PathBuf>) -> Gelesen {
        if let Some(g) = self.dateien.lock().unwrap().get(pfad) {
            return g.clone();
        }
        let gelesen = std::fs::read_to_string(pfad)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => "file not found in dist".to_string(),
                _ => format!("cannot read file: {e}"),
            })
            .map(|css| {
                kette.push(pfad.to_path_buf());
                let g = self.expandieren(index, &css, pfad, kette);
                kette.pop();
                Arc::new(g)
            });
        self.dateien
            .lock()
            .unwrap()
            .insert(pfad.to_path_buf(), gelesen.clone());
        gelesen
    }

    /// Parst `css` und ersetzt jedes `@import` einer lokalen Datei durch deren
    /// Regeln, eingewickelt, wie der Import es verlangt (`layer`, `supports`,
    /// Medienliste). Was nicht auf der Platte liegt, bleibt als `@import`
    /// stehen -- die Kaskade meldet es dann als unaufgelöst.
    fn expandieren(
        &self,
        index: &SiteIndex,
        css: &str,
        von_datei: &Path,
        kette: &mut Vec<PathBuf>,
    ) -> Geladen {
        let mut g = Geladen {
            rules: Vec::new(),
            offen: Vec::new(),
            dunkel: css.contains("prefers-color-scheme"),
        };
        for rule in parse_stylesheet(css).rules {
            let Rule::Other(at) = &rule else {
                g.rules.push(rule);
                continue;
            };
            if at.name != "import" {
                g.rules.push(rule);
                continue;
            }
            let vorspann = at.prelude.trim().to_string();
            let Some(import) = Import::parse(&vorspann) else {
                g.rules.push(rule);
                continue;
            };
            let ziel = aufloesen(index, von_datei, &import.url).and_then(|p| {
                if kette.contains(&p) || kette.len() >= MAX_IMPORT_TIEFE {
                    Err("@import cycle or nesting too deep".into())
                } else {
                    self.lade(index, &p, kette).map(|geladen| (p, geladen))
                }
            });
            match ziel {
                Ok((_, geladen)) => {
                    g.dunkel |= geladen.dunkel;
                    g.offen.extend(geladen.offen.iter().cloned());
                    g.rules.extend(import.wrap(geladen.rules.clone()));
                }
                Err(grund) => {
                    g.offen.push((vorspann, grund));
                    g.rules.push(rule);
                }
            }
        }
        g
    }

    /// Alle Blätter der Seite: Links von der Platte, `<style>` aus dem HTML.
    pub fn fuer_seite(&self, index: &SiteIndex, page: &PageInfo, html: &Html) -> SeitenStile {
        let seite = index.dist_path.join(&page.rel_path);
        let mut aus = SeitenStile {
            blaetter: Vec::new(),
            fehlende: false,
            dunkel: false,
            offene_imports: Vec::new(),
        };
        for r in refs(html) {
            match r {
                Ref::LeererLink => {
                    aus.fehlende = true;
                    self.vermerke_zu(
                        InputKind::StylesheetUnavailable,
                        &page.rel_path,
                        "<link rel=stylesheet> has an empty or missing href",
                    );
                }
                Ref::Inline { css, media } => {
                    let g = self.expandieren(index, &css, &seite, &mut Vec::new());
                    aus.dunkel |= g.dunkel;
                    aus.offene_imports.extend(g.offen);
                    aus.blaetter.push(umhuellen(g.rules, media.as_deref()));
                }
                Ref::Link { href, media } => {
                    let gelesen = aufloesen(index, &seite, &href)
                        .and_then(|p| self.lade(index, &p, &mut Vec::new()));
                    match gelesen {
                        Ok(g) => {
                            aus.dunkel |= g.dunkel;
                            aus.offene_imports.extend(g.offen.iter().cloned());
                            aus.blaetter
                                .push(umhuellen(g.rules.clone(), media.as_deref()));
                        }
                        Err(grund) => {
                            aus.fehlende = true;
                            self.vermerke_zu(InputKind::StylesheetUnavailable, &href, &grund);
                        }
                    }
                }
            }
        }
        for (vorspann, grund) in &aus.offene_imports {
            self.vermerke_zu(InputKind::StylesheetUnresolved, vorspann, grund);
        }
        aus
    }
}
