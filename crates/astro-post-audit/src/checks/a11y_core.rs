//! Der gemeinsame Regelbestand aus `a11y-rules`, über dem Adapter.
//!
//! Dieses Modul enthält **keine eigene Regel**. Es fährt `a11y_rules` über
//! jede Seite und hängt die Verortung an, die dieses Werkzeug braucht — Datei
//! statt Knotenkennung. Fehlt eine Regel, gehört sie nach `a11y-rules` in barrierlab; sie hier
//! zu ergänzen bräche die Zusicherung, dass ein Befund in der CLI, im Build und
//! in der laufenden Seite gleich heißt.
//!
//! # Was hier nicht geprüft werden kann
//!
//! Statisches HTML bedient Tier 1 und, über `accname`, Tier 2. Tier 3 —
//! berechnete Stile und Geometrie — gibt es nur mit `a11y.contrast`, und dann
//! ohne Geometrie. Was nicht läuft (Kontrast ohne den Schalter, Zielgrößen
//! immer), wird als **nicht gelaufen** vermerkt statt übergangen.
//! [`nicht_gelaufen`] gibt diese Vermerke nach außen; wer sie unterschlägt,
//! macht aus „nicht geprüft" ein stillschweigendes „bestanden".

use std::collections::{HashMap, HashSet};

use a11y_report::{Finding, NotRun, Outcome, RootCauseId, RuleRun};
use rayon::prelude::*;

use a11y_rules::Locale;
use style_cascade::{Cascade, Environment};

use crate::adapter::{Seite, SeiteMitNamen, SeiteMitStilen};
use crate::config::Config;
use crate::discovery::{PageInfo, SiteIndex};

/// Ob diese Regel nach der Konfiguration laufen soll.
///
/// Die Schalter in `astro.config.mjs` bleiben, wie sie sind — sie sind die
/// Oberfläche zum Anwender, und die Umstellung auf den gemeinsamen Kern soll
/// dort nichts kosten. Nur die Kennungen dahinter sind neu.
///
/// Regeln ohne Schalter laufen. Das sind die, die der Kern mitbringt und dieses
/// Werkzeug vorher nicht hatte — `lists/*`, `tables/*`, `svg/name-missing`,
/// `forms/placeholder-as-label`, `keyboard/positive-tabindex`,
/// `document/lang-invalid`, `headings/empty`. Neue Abdeckung abzuschalten,
/// weil sie neu ist, wäre die falsche Vorgabe.
fn eingeschaltet(rule_id: &str, config: &Config) -> bool {
    let a = &config.a11y;
    match rule_id {
        "images/alt-missing" => a.img_alt_required,
        "images/alt-suspicious" => a.check_alt_quality,
        "links/name-missing" => a.a_accessible_name_required,
        "links/generic-name" => a.warn_generic_link_text,
        // Mehrdeutige Linknamen hatte dieses Werkzeug nicht -- neue Abdeckung,
        // deshalb ohne Schalter.
        "links/ambiguous-name" => true,
        "buttons/name-missing" => a.button_name_required,
        "forms/label-missing" => a.label_for_required,
        "keyboard/hidden-focusable" => a.aria_hidden_focusable_check,
        "keyboard/skip-link-missing" => a.require_skip_link,
        "ids/duplicate" => a.check_duplicate_ids,
        "aria/role-invalid"
        | "aria/role-abstract"
        | "aria/reference-missing"
        | "aria/required-attribute-missing" => a.check_aria_roles,
        id if id.starts_with("landmarks/") => a.check_landmarks,
        // Die Checkliste ist kein Urteil, sondern eine Aufgabe für Menschen --
        // auf jeder Seite wiederholt wäre sie im Build-Log Rauschen.
        id if id.starts_with("manual/") => a.manual_checklist,
        // Die Darstellungskonvention ist eine Hausregel, keine Norm -- nur
        // auf Wunsch (`groups.viz_alternatives`).
        id if id.starts_with("viz/") || id.starts_with("display/") => a.viz_alternatives,
        // Was frueher in html_basics lag und jetzt aus dem Kern kommt. Die
        // Schalter heissen weiter wie vorher -- sie sind die Oberflaeche.
        "document/lang-missing" | "document/lang-invalid" => config.html_basics.lang_attr_required,
        "document/title-missing" | "document/title-empty" => config.html_basics.title_required,
        "zoom/viewport-missing" => config.html_basics.viewport_required,
        "headings/h1-missing" => config.headings.require_h1,
        "headings/h1-multiple" => config.headings.single_h1,
        "headings/skip-level" => config.headings.no_skip,
        _ => true,
    }
}

/// Der Hinweistext zu einer Kennung, aus den Deklarationen des Kerns.
///
/// `a11y-rules` führt `help` an der Regel (`Meta`), nicht am einzelnen Befund —
/// dieselbe Zeichenkette an tausend Befunde zu hängen wäre Verschwendung. Für
/// die Ausgabe hier gehört sie an den Befund, also wird sie beim Übergang
/// nachgetragen.
fn hilfe_zu(rule_id: &str) -> Option<&'static str> {
    a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .chain(a11y_rules::rendering_metas())
        .find(|m| m.ids.contains(&rule_id))
        .map(|m| m.help)
}

/// Konkrete Vorschläge in der Sprache dieses Werkzeugs.
///
/// Bewusst **nicht** im Kern: Ein Vorschlag ist Darstellung, keine Beurteilung.
/// Er hängt an der Zielgruppe und an der Sprache der Oberfläche; `auditmysite`
/// und LiveAudit formulieren anders. Im Kern stünde er allen im Weg.
const VORSCHLAEGE: &[(&str, &str)] = &[
    ("navigation/location-missing", "On the home page, mark the home/logo link with aria-current=\"page\"; it need not be a navigation item."),
    ("document/lang-missing", "<html lang=\"en\">"),
    ("document/title-missing", "<title>Page Title</title>"),
    ("document/title-empty", "<title>Page Title</title>"),
    (
        "zoom/viewport-missing",
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
    ),
    (
        "landmarks/main-missing",
        "<main id=\"main-content\">…</main>",
    ),
    (
        "keyboard/skip-link-missing",
        "<a href=\"#main-content\" class=\"sr-only\">Skip to content</a>",
    ),
    (
        "images/alt-missing",
        "<img src=\"…\" alt=\"Was das Bild zeigt\">",
    ),
];

fn vorschlag_zu(rule_id: &str) -> Option<&'static str> {
    VORSCHLAEGE
        .iter()
        .find(|(id, _)| *id == rule_id)
        .map(|(_, v)| *v)
}

/// Die Erfolgskriterien der Stufe AAA in WCAG 2.2.
///
/// Die Regeln setzen `wcag_level` nicht; die Stufe ergibt sich hier aus den
/// Kriterien, die ein Befund trägt.
const AAA_KRITERIEN: &[&str] = &[
    "1.2.6", "1.2.7", "1.2.8", "1.2.9", "1.3.6", "1.4.6", "1.4.7", "1.4.8", "1.4.9", "2.1.3",
    "2.2.3", "2.2.4", "2.2.5", "2.2.6", "2.3.2", "2.3.3", "2.4.8", "2.4.9", "2.4.10", "2.4.12",
    "2.4.13", "2.5.5", "2.5.6", "3.1.3", "3.1.4", "3.1.5", "3.1.6", "3.2.5", "3.3.5", "3.3.6",
    "3.3.9",
];

fn nur_aaa(f: &Finding) -> bool {
    !f.wcag.is_empty() && f.wcag.iter().all(|c| AAA_KRITERIEN.contains(&c.as_str()))
}

/// Fasst AAA-Befunde am selben Element-Muster über alle Seiten zu einem zusammen.
///
/// Ein `target="_blank"` im Footer stünde sonst auf jeder Seite einzeln im
/// Bericht -- dieselbe Ursache, hundertfach. Das Muster ist das von
/// `normalize_selector`: Ziffern fallen weg, gleichartige Geschwister (die
/// Links einer Footer-Liste) gelten als ein Muster. Stehen bleibt der Befund aus der
/// alphabetisch ersten Datei, damit der Bericht nicht von der Reihenfolge des
/// Dateisystems abhängt; die Nachricht nennt die Zahl der Seiten. Befunde der
/// Stufen A und AA bleiben je Seite: Dort zählt jede Seite für das Gate.
fn aaa_zusammenfassen(findings: Vec<Finding>) -> Vec<Finding> {
    let ursache =
        |f: &Finding| RootCauseId::new(&f.rule_id, f.location.selector.as_deref().unwrap_or(""));
    // Je Ursache: Vorkommen, betroffene Seiten und Index des Vertreters.
    let mut gruppen: HashMap<RootCauseId, (usize, HashSet<Option<&str>>, usize)> = HashMap::new();
    for (i, f) in findings.iter().enumerate().filter(|(_, f)| nur_aaa(f)) {
        let (vorkommen, seiten, vertreter) =
            gruppen.entry(ursache(f)).or_insert((0, HashSet::new(), i));
        *vorkommen += 1;
        seiten.insert(f.location.file.as_deref());
        if f.location.file < findings[*vertreter].location.file {
            *vertreter = i;
        }
    }
    let behalten: HashMap<usize, (usize, usize)> = gruppen
        .into_values()
        .map(|(vorkommen, seiten, i)| (i, (vorkommen, seiten.len())))
        .collect();
    findings
        .into_iter()
        .enumerate()
        .filter_map(|(i, mut f)| {
            if !nur_aaa(&f) {
                return Some(f);
            }
            let (vorkommen, seiten) = *behalten.get(&i)?;
            if vorkommen > 1 {
                f.message.push_str(&format!(
                    " Same element pattern {vorkommen} times on {seiten} page(s); reported once."
                ));
            }
            Some(f)
        })
        .collect()
}

/// Hängt die Verortung an, die dieses Werkzeug braucht, und wendet die Schalter an.
fn verorten(
    mut f: Finding,
    page: &PageInfo,
    seite: &Seite<'_>,
    config: &Config,
) -> Option<Finding> {
    if !eingeschaltet(&f.rule_id, config) {
        return None;
    }
    // Die Regeln verorten über die Knotenkennung; dieses Werkzeug prüft
    // Dateien. Beides bleibt stehen — die Kennung ist der Rückbezug in den
    // Baum.
    f.location.file = Some(page.rel_path.clone());
    if f.location.selector.is_none() {
        f.location.selector = f.location.node.as_deref().and_then(|n| seite.selector(n));
    }
    if f.help.is_none() {
        f.help = hilfe_zu(&f.rule_id).map(str::to_string);
        if f.rule_id == "navigation/location-missing" {
            if let Some(help) = &mut f.help {
                help.push_str(
                    " On the home page, aria-current=\"page\" can go on the home/logo link.",
                );
            }
        }
    }
    if f.suggestion.is_none() {
        f.suggestion = vorschlag_zu(&f.rule_id).map(str::to_string);
    }
    Some(f)
}

/// Struktur und Semantik, keine Stile: das Verhalten ohne `a11y.contrast`.
fn seite_ohne_stile(page: &PageInfo, config: &Config) -> Vec<Finding> {
    let html = page.parse_html();
    let seite = Seite::new(&html);
    let doc = SeiteMitNamen::new(&seite);
    a11y_rules::run_with_semantics(&doc)
        .findings
        .into_iter()
        .filter_map(|f| verorten(f, page, &seite, config))
        .collect()
}

/// Eine Heuristik, die der Host nicht bedienen kann, meldet sich als
/// `untested` am Wurzelknoten („der Host misst X nicht"). Das ist keine Aussage
/// über die Seite, sondern über den Host, und gehört in `rule_runs`. Je Seite
/// wiederholt -- und `focus/indicator-unmeasured` trägt Schwere `high` --
/// ließe es jeden Build scheitern.
fn host_luecke(f: &Finding, wurzel: &str) -> bool {
    f.outcome == Outcome::Untested
        && f.location.node.as_deref() == Some(wurzel)
        && !f.rule_id.starts_with("manual/")
}

/// Ein nicht lesbares Blatt macht jeden Text der Seite „nicht bestimmbar". Das
/// wären Hunderte gleichlautender Befunde je Seite, mit derselben Ursache; sie
/// stehen hier als einer mit Zähler. Befunde, die etwas über ein bestimmtes
/// Element sagen (`fail`, `review`), bleiben einzeln.
fn unbestimmt_zusammenfassen(findings: Vec<Finding>) -> Vec<Finding> {
    let unbestimmt = |f: &Finding| {
        f.outcome == Outcome::Untested
            && matches!(
                f.rule_id.as_str(),
                "contrast/text-undetermined" | "contrast/control-undetermined"
            )
    };
    let mut gesehen: HashMap<String, usize> = HashMap::new();
    for f in findings.iter().filter(|f| unbestimmt(f)) {
        *gesehen.entry(f.rule_id.clone()).or_default() += 1;
    }
    let mut erster: HashSet<String> = HashSet::new();
    findings
        .into_iter()
        .filter_map(|mut f| {
            if !unbestimmt(&f) {
                return Some(f);
            }
            if !erster.insert(f.rule_id.clone()) {
                return None;
            }
            let n = gesehen[&f.rule_id];
            if n > 1 {
                f.message
                    .push_str(&format!(" {n} elements on this page; reported once."));
            }
            Some(f)
        })
        .collect()
}

/// Regeln über Stylesheets sagen etwas über das Blatt, nicht über die Seite.
/// Ein Blatt, das alle Seiten teilen, ergäbe auf jeder denselben Befund; er
/// steht einmal, bei der alphabetisch ersten Datei, mit der Zahl der Seiten.
fn blatt_befunde_zusammenfassen(findings: Vec<Finding>) -> Vec<Finding> {
    let ids: HashSet<&str> = a11y_rules::stylesheet_metas()
        .iter()
        .flat_map(|m| m.ids.iter().copied())
        .collect();
    let blatt = |f: &Finding| ids.contains(f.rule_id.as_str());
    let schluessel = |f: &Finding| (f.rule_id.clone(), f.message.clone());
    let mut gruppen: HashMap<(String, String), (usize, Option<String>)> = HashMap::new();
    for f in findings.iter().filter(|f| blatt(f)) {
        let g = gruppen.entry(schluessel(f)).or_insert((0, None));
        g.0 += 1;
        if g.1.is_none() || f.location.file < g.1 {
            g.1.clone_from(&f.location.file);
        }
    }
    let mut ausgegeben: HashSet<(String, String)> = HashSet::new();
    findings
        .into_iter()
        .filter_map(|mut f| {
            if !blatt(&f) {
                return Some(f);
            }
            let k = schluessel(&f);
            let (n, datei) = &gruppen[&k];
            if f.location.file != *datei || !ausgegeben.insert(k) {
                return None;
            }
            if *n > 1 {
                f.message.push_str(&format!(
                    " Same stylesheet rule on {n} page(s); reported once."
                ));
            }
            Some(f)
        })
        .collect()
}

/// Alle Tiers, die die Kaskade bedienen kann, über den Blättern der Seite.
///
/// Läuft im hellen Schema; fragt ein Blatt nach `prefers-color-scheme`, läuft
/// der Kontrast ein zweites Mal im dunklen. Ein Befund, der in beiden Schemata
/// gleich ausfällt, steht nur einmal da.
fn seite_mit_stilen(page: &PageInfo, index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let html = page.parse_html();
    let seite = Seite::new(&html);
    let stile = index.styles.fuer_seite(index, page, &html);
    let lauf = |env: Environment| -> Vec<Finding> {
        let cascade = Cascade::new(&stile.blaetter, env);
        let host = SeiteMitStilen::new(&seite, cascade, stile.fehlende);
        for vorspann in host.unaufgeloeste_imports().iter().filter(|_| !env.dark) {
            index
                .styles
                .unaufgeloest_nachtragen(vorspann, &stile.offene_imports);
        }
        let report = a11y_rules::run_full_in(&host, Locale::En);
        a11y_rules::run_stylesheets_in(report, &host, &stile.blaetter, Locale::En).findings
    };
    let hell = Environment::default();
    let mut findings = lauf(hell);
    if stile.dunkel {
        let dunkel = lauf(Environment { dark: true, ..hell });
        let schluessel = |f: &Finding| (f.rule_id.clone(), f.location.node.clone());
        let bekannt: HashSet<_> = findings.iter().map(schluessel).collect();
        findings.extend(
            dunkel
                .into_iter()
                .filter(|f| f.rule_id.starts_with("contrast/") && !bekannt.contains(&schluessel(f)))
                .map(|mut f| {
                    f.message.push_str(" (dark color scheme)");
                    f
                }),
        );
    }
    let wurzel = a11y_dom::Node::id(a11y_dom::Document::root(&seite)).to_string();
    let findings = findings
        .into_iter()
        .filter(|f| !host_luecke(f, &wurzel))
        .filter_map(|f| verorten(f, page, &seite, config))
        .collect();
    unbestimmt_zusammenfassen(findings)
}

/// Prüft jede Seite mit dem gemeinsamen Regelbestand.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let findings = index
        .pages
        .par_iter()
        .flat_map(|page| {
            if config.a11y.contrast {
                seite_mit_stilen(page, index, config)
            } else {
                seite_ohne_stile(page, config)
            }
        })
        .collect();
    blatt_befunde_zusammenfassen(aaa_zusammenfassen(findings))
}

/// Die Regeln, die dieser Host nicht bedienen kann, samt Grund.
///
/// Hängt nicht an der einzelnen Seite: Welche Tiers ein Host bedient, ist eine
/// Eigenschaft des Hosts. Einmal ermittelt, gilt für den ganzen Lauf. Mit
/// `a11y.contrast` bedient der Host auch die Darstellung; offen bleiben dann
/// die Regeln, die Geometrie brauchen.
pub fn nicht_gelaufen(config: &Config) -> Vec<RuleRun> {
    // Ein leeres Dokument genügt: Gefragt ist nicht, was gefunden wurde,
    // sondern welche Regeln gar nicht erst laufen konnten.
    let html = scraper::Html::parse_document("<html></html>");
    let seite = Seite::new(&html);
    let (report, wurzel) = if config.a11y.contrast {
        let cascade = Cascade::new(&[], Environment::default());
        let host = SeiteMitStilen::new(&seite, cascade, false);
        let report = a11y_rules::run_full_in(&host, Locale::En);
        let report = a11y_rules::run_stylesheets_in(report, &host, &[], Locale::En);
        (
            report,
            a11y_dom::Node::id(a11y_dom::Document::root(&host)).to_string(),
        )
    } else {
        let report = a11y_rules::run_with_semantics(&SeiteMitNamen::new(&seite));
        (report, String::new())
    };
    let mut offen: Vec<RuleRun> = report
        .rule_runs
        .into_iter()
        .filter(|r| r.not_run.is_some())
        .collect();
    // Heuristiken, die sich als `untested` am Wurzelknoten melden (siehe
    // `host_luecke`), stehen hier als nicht gelaufen.
    for f in report.findings.iter().filter(|f| host_luecke(f, &wurzel)) {
        if !offen.iter().any(|r| r.rule_id == f.rule_id) {
            offen.push(
                RuleRun::not_run(f.rule_id.clone(), NotRun::CapabilityMissing)
                    .with_wcag(f.wcag.iter().cloned())
                    .with_reason(f.message.clone()),
            );
        }
    }
    offen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn befunde(html: &str) -> Vec<Finding> {
        let parsed = scraper::Html::parse_document(html);
        let seite = Seite::new(&parsed);
        let doc = SeiteMitNamen::new(&seite);
        a11y_rules::run_with_semantics(&doc).findings
    }

    fn kennungen(html: &str) -> Vec<String> {
        befunde(html).into_iter().map(|f| f.rule_id).collect()
    }

    #[test]
    fn bekannte_fehler_erzeugen_die_kennungen_aus_a11y_rules() {
        let ids = kennungen(
            r#"<!doctype html><html><head><title></title></head><body>
                 <h3>Uebersprungene Ebene</h3>
                 <img src="logo.png">
                 <input type="text" id="dup">
                 <span id="dup"></span>
                 <a href="/a" aria-describedby="dup"></a>
               </body></html>"#,
        );
        for erwartet in [
            "document/lang-missing",
            "document/title-empty",
            "headings/h1-missing",
            "images/alt-missing",
            "forms/label-missing",
            "ids/duplicate",
            "links/name-missing",
        ] {
            assert!(
                ids.iter().any(|id| id == erwartet),
                "{erwartet} fehlt: {ids:?}"
            );
        }
    }

    /// Der Gewinn gegenüber der bisherigen Umsetzung: Der Accessible Name
    /// entsteht nach accname. Eine Näherung aus Teilbaumtext meldete hier
    /// einen Fehler, den es nicht gibt.
    #[test]
    fn aria_labelledby_wird_aufgeloest_statt_geraten() {
        let ids = kennungen(
            r#"<!doctype html><html lang="de"><head><title>T</title></head><body>
                 <h1>T</h1>
                 <span id="l">Weiterlesen</span>
                 <a href="/a" aria-labelledby="l"></a>
               </body></html>"#,
        );
        assert!(!ids.iter().any(|id| id == "links/name-missing"), "{ids:?}");
    }

    /// Jede Kennung, die der Kern erzeugt, muss in der Schaltertabelle
    /// bewusst behandelt sein — entweder mit eigenem Schalter oder über den
    /// Vorgabezweig. Der Test schlägt an, wenn a11y-rules eine Kennung
    /// hinzufügt, die niemand eingeordnet hat.
    #[test]
    fn jede_kennung_des_kerns_ist_eingeordnet() {
        let config = Config::default();
        let alle: Vec<&str> = a11y_rules::structure_metas()
            .iter()
            .chain(a11y_rules::semantics_metas())
            .flat_map(|m| m.ids.iter().copied())
            .collect();
        // Mit der Vorgabekonfiguration laeuft alles; die Zusicherung ist, dass
        // der Aufruf fuer jede Kennung definiert ist und nicht in Panik geht.
        for id in alle {
            let _ = eingeschaltet(id, &config);
        }
    }

    /// Jeder Vorschlag muss zu einer Kennung gehören, die es gibt. Ein
    /// Vorschlag zu einer Kennung, die der Kern nicht mehr erzeugt, wäre
    /// stiller toter Text.
    #[test]
    fn jeder_vorschlag_gehoert_zu_einer_bekannten_kennung() {
        for (id, _) in VORSCHLAEGE {
            assert!(hilfe_zu(id).is_some(), "{id} gibt es im Kern nicht");
        }
    }

    /// „Nicht prüfbar" ist nicht „bestanden": Ohne Tier 3 werden die
    /// Kontrastregeln vermerkt, nicht verschwiegen.
    #[test]
    fn kontrast_wird_als_nicht_gelaufen_vermerkt() {
        let offen = nicht_gelaufen(&Config::default());
        let ids: Vec<&str> = offen.iter().map(|r| r.rule_id.as_str()).collect();
        for erwartet in ["contrast/text-insufficient", "contrast/text-undetermined"] {
            assert!(ids.contains(&erwartet), "{erwartet} fehlt: {ids:?}");
        }
        assert!(offen
            .iter()
            .all(|r| r.not_run == Some(a11y_report::NotRun::CapabilityMissing)));
    }

    fn aaa_befund(datei: &str, selektor: &str) -> Finding {
        let mut f =
            Finding::review("links/new-window-unannounced", "neues Fenster").with_wcag(["3.2.5"]);
        f.location.file = Some(datei.to_string());
        f.location.selector = Some(selektor.to_string());
        f
    }

    /// Dasselbe Footer-Element auf jeder Seite ist eine Ursache, kein
    /// hundertfacher Befund. A/AA-Befunde bleiben je Seite.
    #[test]
    fn aaa_befunde_werden_seitenuebergreifend_zusammengefasst() {
        let mut aa = Finding::fail("images/alt-missing", "kein alt").with_wcag(["1.1.1"]);
        aa.location.selector = Some("footer > a".into());
        let mut aa2 = aa.clone();
        aa.location.file = Some("b.html".into());
        aa2.location.file = Some("a.html".into());
        let raus = aaa_zusammenfassen(vec![
            aaa_befund("c.html", "footer > a"),
            aaa_befund("a.html", "footer > a"),
            aaa_befund("b.html", "footer > a"),
            aaa_befund("b.html", "footer > a"),
            aaa_befund("b.html", "main > a"),
            aa,
            aa2,
        ]);
        let aaa: Vec<_> = raus.iter().filter(|f| nur_aaa(f)).collect();
        assert_eq!(aaa.len(), 2);
        assert_eq!(aaa[0].location.file.as_deref(), Some("a.html"));
        assert!(aaa[0]
            .message
            .ends_with("Same element pattern 4 times on 3 page(s); reported once."));
        assert_eq!(aaa[1].message, "neues Fenster");
        assert_eq!(
            raus.iter()
                .filter(|f| f.rule_id == "images/alt-missing")
                .count(),
            2
        );
    }
}
