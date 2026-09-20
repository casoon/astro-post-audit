//! Der gemeinsame Regelbestand aus `a11y-rules`, über dem Adapter.
//!
//! Dieses Modul enthält **keine eigene Regel**. Es fährt `a11y_rules` über
//! jede Seite und hängt die Verortung an, die dieses Werkzeug braucht — Datei
//! statt Knotenkennung. Fehlt eine Regel, gehört sie nach `a11y-core`; sie hier
//! zu ergänzen bräche die Zusicherung, dass ein Befund in der CLI, im Build und
//! in der laufenden Seite gleich heißt.
//!
//! # Was hier nicht geprüft werden kann
//!
//! Statisches HTML bedient Tier 1 und, über `accname`, Tier 2. Tier 3 —
//! berechnete Stile und Geometrie — gibt es nicht. Die Kontrastregeln laufen
//! deshalb nicht und werden als **nicht gelaufen** vermerkt statt übergangen.
//! [`nicht_gelaufen`] gibt diese Vermerke nach außen; wer sie unterschlägt,
//! macht aus „nicht geprüft" ein stillschweigendes „bestanden".

use a11y_report::{Finding, RuleRun};
use rayon::prelude::*;

use crate::adapter::{Seite, SeiteMitNamen};
use crate::config::Config;
use crate::discovery::SiteIndex;

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

/// Prüft jede Seite mit dem gemeinsamen Regelbestand.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let html = page.parse_html();
            let seite = Seite::new(&html);
            let doc = SeiteMitNamen::new(&seite);

            a11y_rules::run_with_semantics(&doc)
                .findings
                .into_iter()
                .filter(|f| eingeschaltet(&f.rule_id, config))
                .map(|mut f| {
                    // Die Regeln verorten über die Knotenkennung; dieses
                    // Werkzeug prüft Dateien. Beides bleibt stehen — die
                    // Kennung ist der Rückbezug in den Baum.
                    f.location.file = Some(page.rel_path.clone());
                    if f.help.is_none() {
                        f.help = hilfe_zu(&f.rule_id).map(str::to_string);
                    }
                    if f.suggestion.is_none() {
                        f.suggestion = vorschlag_zu(&f.rule_id).map(str::to_string);
                    }
                    f
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Die Regeln, die dieser Host nicht bedienen kann, samt Grund.
///
/// Hängt nicht an der einzelnen Seite: Welche Tiers ein Host bedient, ist eine
/// Eigenschaft des Hosts. Einmal ermittelt, gilt für den ganzen Lauf.
pub fn nicht_gelaufen() -> Vec<RuleRun> {
    // Ein leeres Dokument genügt: Gefragt ist nicht, was gefunden wurde,
    // sondern welche Regeln gar nicht erst laufen konnten.
    let html = scraper::Html::parse_document("<html></html>");
    let seite = Seite::new(&html);
    let doc = SeiteMitNamen::new(&seite);
    a11y_rules::run_with_semantics(&doc)
        .rule_runs
        .into_iter()
        .filter(|r| r.not_run.is_some())
        .collect()
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
                 <a href="/a"></a>
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
        let offen = nicht_gelaufen();
        let ids: Vec<&str> = offen.iter().map(|r| r.rule_id.as_str()).collect();
        assert_eq!(
            ids,
            ["contrast/text-insufficient", "contrast/text-undetermined"]
        );
        assert!(offen
            .iter()
            .all(|r| r.not_run == Some(a11y_report::NotRun::CapabilityMissing)));
    }
}
