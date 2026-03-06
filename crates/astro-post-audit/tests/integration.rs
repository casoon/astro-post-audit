//! Integration tests for all check modules.
//!
//! Strategy: use tempfile to create dist directories with specific HTML content,
//! build a SiteIndex, run checks, and assert on the findings.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

mod common;
use common::{run_audit, run_audit_json, write_valid_page};

// ==========================================================================
// Good fixtures: zero findings under default config
// ==========================================================================

#[test]
fn good_fixtures_pass_clean() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/good");
    let (json, code) = run_audit_json(
        &fixture_path,
        r#"{"site":{"base_url":"https://example.com"}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    // Good fixtures should produce no errors under default config.
    // They may produce canonical/target-missing warnings since canonicals point
    // to https://example.com/... but the fixture dist doesn't include all routes.
    let errors: Vec<_> = findings.iter().filter(|f| f["level"] == "error").collect();
    assert!(
        errors.is_empty(),
        "Expected no errors on good fixtures, got: {:?}",
        errors
    );
    assert_eq!(code, 0);
}

// ==========================================================================
// Bad fixtures: should detect many errors
// ==========================================================================

#[test]
fn bad_fixtures_detect_errors() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/bad");
    let (json, code) = run_audit_json(&fixture_path, "{}");
    let findings = json["findings"].as_array().unwrap();
    let rule_ids: Vec<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();

    // Should detect these errors on the bad fixture
    assert!(
        rule_ids.contains(&"html/lang-missing"),
        "Missing lang detection"
    );
    assert!(
        rule_ids.contains(&"html/title-empty"),
        "Empty title detection"
    );
    assert!(
        rule_ids.contains(&"html/viewport-missing"),
        "Missing viewport"
    );
    assert!(rule_ids.contains(&"canonical/missing"), "Missing canonical");
    assert!(rule_ids.contains(&"a11y/img-alt"), "Missing img alt");
    assert!(rule_ids.contains(&"a11y/link-name"), "Empty link name");
    assert!(rule_ids.contains(&"a11y/button-name"), "Empty button name");
    assert!(rule_ids.contains(&"a11y/form-label"), "Missing form label");
    assert!(rule_ids.contains(&"headings/no-h1"), "Missing h1");

    assert_eq!(code, 1, "Should exit with code 1 on errors");
}

// ==========================================================================
// SEO / Canonical checks
// ==========================================================================

#[test]
fn seo_canonical_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), "{}");
    let findings = json["findings"].as_array().unwrap();
    let rule_ids: Vec<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(rule_ids.contains(&"canonical/missing"));
    assert_eq!(code, 1);
}

#[test]
fn seo_canonical_multiple() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><link rel="canonical" href="https://example.com/other/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let rule_ids: Vec<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(rule_ids.contains(&"canonical/multiple"));
    assert_eq!(code, 1);
}

#[test]
fn seo_canonical_empty() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href=""></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), "{}");
    let findings = json["findings"].as_array().unwrap();
    let rule_ids: Vec<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(rule_ids.contains(&"canonical/empty"));
    assert_eq!(code, 1);
}

#[test]
fn seo_canonical_not_absolute() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="/about/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), "{}");
    let findings = json["findings"].as_array().unwrap();
    let rule_ids: Vec<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(rule_ids.contains(&"canonical/not-absolute"));
    assert_eq!(code, 1);
}

#[test]
fn seo_canonical_cross_origin() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://other-site.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let rule_ids: Vec<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(rule_ids.contains(&"canonical/cross-origin"));
    assert_eq!(code, 1);
}

#[test]
fn seo_noindex_detection() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><meta name="robots" content="noindex"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    // Default: allow_noindex=true, fail_if_noindex=false -> no finding
    let (json, _code) =
        run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let has_noindex = findings.iter().any(|f| f["rule_id"] == "robots/noindex");
    assert!(!has_noindex, "Default config should allow noindex");
}

// ==========================================================================
// HTML Basics checks
// ==========================================================================

#[test]
fn html_basics_lang_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule_id"] == "html/lang-missing"));
}

#[test]
fn html_basics_title_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "html/title-missing"));
}

#[test]
fn html_basics_title_too_long() {
    let dir = TempDir::new().unwrap();
    let long_title = "A".repeat(80);
    fs::write(
        dir.path().join("index.html"),
        format!(
            r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{}</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
            long_title
        ),
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "html/title-too-long"));
    assert_eq!(code, 0, "title-too-long is a warning, not error");
}

#[test]
fn html_basics_viewport_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "html/viewport-missing"));
}

// ==========================================================================
// Heading checks
// ==========================================================================

#[test]
fn headings_no_h1() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h2>Not an H1</h2></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule_id"] == "headings/no-h1"));
    assert_eq!(code, 1);
}

#[test]
fn headings_multiple_h1() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>First</h1><h1>Second</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "headings/multiple-h1"));
    assert_eq!(code, 0, "multiple-h1 is a warning");
}

// ==========================================================================
// A11y checks
// ==========================================================================

#[test]
fn a11y_img_alt_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><img src="/photo.jpg"></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule_id"] == "a11y/img-alt"));
    assert_eq!(code, 1);
}

#[test]
fn a11y_decorative_image_no_error() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><img src="/spacer.gif" role="presentation"><img src="/bg.jpg" aria-hidden="true"></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let img_alt_findings: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "a11y/img-alt")
        .collect();
    assert!(
        img_alt_findings.is_empty(),
        "Decorative images should not trigger img-alt error"
    );
}

#[test]
fn a11y_link_name_empty() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><a href="/page/"></a></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule_id"] == "a11y/link-name"));
    assert_eq!(code, 1);
}

#[test]
fn a11y_link_with_aria_label_ok() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><a href="/settings/" aria-label="Settings"></a></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"] == "a11y/link-name"),
        "aria-label should satisfy link-name"
    );
}

#[test]
fn a11y_generic_link_text() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><a href="/page/">click here</a><a href="/other/">mehr</a></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let generic: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "a11y/generic-link-text")
        .collect();
    assert_eq!(
        generic.len(),
        2,
        "Should detect both 'click here' and 'mehr' as generic"
    );
}

#[test]
fn a11y_button_name_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><button></button><button aria-label="OK"></button></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let button_findings: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "a11y/button-name")
        .collect();
    assert_eq!(
        button_findings.len(),
        1,
        "Only the empty button, not the one with aria-label"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_form_label_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><input type="text" name="q"><input type="hidden" name="secret"><label for="email">Email</label><input type="email" id="email" name="email"></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let label_findings: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "a11y/form-label")
        .collect();
    assert_eq!(
        label_findings.len(),
        1,
        "Only the text input without label, not hidden or labeled ones"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_form_label_wrapped_input_ok() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><label>Search<input type="text" name="q"></label></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let has_form_label = findings.iter().any(|f| f["rule_id"] == "a11y/form-label");
    assert!(!has_form_label, "Wrapped input should count as labeled");
    assert_eq!(code, 0);
}

#[test]
fn a11y_aria_hidden_focusable() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><button aria-hidden="true">Bad</button><span aria-hidden="true">OK</span><div tabindex="0" aria-hidden="true">Bad div</div><div tabindex="-1" aria-hidden="true">OK div</div></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let aria_findings: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "a11y/aria-hidden-focusable")
        .collect();
    assert_eq!(
        aria_findings.len(),
        2,
        "Button and tabindex=0 div, but not span or tabindex=-1 div"
    );
}

// ==========================================================================
// Link checks
// ==========================================================================

#[test]
fn links_broken_internal() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    // Add a link to a page that doesn't exist
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><h1>Home</h1><a href="/nonexistent/">Bad link</a></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule_id"] == "links/broken"));
    assert_eq!(code, 1);
}

#[test]
fn links_query_params() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    write_valid_page(dir.path(), "about/index.html", "About", "About", "/about/");
    // Overwrite index with a query-param link
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><h1>Home</h1><a href="/about/?ref=nav">About</a></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "links/query-params"));
    assert_eq!(code, 1);
}

#[test]
fn links_valid_internal_no_error() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    write_valid_page(dir.path(), "about/index.html", "About", "About", "/about/");
    // Overwrite with a valid link
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><h1>Home</h1><a href="/about/">About</a></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(!findings.iter().any(|f| f["rule_id"] == "links/broken"));
    assert_eq!(code, 0);
}

// ==========================================================================
// Security checks
// ==========================================================================

#[test]
fn security_target_blank_noopener() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><a href="https://ext.com" target="_blank">External</a><a href="https://safe.com" target="_blank" rel="noopener">Safe</a></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let target_blank: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "security/target-blank-noopener")
        .collect();
    assert_eq!(target_blank.len(), 1, "Only the one without rel=noopener");
}

#[test]
fn security_mixed_content() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><img src="http://insecure.com/img.jpg" alt="mixed"><img src="https://secure.com/img.jpg" alt="fine"></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let mixed: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "security/mixed-content")
        .collect();
    assert_eq!(mixed.len(), 1, "Only the http:// image");
}

#[test]
fn security_inline_scripts() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><script>alert(1);</script><script type="application/ld+json">{"@type":"test"}</script></body></html>"#,
    ).unwrap();
    // inline scripts warning is off by default; need a config file to enable it
    // Enable security checks via config-stdin (which doesn't toggle inline scripts)
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"security":{"check_target_blank":true,"check_mixed_content":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    // Default security config doesn't enable warn_inline_scripts
    let inline: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "security/inline-scripts")
        .collect();
    assert!(inline.is_empty(), "warn_inline_scripts defaults to false");
}

// ==========================================================================
// Structured data checks
// ==========================================================================

#[test]
fn structured_data_invalid_json() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><script type="application/ld+json">{bad json}</script></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "structured-data/invalid-json"));
    assert_eq!(code, 1);
}

#[test]
fn structured_data_empty_script() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><script type="application/ld+json">  </script></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "structured-data/empty"));
    assert_eq!(code, 1);
}

#[test]
fn structured_data_valid_json_ld() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><script type="application/ld+json">{"@context":"https://schema.org","@type":"WebPage"}</script></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(!findings.iter().any(|f| f["rule_id"]
        .as_str()
        .unwrap()
        .starts_with("structured-data/")));
}

// ==========================================================================
// Content quality checks
// ==========================================================================

#[test]
fn content_quality_duplicate_titles() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Same Title", "Home", "/");
    write_valid_page(
        dir.path(),
        "about/index.html",
        "Same Title",
        "About",
        "/about/",
    );
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"content_quality":{"detect_duplicate_titles":true,"detect_duplicate_descriptions":true,"detect_duplicate_h1":true,"detect_duplicate_pages":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "content/duplicate-title"));
}

#[test]
fn content_quality_unique_titles_no_warning() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    write_valid_page(dir.path(), "about/index.html", "About", "About", "/about/");
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"content_quality":{"detect_duplicate_titles":true,"detect_duplicate_descriptions":true,"detect_duplicate_h1":true,"detect_duplicate_pages":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(!findings
        .iter()
        .any(|f| f["rule_id"] == "content/duplicate-title"));
}

// ==========================================================================
// Assets checks
// ==========================================================================

#[test]
fn assets_broken_img() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><img src="/missing.jpg" alt="gone"></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"assets":{"check_broken_assets":true,"check_image_dimensions":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule_id"] == "assets/broken"));
    assert_eq!(code, 1);
}

#[test]
fn assets_img_dimensions_missing() {
    let dir = TempDir::new().unwrap();
    // Create a real image file so it's not reported as broken
    fs::write(dir.path().join("photo.jpg"), "fake image").unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><img src="/photo.jpg" alt="photo"><img src="/photo.jpg" alt="sized" width="100" height="100"></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"assets":{"check_broken_assets":true,"check_image_dimensions":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    let dim_findings: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "assets/img-dimensions")
        .collect();
    assert_eq!(dim_findings.len(), 1, "Only the img without width/height");
}

#[test]
fn assets_existing_img_no_broken_error() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("photo.jpg"), "fake image data").unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><img src="/photo.jpg" alt="exists" width="100" height="100"></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"assets":{"check_broken_assets":true,"check_image_dimensions":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"] == "assets/broken"),
        "Existing asset should not be broken"
    );
}

// ==========================================================================
// Robots.txt checks
// ==========================================================================

#[test]
fn robots_txt_missing_when_required() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"robots_txt":{"require":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "robots-txt/missing"));
}

#[test]
fn robots_txt_no_sitemap_link() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    fs::write(dir.path().join("robots.txt"), "User-agent: *\nAllow: /\n").unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"robots_txt":{"require":true,"require_sitemap_link":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "robots-txt/no-sitemap"));
}

#[test]
fn robots_txt_with_sitemap_ok() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    fs::write(
        dir.path().join("robots.txt"),
        "User-agent: *\nAllow: /\nSitemap: https://example.com/sitemap.xml\n",
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"robots_txt":{"require":true,"require_sitemap_link":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(!findings
        .iter()
        .any(|f| f["rule_id"].as_str().unwrap().starts_with("robots-txt/")));
}

// ==========================================================================
// Edge cases: empty file, whitespace, malformed HTML
// ==========================================================================

#[test]
fn edge_case_empty_file() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("index.html"), "").unwrap();
    let (json, code) = run_audit_json(dir.path(), "{}");
    let findings = json["findings"].as_array().unwrap();
    // Empty file should trigger many missing checks
    let rule_ids: Vec<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(rule_ids.contains(&"canonical/missing"));
    assert!(rule_ids.contains(&"html/lang-missing"));
    assert!(rule_ids.contains(&"html/title-missing"));
    assert!(rule_ids.contains(&"headings/no-h1"));
    assert_eq!(code, 1);
    // Should NOT crash
}

#[test]
fn edge_case_whitespace_only_file() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("index.html"), "   \n\n  \n").unwrap();
    let (_json, code) = run_audit_json(dir.path(), "{}");
    // Should not crash, should have errors
    assert_eq!(code, 1);
}

#[test]
fn edge_case_malformed_html() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><title>Malformed</title><link rel="canonical" href="https://example.com/"></head><body><h1>OK</h1><p>Unclosed <div>mismatched</span></body></html>"#,
    ).unwrap();
    // Should not crash; scraper is tolerant of malformed HTML
    let (_json, _code) =
        run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
}

#[test]
fn edge_case_no_doctype() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<html><head><title>No DOCTYPE</title></head><body><h1>Test</h1></body></html>"#,
    )
    .unwrap();
    let (json, code) = run_audit_json(dir.path(), "{}");
    let findings = json["findings"].as_array().unwrap();
    // Should detect missing lang and canonical at minimum
    let rule_ids: Vec<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(rule_ids.contains(&"html/lang-missing"));
    assert!(rule_ids.contains(&"canonical/missing"));
    assert_eq!(code, 1);
}

// ==========================================================================
// CLI flags: --strict
// ==========================================================================

#[test]
fn strict_mode_warnings_become_errors() {
    let dir = TempDir::new().unwrap();
    // Create a page with only warnings (e.g., multiple h1)
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>First</h1><h1>Second</h1></body></html>"#,
    ).unwrap();
    // Without strict: exit 0 (only warnings)
    let (_, _, code_normal) = run_audit(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"format":"json"}"#,
    );
    assert_eq!(code_normal, 0, "Warnings should exit 0 without strict");

    // With strict: exit 1
    let (_, _, code_strict) = run_audit(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"strict":true,"format":"json"}"#,
    );
    assert_eq!(code_strict, 1, "Warnings should exit 1 with strict");
}

// ==========================================================================
// CLI: --format json produces valid JSON
// ==========================================================================

#[test]
fn json_output_structure() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    assert!(json["findings"].is_array(), "findings should be array");
    assert!(json["summary"].is_object(), "summary should be object");
    assert!(json["summary"]["errors"].is_number());
    assert!(json["summary"]["warnings"].is_number());
    assert!(json["summary"]["files_checked"].is_number());
    assert_eq!(json["summary"]["files_checked"].as_u64().unwrap(), 1);
}

// ==========================================================================
// CLI: exit code 2 on bad dist path
// ==========================================================================

#[test]
fn exit_code_2_on_bad_dist_path() {
    let (_, stderr, code) = run_audit(Path::new("/nonexistent/dist/path"), "{}");
    assert_eq!(code, 2);
    assert!(stderr.contains("does not exist") || stderr.contains("Error"));
}

// ==========================================================================
// Include/Exclude glob filters
// ==========================================================================

#[test]
fn exclude_glob_skips_files() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    // Create a bad page
    fs::write(
        dir.path().join("bad.html"),
        r#"<!DOCTYPE html><html><head><title></title></head><body></body></html>"#,
    )
    .unwrap();
    // Exclude bad.html
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"filters":{"exclude":["bad.html"]}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    // bad.html should be excluded, so no findings from it
    assert!(
        !findings
            .iter()
            .any(|f| f["file"].as_str().unwrap() == "bad.html"),
        "Excluded file should not have findings"
    );
    assert_eq!(code, 0);
}

#[test]
fn config_exclude_filters_files() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    // Create pages that should be excluded via config
    fs::write(
        dir.path().join("404.html"),
        r#"<!DOCTYPE html><html><head><title>Not Found</title></head><body><h1>404</h1></body></html>"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("drafts")).unwrap();
    fs::write(
        dir.path().join("drafts/index.html"),
        r#"<!DOCTYPE html><html><head><title>Draft</title></head><body><h1>Draft</h1></body></html>"#,
    )
    .unwrap();
    // Config excludes 404.html and drafts/**
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"filters":{"exclude":["404.html","drafts/**"]}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["file"].as_str().unwrap().contains("404")),
        "Config-excluded 404.html should not have findings"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f["file"].as_str().unwrap().contains("drafts")),
        "Config-excluded drafts/** should not have findings"
    );
    assert_eq!(code, 0);
}

#[test]
fn include_glob_limits_files() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    // bad page in another dir
    fs::create_dir_all(dir.path().join("blog")).unwrap();
    fs::write(
        dir.path().join("blog/index.html"),
        r#"<!DOCTYPE html><html><head><title></title></head><body></body></html>"#,
    )
    .unwrap();
    // Include only index.html at root
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"filters":{"include":["index.html"]}}"#,
    );
    let summary = &json["summary"];
    assert_eq!(summary["files_checked"].as_u64().unwrap(), 1);
}

// ==========================================================================
// Sitemap checks
// ==========================================================================

#[test]
fn sitemap_missing_when_required() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"sitemap":{"require":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule_id"] == "sitemap/missing"));
    assert_eq!(code, 1);
}

#[test]
fn sitemap_stale_entry() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    // Sitemap references a page that doesn't exist
    fs::write(
        dir.path().join("sitemap.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>https://example.com/</loc></url><url><loc>https://example.com/deleted-page/</loc></url></urlset>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "sitemap/entry-not-in-dist"));
}

#[test]
fn no_sitemap_check_via_config() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"sitemap":{"require":false,"canonical_must_be_in_sitemap":false,"forbid_noncanonical_in_sitemap":false,"entries_must_exist_in_dist":false}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(!findings
        .iter()
        .any(|f| f["rule_id"].as_str().unwrap().starts_with("sitemap/")));
    assert_eq!(code, 0);
}

// ==========================================================================
// Max-errors cap
// ==========================================================================

#[test]
fn max_errors_caps_output() {
    let dir = TempDir::new().unwrap();
    // Create a page with many errors (missing lang, title, viewport, canonical = 4+ errors)
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head></head><body><img src="/a.jpg"><img src="/b.jpg"><a href="/x/"></a><button></button><input type="text" name="q"></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"max_errors":2}"#);
    let findings = json["findings"].as_array().unwrap();
    let error_count = findings.iter().filter(|f| f["level"] == "error").count();
    // With --max-errors=2, at most 2 errors
    assert!(
        error_count <= 2,
        "Should have at most 2 errors, got {}",
        error_count
    );
}

#[test]
fn max_errors_exact_count() {
    let dir = TempDir::new().unwrap();
    // Create many pages each with a broken link (all discovered in a single check_all run)
    // This ensures the post-processing cap works even when one check returns many errors
    for i in 0..5 {
        let page_dir = dir.path().join(format!("page{}", i));
        fs::create_dir_all(&page_dir).unwrap();
        fs::write(
            page_dir.join("index.html"),
            format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Page {i}</title>
  <link rel="canonical" href="https://example.com/page{i}/">
</head>
<body>
  <h1>Page {i}</h1>
  <a href="/nonexistent{i}/">broken</a>
</body>
</html>"#
            ),
        )
        .unwrap();
    }
    // Also add root page
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");

    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"max_errors":2}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    let error_count = findings.iter().filter(|f| f["level"] == "error").count();
    assert!(
        error_count <= 2,
        "With --max-errors=2, should have at most 2 errors, got {}",
        error_count
    );
}

#[test]
fn max_errors_truncated_in_json() {
    let dir = TempDir::new().unwrap();
    // Create multiple pages that each produce errors in the same check module (seo)
    for i in 0..5 {
        let page_dir = dir.path().join(format!("p{}", i));
        fs::create_dir_all(&page_dir).unwrap();
        // Each page missing canonical = 1 error each from seo::check_all
        fs::write(
            page_dir.join("index.html"),
            format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Page {i}</title>
</head>
<body><h1>Page {i}</h1></body>
</html>"#
            ),
        )
        .unwrap();
    }

    let (json, _) = run_audit_json(dir.path(), r#"{"max_errors":1}"#);
    let findings = json["findings"].as_array().unwrap();
    let error_count = findings.iter().filter(|f| f["level"] == "error").count();
    // The seo check runs in parallel and returns 5 errors,
    // then the post-processing caps to exactly 1
    assert!(
        error_count <= 1,
        "With --max-errors=1, should have at most 1 error, got {}",
        error_count
    );
    // truncated should be true when errors were actually removed
    if error_count == 1 {
        assert!(
            json["summary"]["truncated"].as_bool().unwrap_or(false),
            "Summary should indicate truncation when errors were capped"
        );
    }
}

#[test]
fn config_validation_rejects_zero_max_errors() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (_stdout, stderr, code) = run_audit(dir.path(), r#"{"max_errors":0}"#);
    assert_eq!(code, 2);
    assert!(stderr.contains("max_errors must be greater than 0"));
}

#[test]
fn config_validation_rejects_invalid_external_links_settings() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (_stdout, stderr, code) = run_audit(
        dir.path(),
        r#"{"external_links":{"enabled":true,"timeout_ms":0,"max_concurrent":10}}"#,
    );
    assert_eq!(code, 2);
    assert!(stderr.contains("external_links.timeout_ms must be greater than 0"));
}

// ==========================================================================
// Config file auto-discovery
// ==========================================================================

// ==========================================================================
// Config file loading (explicit)
// ==========================================================================

#[test]
fn config_disables_checks() {
    let dir = TempDir::new().unwrap();
    // Page with missing lang - normally an error
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    // Config that disables lang check
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"html_basics":{"lang_attr_required":false}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"] == "html/lang-missing"),
        "Disabled check should not fire"
    );
}

// ==========================================================================
// Snapshot test: text output format
// ==========================================================================

#[test]
fn text_output_format_structure() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (stdout, _, code) = run_audit(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    assert_eq!(code, 0);
    assert!(stdout.contains("All checks passed") || stdout.contains("Summary"));
}

#[test]
fn text_output_with_errors() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("index.html"), "").unwrap();
    let (stdout, _, code) = run_audit(dir.path(), "{}");
    assert_eq!(code, 1);
    assert!(
        stdout.contains("error"),
        "Output should contain error summary"
    );
    assert!(
        stdout.contains("file"),
        "Output should mention files checked"
    );
}

// ==========================================================================
// JSON snapshot test: verify structure of findings
// ==========================================================================

#[test]
fn json_finding_structure() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("index.html"), "").unwrap();
    let (json, _) = run_audit_json(dir.path(), "{}");
    let findings = json["findings"].as_array().unwrap();
    assert!(!findings.is_empty());
    let f = &findings[0];
    assert!(f["level"].is_string());
    assert!(f["rule_id"].is_string());
    assert!(f["file"].is_string());
    assert!(f["selector"].is_string());
    assert!(f["message"].is_string());
    assert!(f["help"].is_string());
}

// ==========================================================================
// Assets: require_hashed_filenames
// ==========================================================================

#[test]
fn assets_unhashed_filename_warns() {
    let dir = TempDir::new().unwrap();
    // Page references a script without hash in filename
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://example.com/">
  <link rel="stylesheet" href="/styles/main.css">
</head>
<body>
  <h1>Test</h1>
  <script src="/js/app.js"></script>
</body>
</html>"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("styles")).unwrap();
    fs::write(dir.path().join("styles/main.css"), "body {}").unwrap();
    fs::create_dir_all(dir.path().join("js")).unwrap();
    fs::write(dir.path().join("js/app.js"), "console.log('hi')").unwrap();

    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"assets":{"require_hashed_filenames":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    let unhashed: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "assets/unhashed-filename")
        .collect();
    assert_eq!(
        unhashed.len(),
        2,
        "Should warn for both unhashed JS and CSS"
    );
}

#[test]
fn assets_hashed_filename_no_warning() {
    let dir = TempDir::new().unwrap();
    // Page references assets WITH hashed filenames
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://example.com/">
  <link rel="stylesheet" href="/_astro/style.DfQ4EE2a.css">
</head>
<body>
  <h1>Test</h1>
  <script src="/_astro/main.a1b2c3d4.js"></script>
</body>
</html>"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("_astro")).unwrap();
    fs::write(dir.path().join("_astro/style.DfQ4EE2a.css"), "body {}").unwrap();
    fs::write(
        dir.path().join("_astro/main.a1b2c3d4.js"),
        "console.log('hi')",
    )
    .unwrap();

    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"assets":{"require_hashed_filenames":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "assets/unhashed-filename"),
        "Hashed filenames should not trigger warning"
    );
}

// ==========================================================================
// External links checking
// ==========================================================================

#[test]
fn external_links_disabled_by_default() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><a href="https://httpstat.us/404">Bad</a></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"]
            .as_str()
            .unwrap()
            .starts_with("external-links/")),
        "External link checks should not run when disabled (default)"
    );
}

#[test]
fn external_links_no_findings_when_no_external_links() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><a href="/about/">Internal</a></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"external_links":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"]
            .as_str()
            .unwrap()
            .starts_with("external-links/")),
        "No external link findings when page has no external links"
    );
}

// ==========================================================================
// Meta description: length check decoupled from required
// ==========================================================================

#[test]
fn meta_description_length_checked_even_when_not_required() {
    let dir = TempDir::new().unwrap();
    // Page with a too-long description, but description is NOT required
    let long_desc = "A".repeat(200);
    fs::write(
        dir.path().join("index.html"),
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <meta name="description" content="{long_desc}">
  <link rel="canonical" href="https://example.com/">
</head>
<body><h1>Test</h1></body>
</html>"#
        ),
    )
    .unwrap();

    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"html_basics":{"meta_description_required":false,"meta_description_max_length":160}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "html/meta-description-too-long"),
        "Length should be checked even when meta_description_required = false"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "html/meta-description-missing"),
        "Should NOT warn about missing description when required = false"
    );
}

#[test]
fn meta_description_missing_no_warning_when_not_required() {
    let dir = TempDir::new().unwrap();
    // Page WITHOUT description, and required = false
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://example.com/">
</head>
<body><h1>Test</h1></body>
</html>"#,
    )
    .unwrap();

    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"html_basics":{"meta_description_required":false}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "html/meta-description-missing"),
        "Should NOT warn about missing description when required = false"
    );
}

// ==========================================================================
// Severity mapping
// ==========================================================================

#[test]
fn severity_mapping_downgrades_error_to_warning() {
    let dir = TempDir::new().unwrap();
    // Page missing lang (normally an error)
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"severity":{"html/lang-missing":"warning"}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    let lang = findings
        .iter()
        .find(|f| f["rule_id"] == "html/lang-missing");
    assert!(lang.is_some(), "Should still report lang-missing");
    assert_eq!(
        lang.unwrap()["level"],
        "warning",
        "Severity should be downgraded to warning"
    );
    // Without --strict, warnings don't cause exit code 1
    assert_eq!(code, 0, "Downgraded to warning should not cause error exit");
}

#[test]
fn severity_mapping_off_suppresses_finding() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"severity":{"html/lang-missing":"off"}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"] == "html/lang-missing"),
        "Rule with severity=off should be suppressed entirely"
    );
    assert_eq!(code, 0, "No errors should mean exit code 0");
}

#[test]
fn severity_mapping_upgrades_warning_to_error() {
    let dir = TempDir::new().unwrap();
    let long_title = "A".repeat(80);
    fs::write(
        dir.path().join("index.html"),
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{long_title}</title>
  <link rel="canonical" href="https://example.com/">
</head>
<body><h1>Test</h1></body>
</html>"#
        ),
    )
    .unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"severity":{"html/title-too-long":"error"}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    let title = findings
        .iter()
        .find(|f| f["rule_id"] == "html/title-too-long");
    assert!(title.is_some(), "Should still report title-too-long");
    assert_eq!(
        title.unwrap()["level"],
        "error",
        "Severity should be upgraded to error"
    );
    assert_eq!(code, 1, "Upgraded to error should cause exit code 1");
}

// ==========================================================================
// Structured data: semantic checks
// ==========================================================================

#[test]
fn structured_data_missing_context() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://example.com/">
  <script type="application/ld+json">{"@type": "Article", "headline": "Test"}</script>
</head>
<body><h1>Test</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "structured-data/missing-context"),
        "Should warn about missing @context"
    );
}

#[test]
fn structured_data_missing_required_property() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://example.com/">
  <script type="application/ld+json">{"@context": "https://schema.org", "@type": "Article"}</script>
</head>
<body><h1>Test</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "structured-data/missing-property"
                && f["message"].as_str().unwrap_or("").contains("headline")),
        "Should warn about missing 'headline' for Article type"
    );
}

#[test]
fn structured_data_valid_article_no_semantic_warning() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://example.com/">
  <script type="application/ld+json">{"@context": "https://schema.org", "@type": "Article", "headline": "Test Article"}</script>
</head>
<body><h1>Test</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| {
            let rid = f["rule_id"].as_str().unwrap_or("");
            rid.starts_with("structured-data/missing") || rid == "structured-data/unusual-context"
        }),
        "Valid Article should produce no semantic warnings"
    );
}

#[test]
fn structured_data_duplicate_type_warns() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://example.com/">
  <script type="application/ld+json">{"@context":"https://schema.org","@type":"Article","headline":"First"}</script>
  <script type="application/ld+json">{"@context":"https://schema.org","@type":"Article","headline":"Second"}</script>
</head>
<body><h1>Test</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"detect_duplicate_types":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "structured-data/duplicate-type"),
        "Should warn about duplicate Article @type on same page"
    );
}

#[test]
fn structured_data_different_types_no_warning() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://example.com/">
  <script type="application/ld+json">{"@context":"https://schema.org","@type":"Article","headline":"Test"}</script>
  <script type="application/ld+json">{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[]}</script>
</head>
<body><h1>Test</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"detect_duplicate_types":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "structured-data/duplicate-type"),
        "Different @types should not trigger duplicate warning"
    );
}

// ==========================================================================
// Canonical cluster detection
// ==========================================================================

#[test]
fn canonical_cluster_detected() {
    let dir = TempDir::new().unwrap();
    // Two pages sharing the same canonical URL
    let pages = dir.path().join("index.html");
    fs::write(
        &pages,
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><h1>Home</h1></body></html>"#,
    ).unwrap();
    let dup_dir = dir.path().join("duplicate");
    fs::create_dir_all(&dup_dir).unwrap();
    fs::write(
        dup_dir.join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Duplicate</title><link rel="canonical" href="https://example.com/"></head><body><h1>Duplicate</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    assert_eq!(code, 0, "Clusters are warnings, not errors");
    let findings = json["findings"].as_array().unwrap();
    let cluster_findings: Vec<_> = findings
        .iter()
        .filter(|f| f["rule_id"] == "canonical/cluster")
        .collect();
    assert_eq!(
        cluster_findings.len(),
        2,
        "Both pages should get a cluster warning"
    );
}

#[test]
fn canonical_cluster_disabled() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><h1>Home</h1></body></html>"#,
    ).unwrap();
    let dup_dir = dir.path().join("duplicate");
    fs::create_dir_all(&dup_dir).unwrap();
    fs::write(
        dup_dir.join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Duplicate</title><link rel="canonical" href="https://example.com/"></head><body><h1>Duplicate</h1></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"canonical":{"detect_clusters":false}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"] == "canonical/cluster"),
        "Cluster detection should be suppressed when disabled"
    );
}

#[test]
fn canonical_no_cluster_when_unique() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><h1>Home</h1></body></html>"#,
    ).unwrap();
    let about_dir = dir.path().join("about");
    fs::create_dir_all(&about_dir).unwrap();
    fs::write(
        about_dir.join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>About</title><link rel="canonical" href="https://example.com/about/"></head><body><h1>About</h1></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"] == "canonical/cluster"),
        "Unique canonicals should not trigger cluster warning"
    );
}

// ==========================================================================
// Fragment check: percent-encoded umlauts (e.g. rehype-slug + MDX)
// ==========================================================================

#[test]
fn fragment_percent_encoded_umlaut_same_page() {
    // Href uses percent-encoded umlaut (%C3%A4 = ä), ID uses raw umlaut (rehype-slug behavior)
    let dir = TempDir::new().unwrap();
    // Build HTML with umlaut in ID but percent-encoded in href
    let html = "<!DOCTYPE html><html lang=\"de\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Test</title><link rel=\"canonical\" href=\"https://example.com/\"></head><body><h1>Test</h1><a href=\"#n%C3%A4chste-schritte\">Link</a><h2 id=\"n\u{00E4}chste-schritte\">N\u{00E4}chste Schritte</h2></body></html>";
    fs::write(dir.path().join("index.html"), html).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"links":{"check_fragments":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "links/broken-fragment"),
        "Percent-encoded umlaut fragment should match decoded ID, findings: {:?}",
        findings
            .iter()
            .filter(|f| f["rule_id"] == "links/broken-fragment")
            .collect::<Vec<_>>()
    );
}

#[test]
fn fragment_percent_encoded_umlaut_cross_page() {
    let dir = TempDir::new().unwrap();
    // Page with umlaut heading (ü = \u{00FC})
    let target_dir = dir.path().join("target");
    fs::create_dir_all(&target_dir).unwrap();
    let target_html = "<!DOCTYPE html><html lang=\"de\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Target</title><link rel=\"canonical\" href=\"https://example.com/target/\"></head><body><h1>Target</h1><h2 id=\"\u{00FC}ber-uns\">\u{00DC}ber uns</h2></body></html>";
    fs::write(target_dir.join("index.html"), target_html).unwrap();
    // Page linking to it with percent-encoded fragment (%C3%BC = ü)
    let home_html = "<!DOCTYPE html><html lang=\"de\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Home</title><link rel=\"canonical\" href=\"https://example.com/\"></head><body><h1>Home</h1><a href=\"/target/#%C3%BCber-uns\">Link</a></body></html>";
    fs::write(dir.path().join("index.html"), home_html).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"links":{"check_fragments":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "links/broken-fragment"),
        "Cross-page percent-encoded umlaut fragment should match decoded ID"
    );
}

// ==========================================================================
// Presets
// ==========================================================================

#[test]
fn preset_strict_enables_all_checks() {
    let dir = TempDir::new().unwrap();
    // Valid page but missing OG, structured data, skip-link etc.
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"preset":"strict"}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    // Strict should flag missing OG tags
    assert!(
        findings.iter().any(|f| f["rule_id"]
            .as_str()
            .unwrap_or("")
            .starts_with("opengraph/")),
        "Strict preset should enable opengraph checks"
    );
    // Strict should flag missing skip link
    assert!(
        findings.iter().any(|f| f["rule_id"] == "a11y/skip-link"),
        "Strict preset should enable skip-link check"
    );
}

#[test]
fn preset_relaxed_is_lenient() {
    let dir = TempDir::new().unwrap();
    // Page missing meta description — relaxed should not require it
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"preset":"relaxed"}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "html/meta-description-missing"),
        "Relaxed preset should not require meta description"
    );
    assert_eq!(code, 0, "Relaxed preset on valid page should exit 0");
}

#[test]
fn preset_with_user_override() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    // Use strict preset but disable opengraph
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"preset":"strict","opengraph":{"require_og_title":false,"require_og_description":false,"require_og_image":false,"require_twitter_card":false}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"]
            .as_str()
            .unwrap_or("")
            .starts_with("opengraph/")),
        "User override should disable opengraph even with strict preset"
    );
}

// ==========================================================================
// Fix suggestions in JSON output
// ==========================================================================

#[test]
fn json_suggestion_present_for_lang_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let lang = findings
        .iter()
        .find(|f| f["rule_id"] == "html/lang-missing");
    assert!(lang.is_some());
    assert!(
        lang.unwrap()["suggestion"].is_string(),
        "html/lang-missing should have a suggestion"
    );
}

#[test]
fn benchmark_json_output() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"benchmark":true}"#,
    );
    assert!(
        json["benchmark"].is_object(),
        "benchmark should be present when enabled"
    );
    assert!(json["benchmark"]["total_ms"].is_number());
    assert!(json["benchmark"]["pages_checked"].is_number());
    assert!(json["benchmark"]["pages_per_second"].is_number());
    assert!(json["benchmark"]["discovery_ms"].is_number());
    assert!(json["benchmark"]["check_timings"].is_array());
}

#[test]
fn benchmark_absent_by_default() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    assert!(
        json.get("benchmark").is_none() || json["benchmark"].is_null(),
        "benchmark should not be present by default"
    );
}

#[test]
fn json_suggestion_absent_for_broken_link() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><a href="/nonexistent/">Bad</a></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let broken = findings.iter().find(|f| f["rule_id"] == "links/broken");
    assert!(broken.is_some());
    assert!(
        broken.unwrap().get("suggestion").is_none() || broken.unwrap()["suggestion"].is_null(),
        "links/broken should not have a suggestion"
    );
}
