//! `a11y.contrast`: berechnete Stile aus den Stylesheets im Build.
//!
//! Die Seiten liegen in `tests/fixtures/styles`. Geprüft wird über das Binary,
//! wie es der Astro-Integration begegnet.

use std::path::Path;

mod common;
use common::run_audit_json;

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/styles")
}

fn run(config: &str) -> serde_json::Value {
    run_audit_json(&fixture(), config).0
}

/// Die Befunde einer Regel-Familie je Datei.
fn contrast<'a>(json: &'a serde_json::Value, file: &str) -> Vec<&'a serde_json::Value> {
    json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| {
            f["rule_id"].as_str().unwrap().starts_with("contrast/") && f["location"]["file"] == file
        })
        .collect()
}

fn ids(findings: &[&serde_json::Value]) -> Vec<String> {
    findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap().to_string())
        .collect()
}

const AN: &str = r#"{"a11y":{"contrast":true}}"#;

#[test]
fn off_by_default_contrast_stays_not_run() {
    let json = run("{}");
    let alle: Vec<_> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule_id"].as_str().unwrap().starts_with("contrast/"))
        .collect();
    assert!(alle.is_empty(), "{alle:?}");
    let runs = json["rule_runs"].as_array().unwrap();
    assert!(runs
        .iter()
        .any(|r| r["rule_id"] == "contrast/text-insufficient"
            && r["not_run"] == "capability_missing"));
    assert!(json.get("inputs").is_none());
}

#[test]
fn linked_stylesheet_gives_failing_and_passing_text() {
    let json = run(AN);
    let f = contrast(&json, "index.html");
    // Genau ein Text ist zu hell: `.low`. `.good` besteht und meldet nichts.
    assert_eq!(ids(&f), ["contrast/text-insufficient"], "{f:?}");
    assert_eq!(f[0]["outcome"], "fail");
    assert!(f[0]["message"].as_str().unwrap().contains("1.9"), "{f:?}");
}

#[test]
fn root_relative_link_resolves_against_dist() {
    let json = run(AN);
    let f = contrast(&json, "nested/root-relative.html");
    assert_eq!(ids(&f), ["contrast/text-insufficient"], "{f:?}");
}

#[test]
fn query_and_fragment_do_not_hide_the_file() {
    // clean.html links `/assets/site.css?v=3#top`; the readable text passes.
    let json = run(AN);
    assert!(contrast(&json, "clean.html").is_empty());
}

#[test]
fn a_link_leaving_dist_is_not_read() {
    let json = run(AN);
    let f = contrast(&json, "escape.html");
    assert_eq!(ids(&f), ["contrast/text-undetermined"], "{f:?}");
    let inputs = json["inputs"].as_array().unwrap();
    assert!(inputs.iter().any(|n| n["subject"] == "../outside.css"
        && n["detail"]
            .as_str()
            .unwrap()
            .contains("outside the dist directory")));
}

#[test]
fn inline_style_element_and_attribute_count() {
    let json = run(AN);
    let f = contrast(&json, "inline.html");
    assert_eq!(
        ids(&f),
        ["contrast/text-insufficient", "contrast/text-insufficient"],
        "{f:?}"
    );
}

#[test]
fn local_import_is_followed() {
    let json = run(AN);
    let f = contrast(&json, "imported.html");
    assert_eq!(ids(&f), ["contrast/text-insufficient"], "{f:?}");
    let inputs = json["inputs"].as_array().unwrap();
    assert!(
        !inputs
            .iter()
            .any(|n| n["subject"].as_str().unwrap_or("").contains("imported.css")),
        "{inputs:?}"
    );
}

#[test]
fn dark_scheme_is_checked_when_a_sheet_asks_for_it() {
    let json = run(AN);
    let f = contrast(&json, "dark.html");
    // Heading and paragraph are #444444 on #111111 in the dark scheme; both
    // pass in the light one, so only the dark pass reports them.
    assert_eq!(ids(&f), ["contrast/text-insufficient"; 2], "{f:?}");
    assert!(f
        .iter()
        .all(|f| f["message"].as_str().unwrap().contains("dark color scheme")));
    // site.css hat ein dunkles Schema, das besteht: kein dunkler Zusatzbefund.
    let clean = contrast(&json, "clean.html");
    assert!(clean.is_empty(), "{clean:?}");
}

#[test]
fn missing_and_remote_stylesheets_keep_contrast_untested_with_a_note() {
    let json = run(AN);
    let f = contrast(&json, "missing.html");
    assert_eq!(ids(&f), ["contrast/text-undetermined"], "{f:?}");
    assert_eq!(f[0]["outcome"], "untested");

    let inputs = json["inputs"].as_array().expect("inputs note");
    let note = |subject: &str| {
        inputs
            .iter()
            .find(|n| n["subject"] == subject)
            .unwrap_or_else(|| panic!("no note for {subject}: {inputs:?}"))
    };
    let missing = note("/assets/not-built.css");
    assert_eq!(missing["kind"], "stylesheet_unavailable");
    assert!(missing["detail"].as_str().unwrap().contains("not found"));
    let remote = note("https://cdn.example.com/remote.css");
    assert_eq!(remote["kind"], "stylesheet_unavailable");
    assert!(remote["detail"].as_str().unwrap().contains("not on disk"));
}

#[test]
fn remote_import_is_unresolved_and_never_a_silent_pass() {
    let json = run(AN);
    let f = contrast(&json, "remote-import.html");
    assert_eq!(ids(&f), ["contrast/text-undetermined"], "{f:?}");
    let inputs = json["inputs"].as_array().unwrap();
    assert!(inputs.iter().any(|n| n["kind"] == "stylesheet_unresolved"
        && n["subject"].as_str().unwrap().contains("fonts.example.com")));
}

#[test]
fn rules_that_need_geometry_are_listed_as_not_run_not_as_findings() {
    let json = run(AN);
    let runs = json["rule_runs"].as_array().unwrap();
    let ids: Vec<_> = runs
        .iter()
        .map(|r| r["rule_id"].as_str().unwrap())
        .collect();
    assert!(!ids.contains(&"contrast/text-insufficient"), "{ids:?}");
    assert!(ids.contains(&"focus/obscured"), "{ids:?}");
    // Kein Befund `focus/indicator-unmeasured`: Er trüge Schwere `high` und
    // ließe jeden Build scheitern.
    assert!(!json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["outcome"] == "untested" && f["rule_id"] == "focus/indicator-unmeasured"));
}

#[test]
fn shared_stylesheet_notes_are_reported_once_with_a_page_count() {
    let json = run(AN);
    let inputs = json["inputs"].as_array().unwrap();
    let n = inputs
        .iter()
        .filter(|n| n["subject"] == "/assets/not-built.css")
        .count();
    assert_eq!(n, 1);
}
