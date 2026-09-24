//! Die Übersetzung der alten Regelkennungen auf die aus `a11y-rules`.
//!
//! Die Kennungen dieses Werkzeugs und die des gemeinsamen Regelkerns sind
//! historisch verschieden gewachsen: `a11y/img-alt` hier, `images/alt-missing`
//! dort. Der Kern setzt sich durch — sonst hieße ein Befund in der CLI anders
//! als im Build und anders als in der laufenden Seite, und genau das soll das
//! geteilte Modell verhindern.
//!
//! Der Preis ist ein Bruch für alle, die eine Baseline-Datei committet oder
//! `severity`-Overrides gesetzt haben. Diese Tabelle federt ihn ab: Beim
//! Einlesen wird jede Altkennung übersetzt und einmal je Kennung gewarnt.
//! Geschrieben wird ausschließlich neu — die Tabelle ist ein Migrationspfad,
//! kein Dauerzustand.
//!
//! **Nicht übersetzt** werden `html/meta-description-missing`,
//! `html/meta-description-too-long` und `html/title-too-long`. Das sind
//! SEO-Regeln, keine Barrierefreiheit; sie gehören nicht in `a11y-rules` und
//! behalten ihre Kennung.

use std::collections::HashSet;
use std::sync::Mutex;

/// Alte Kennung → Kennung in `a11y-rules`.
///
/// `a11y/duplicate-id` und `a11y/duplicate-id-aria` laufen beide auf
/// `ids/duplicate` zu: Der Kern unterscheidet nicht, ob die doppelte ID von
/// einem ARIA-Attribut referenziert wird. Für die Übersetzung einer Baseline
/// ist das unschädlich — sie wird dadurch großzügiger, nicht strenger.
pub const ALTE_KENNUNGEN: &[(&str, &str)] = &[
    ("a11y/aria-hidden-focusable", "keyboard/hidden-focusable"),
    ("a11y/aria-required-attr", "aria/required-attribute-missing"),
    ("a11y/aria-role-abstract", "aria/role-abstract"),
    ("a11y/aria-role-invalid", "aria/role-invalid"),
    ("a11y/button-name", "buttons/name-missing"),
    ("a11y/duplicate-id", "ids/duplicate"),
    ("a11y/duplicate-id-aria", "ids/duplicate"),
    ("a11y/form-label", "forms/label-missing"),
    // Nicht auf links/ambiguous-name: Das ist eine andere Regel ("zwei Links
    // heissen gleich, fuehren aber woandershin"). Der nichtssagende Linktext
    // heisst im Kern links/generic-name.
    ("a11y/generic-link-text", "links/generic-name"),
    ("a11y/img-alt", "images/alt-missing"),
    ("a11y/invalid-img-alt", "images/alt-suspicious"),
    (
        "a11y/landmark-footer-missing",
        "landmarks/contentinfo-missing",
    ),
    ("a11y/landmark-header-missing", "landmarks/banner-missing"),
    ("a11y/landmark-main-duplicate", "landmarks/main-duplicate"),
    ("a11y/landmark-main-missing", "landmarks/main-missing"),
    ("a11y/landmark-nav-missing", "landmarks/navigation-missing"),
    ("a11y/link-name", "links/name-missing"),
    ("a11y/skip-link", "keyboard/skip-link-missing"),
    ("headings/multiple-h1", "headings/h1-multiple"),
    ("headings/no-h1", "headings/h1-missing"),
    ("html/lang-missing", "document/lang-missing"),
    ("html/title-empty", "document/title-empty"),
    ("html/title-missing", "document/title-missing"),
    ("html/viewport-missing", "zoom/viewport-missing"),
];

/// Kennungen, über die schon gewarnt wurde. Eine Baseline mit 400 Einträgen
/// derselben Regel soll eine Zeile erzeugen, nicht 400.
static GEWARNT: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Übersetzt eine Kennung, falls sie eine Altkennung ist.
///
/// Unbekannte Kennungen bleiben unverändert — eine Baseline darf Einträge
/// enthalten, die dieses Werkzeug gar nicht kennt, etwa aus einer neueren
/// Version.
pub fn uebersetzen(kennung: &str) -> &str {
    ALTE_KENNUNGEN
        .iter()
        .find(|(alt, _)| *alt == kennung)
        .map_or(kennung, |(_, neu)| *neu)
}

/// Die Kennungen, unter denen ein Eintrag gelten soll: die angegebene und,
/// falls es eine Altkennung ist, zusätzlich die neue.
///
/// Beide, nicht nur die neue. Solange die Regeln dieses Werkzeugs noch die
/// alten Kennungen erzeugen, liefe eine nur übersetzte Baseline ins Leere; nach
/// dem Regeltausch wäre es umgekehrt. Ein Eintrag soll den Befund unterdrücken,
/// gleich wie er gerade heißt.
pub fn beide_kennungen(kennung: &str, herkunft: &str) -> Vec<String> {
    let neu = uebersetzen_mit_hinweis(kennung, herkunft);
    if neu == kennung {
        vec![neu]
    } else {
        vec![kennung.to_string(), neu]
    }
}

/// Wie [`uebersetzen`], warnt aber einmal je übersetzter Kennung.
pub fn uebersetzen_mit_hinweis(kennung: &str, herkunft: &str) -> String {
    let neu = uebersetzen(kennung);
    if neu != kennung {
        let mut gesehen = GEWARNT.lock().unwrap_or_else(|e| e.into_inner());
        let gesehen = gesehen.get_or_insert_with(HashSet::new);
        if gesehen.insert(kennung.to_string()) {
            eprintln!(
                "warning: {herkunft} uses the old rule id '{kennung}'. \
                 It now reads '{neu}'. The old id keeps working for now; \
                 update it to silence this warning."
            );
        }
    }
    neu.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn alle_altkennungen_sind_verschieden() {
        let alt: HashSet<&str> = ALTE_KENNUNGEN.iter().map(|(a, _)| *a).collect();
        assert_eq!(alt.len(), ALTE_KENNUNGEN.len(), "doppelte Altkennung");
    }

    #[test]
    fn keine_altkennung_zeigt_auf_sich_selbst() {
        for (alt, neu) in ALTE_KENNUNGEN {
            assert_ne!(alt, neu, "{alt} braucht keinen Eintrag");
        }
    }

    /// Die Tabelle ist einstufig: Keine Zielkennung darf selbst wieder
    /// übersetzt werden, sonst hinge das Ergebnis an der Reihenfolge.
    #[test]
    fn die_tabelle_ist_einstufig() {
        for (_, neu) in ALTE_KENNUNGEN {
            assert_eq!(uebersetzen(neu), *neu, "{neu} wird weiter uebersetzt");
        }
    }

    #[test]
    fn unbekannte_kennungen_bleiben_unveraendert() {
        assert_eq!(uebersetzen("links/broken"), "links/broken");
        assert_eq!(
            uebersetzen("html/meta-description-missing"),
            "html/meta-description-missing"
        );
    }

    #[test]
    fn beide_kennungen_gelten_waehrend_der_umstellung() {
        assert_eq!(
            beide_kennungen("a11y/img-alt", "test"),
            vec!["a11y/img-alt", "images/alt-missing"]
        );
        // Eine Kennung ohne Alteintrag bleibt einfach sie selbst.
        assert_eq!(
            beide_kennungen("links/broken", "test"),
            vec!["links/broken"]
        );
    }

    #[test]
    fn bekannte_kennungen_werden_uebersetzt() {
        assert_eq!(uebersetzen("a11y/img-alt"), "images/alt-missing");
        assert_eq!(uebersetzen("headings/no-h1"), "headings/h1-missing");
        assert_eq!(
            uebersetzen("html/viewport-missing"),
            "zoom/viewport-missing"
        );
    }
}
