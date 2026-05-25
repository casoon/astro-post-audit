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
            r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{}</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
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
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>First</h1><h1>Second</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
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
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><label>Search<input type="text" name="q"></label></main><footer><a href="/">Home</a></footer></body></html>"#,
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
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Home</h1><a href="/about/">About</a></main><footer><a href="/">Home</a></footer></body></html>"#,
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
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>First</h1><h1>Second</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
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
fn sitemap_parse_error_reported() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    fs::write(
        dir.path().join("sitemap.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset><url><loc>https://example.com/</loc></url"#,
    )
    .unwrap();

    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"sitemap":{"require":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "sitemap/parse-error"));
    assert_eq!(
        code, 1,
        "With sitemap.require=true parse errors should fail"
    );
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
fn max_errors_respects_severity_overrides_before_early_stop() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/bad");
    let (json, _code) = run_audit_json(
        &fixture_path,
        r#"{"max_errors":1,"severity":{"canonical/missing":"off"}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.is_empty(),
        "max_errors should not hide unrelated findings after severity override"
    );
    assert!(
        !findings.iter().any(|f| f["rule_id"] == "canonical/missing"),
        "severity=off must still suppress canonical/missing"
    );
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

#[test]
fn text_output_includes_top_issues_when_many_findings() {
    // Generate enough empty pages to exceed the 20-finding threshold
    // and trigger the "Top issues" summary block.
    let dir = TempDir::new().unwrap();
    for i in 0..15 {
        let sub = dir.path().join(format!("p{}", i));
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("index.html"), "").unwrap();
    }
    let (stdout, _, _) = run_audit(dir.path(), "{}");
    assert!(
        stdout.contains("Top issues"),
        "Should include Top issues summary when finding count exceeds threshold"
    );
}

#[test]
fn text_output_skips_top_issues_when_few_findings() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("index.html"), "").unwrap();
    let (stdout, _, _) = run_audit(dir.path(), "{}");
    assert!(
        !stdout.contains("Top issues"),
        "Should not include Top issues summary for small result sets"
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

#[test]
fn assets_astro_directory_treated_as_hashed() {
    // Even when the filename itself looks unhashed, files under /_astro/
    // are Astro's hashed bundle output and should not be flagged.
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
  <link rel="stylesheet" href="/_astro/main.css">
</head>
<body>
  <h1>Test</h1>
  <script src="/_astro/main.js"></script>
</body>
</html>"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("_astro")).unwrap();
    fs::write(dir.path().join("_astro/main.css"), "body {}").unwrap();
    fs::write(dir.path().join("_astro/main.js"), "console.log('hi')").unwrap();

    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"assets":{"require_hashed_filenames":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "assets/unhashed-filename"),
        "Assets under /_astro/ should be treated as hashed"
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
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
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
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
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
// Innovative dist-only audits
// ==========================================================================

#[test]
fn i18n_audit_detects_route_lang_mismatch() {
    let dir = TempDir::new().unwrap();
    let de_dir = dir.path().join("de");
    fs::create_dir_all(&de_dir).unwrap();
    fs::write(
        de_dir.join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Deutsch</title>
  <link rel="canonical" href="https://example.com/de/">
  <link rel="alternate" hreflang="en" href="https://example.com/de/">
</head>
<body><h1>Deutsch</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"i18n_audit":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "i18n/lang-locale-mismatch"),
        "Should detect mismatch between /de/ route and lang='en'"
    );
}

#[test]
fn crawl_budget_detects_query_and_variant_links() {
    let dir = TempDir::new().unwrap();
    let about_dir = dir.path().join("about");
    fs::create_dir_all(&about_dir).unwrap();
    fs::write(
        about_dir.join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>About</title><link rel="canonical" href="https://example.com/about/"></head><body><h1>About</h1></body></html>"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><h1>Home</h1><a href="/about/?ref=nav">About</a><a href="/about/index.html">About variant</a></body></html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"crawl_budget":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "crawl-budget/query-variants"));
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "crawl-budget/non-canonical-link-variant"));
}

#[test]
fn render_blocking_detects_sync_scripts_and_missing_hints() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("styles")).unwrap();
    fs::write(dir.path().join("styles/main.css"), "body{}").unwrap();
    fs::create_dir_all(dir.path().join("js")).unwrap();
    fs::write(dir.path().join("js/app.js"), "console.log('x');").unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Home</title>
  <link rel="canonical" href="https://example.com/">
  <script src="/js/app.js"></script>
  <script src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js"></script>
  <link rel="stylesheet" href="/styles/main.css">
</head>
<body><h1>Home</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"render_blocking":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "render-blocking/sync-head-scripts"));
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "render-blocking/missing-style-preload"));
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "render-blocking/missing-preconnect"));
}

#[test]
fn privacy_security_detects_third_party_sri_and_consent_gaps() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Home</title>
  <link rel="canonical" href="https://example.com/">
  <script src="https://www.googletagmanager.com/gtm.js"></script>
  <script>window.inline = true;</script>
</head>
<body><h1>Home</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"privacy_security":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "privacy-security/third-party-domains"));
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "privacy-security/missing-sri-script"));
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "privacy-security/csp-readiness-inline-script"));
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "privacy-security/missing-consent-indicator"));
}

#[test]
fn structured_data_graph_detects_cross_page_conflicts() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Home</title>
  <link rel="canonical" href="https://example.com/">
  <script type="application/ld+json">{"@context":"https://schema.org","@id":"https://example.com/#org","@type":"Organization","name":"Example Org","url":"https://example.com/"}</script>
</head>
<body><h1>Home</h1></body>
</html>"#,
    )
    .unwrap();
    let about_dir = dir.path().join("about");
    fs::create_dir_all(&about_dir).unwrap();
    fs::write(
        about_dir.join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>About</title>
  <link rel="canonical" href="https://example.com/about/">
  <script type="application/ld+json">{"@context":"https://schema.org","@id":"https://example.com/#org","@type":"Person","name":"Example Person","url":"https://example.com/missing/"}</script>
</head>
<body><h1>About</h1></body>
</html>"#,
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data_graph":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "structured-data-graph/type-conflict"));
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "structured-data-graph/internal-url-missing"));
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
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Home</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Home</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let dup_dir = dir.path().join("duplicate");
    fs::create_dir_all(&dup_dir).unwrap();
    fs::write(
        dup_dir.join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Duplicate</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Duplicate</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
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
        r#"{"site":{"base_url":"https://example.com"},"preset":"strict","opengraph":{"require_og_title":false,"require_og_description":false,"require_og_image":false,"require_twitter_card":false,"require_og_type":false,"require_og_url":false,"og_image_absolute_url":false,"twitter_card_valid_values":false}}"#,
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

#[test]
fn preset_seo_is_supported() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"preset":"seo"}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert_eq!(code, 1, "SEO preset should run without config parse errors");
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "html/meta-description-missing"),
        "SEO preset should require meta descriptions"
    );
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "opengraph/title-missing"),
        "SEO preset should enable Open Graph checks"
    );
}

#[test]
fn preset_accessibility_is_supported() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"preset":"accessibility"}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert_eq!(
        code, 0,
        "Accessibility preset should run without config parse errors"
    );
    assert!(
        findings.iter().any(|f| f["rule_id"] == "a11y/skip-link"),
        "Accessibility preset should require skip links"
    );
}

#[test]
fn max_warnings_fails_when_threshold_exceeded() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>This title is intentionally much longer than sixty characters to trigger warning</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Home</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"max_warnings":0}"#,
    );
    assert_eq!(json["summary"]["warnings"].as_u64(), Some(1));
    assert_eq!(code, 1, "max_warnings=0 should fail on any warning");
}

#[test]
fn baseline_write_and_filter() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head><title></title></head><body><img src="/x.png"></body></html>"#,
    ).unwrap();
    let baseline_path = dir.path().join("baseline.json");
    let baseline_str = baseline_path.to_string_lossy().replace('\\', "/");

    let (_, _, write_code) = run_audit(
        dir.path(),
        &format!(
            r#"{{"format":"json","baseline":"{}","write_baseline":true}}"#,
            baseline_str
        ),
    );
    assert_eq!(write_code, 0, "writing a baseline should exit successfully");
    assert!(baseline_path.exists(), "baseline file should be written");

    let (json, code) = run_audit_json(dir.path(), &format!(r#"{{"baseline":"{}"}}"#, baseline_str));
    assert_eq!(
        json["findings"].as_array().unwrap().len(),
        0,
        "baseline should suppress existing findings"
    );
    assert_eq!(code, 0, "baseline-suppressed findings should not fail");
}

#[test]
fn hreflang_self_reference_not_checked_without_base_url() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Home</title>
  <link rel="canonical" href="https://example.com/">
  <link rel="alternate" hreflang="en" href="/">
</head>
<body><h1>Home</h1></body>
</html>"#,
    )
    .unwrap();

    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"hreflang":{"check_hreflang":true,"require_self_reference":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "hreflang/no-self-reference"),
        "Self-reference hreflang check should be skipped without site.base_url"
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

// ==========================================================================
// Go-live gate checks (#11)
// ==========================================================================

#[test]
fn golive_disabled_no_findings() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"go_live":{"enabled":false},"site":{"base_url":"https://example.com"}}"#,
    );
    let rule_ids: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .filter(|id| id.starts_with("golive/"))
        .collect();
    assert!(
        rule_ids.is_empty(),
        "go-live disabled should emit no golive findings"
    );
    assert_eq!(code, 0);
}

#[test]
fn golive_enabled_no_site_emits_config_error() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, code) = run_audit_json(dir.path(), r#"{"go_live":{"enabled":true}}"#);
    let rule_ids: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(
        rule_ids.contains(&"golive/config-missing-site"),
        "Should emit config-missing-site when no expected site, got: {:?}",
        rule_ids
    );
    assert_eq!(code, 1);
}

#[test]
fn golive_noindex_fails_when_enabled() {
    let dir = TempDir::new().unwrap();
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <meta name="robots" content="noindex">
  <title>Test</title>
  <link rel="canonical" href="https://prod.example.com/">
</head>
<body><h1>Test</h1></body>
</html>"#;
    fs::write(dir.path().join("index.html"), html).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"go_live":{"enabled":true},"site":{"base_url":"https://prod.example.com"}}"#,
    );
    let rule_ids: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(
        rule_ids.contains(&"golive/noindex"),
        "Should flag noindex pages"
    );
    assert_eq!(code, 1);
}

#[test]
fn golive_canonical_wrong_origin_fails() {
    let dir = TempDir::new().unwrap();
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://staging.example.com/">
</head>
<body><h1>Test</h1></body>
</html>"#;
    fs::write(dir.path().join("index.html"), html).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"go_live":{"enabled":true},"site":{"base_url":"https://prod.example.com"}}"#,
    );
    let rule_ids: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(
        rule_ids.contains(&"golive/canonical-origin"),
        "Should flag canonical pointing to staging origin, got: {:?}",
        rule_ids
    );
    assert_eq!(code, 1);
}

#[test]
fn golive_forbidden_domain_in_link_fails() {
    let dir = TempDir::new().unwrap();
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Test</title>
  <link rel="canonical" href="https://prod.example.com/">
</head>
<body>
  <h1>Test</h1>
  <a href="https://staging.example.com/page">staging link</a>
</body>
</html>"#;
    fs::write(dir.path().join("index.html"), html).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"go_live":{"enabled":true,"forbidden_domains":["staging.example.com"]},"site":{"base_url":"https://prod.example.com"}}"#,
    );
    let rule_ids: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(
        rule_ids.contains(&"golive/forbidden-domain"),
        "Should flag forbidden domain in link, got: {:?}",
        rule_ids
    );
    assert_eq!(code, 1);
}

#[test]
fn golive_robots_txt_global_disallow_fails() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    fs::write(
        dir.path().join("robots.txt"),
        "User-agent: *\nDisallow: /\n",
    )
    .unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"go_live":{"enabled":true},"site":{"base_url":"https://prod.example.com"}}"#,
    );
    let rule_ids: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert!(
        rule_ids.contains(&"golive/robots-blocked"),
        "Should flag robots.txt global disallow, got: {:?}",
        rule_ids
    );
    assert_eq!(code, 1);
}

#[test]
fn golive_robots_txt_partial_disallow_passes() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    fs::write(
        dir.path().join("robots.txt"),
        "User-agent: *\nDisallow: /admin/\n",
    )
    .unwrap();
    let (json, _code) = run_audit_json(
        dir.path(),
        r#"{"go_live":{"enabled":true},"site":{"base_url":"https://prod.example.com"}}"#,
    );
    let golive_rules: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .filter(|id| *id == "golive/robots-blocked")
        .collect();
    assert!(
        golive_rules.is_empty(),
        "Partial disallow should not trigger robots-blocked"
    );
}

// ==========================================================================
// Config parity: TS ↔ Rust option surface (#15)
// ==========================================================================

#[test]
fn config_parity_fixture_deserializes_cleanly() {
    // Verifies that the full config JSON (matching the TypeScript API surface)
    // round-trips through Rust deserialization without unknown-field errors.
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/config-parity.json");
    let json = fs::read_to_string(&fixture)
        .unwrap_or_else(|_| panic!("missing parity fixture: {}", fixture.display()));

    // Deserialize without error
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    // Run with format=json to capture output; go_live.enabled=true will check
    // origin and may find canonical/sitemap issues — we only care that it
    // doesn't crash or produce a parse error (exit code 2).
    let mut config: serde_json::Value = serde_json::from_str(&json).unwrap();
    // Patch go_live.expected_site to match the canonical in write_valid_page
    config["go_live"]["expected_site"] = serde_json::json!("https://example.com");
    config["go_live"]["enabled"] = serde_json::json!(false);
    config["format"] = serde_json::json!("json");
    // Remove extra_reports paths that don't exist in the test environment
    config["extra_reports"] = serde_json::json!([]);

    let (_stdout, _stderr, code) = run_audit(dir.path(), &config.to_string());
    assert_ne!(
        code, 2,
        "Config must parse without error (exit code 2 = parse/runtime error)"
    );
}

#[test]
fn config_parity_all_preset_names_valid() {
    // Verifies that every preset name documented in the TypeScript API is accepted by Rust.
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    for preset in &[
        "strict",
        "relaxed",
        "seo",
        "accessibility",
        "performance",
        "production",
        "standard",
    ] {
        let config = format!(r#"{{"preset":"{}","format":"json"}}"#, preset);
        let (_stdout, _stderr, code) = run_audit(dir.path(), &config);
        assert_ne!(
            code, 2,
            "Preset '{}' should be recognized by Rust config parser",
            preset
        );
    }
}

#[test]
fn config_parity_severity_level_names_valid() {
    // Verifies that all severity level names from the TypeScript API parse in Rust.
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let config = r#"{
        "format": "json",
        "severity": {
            "html/title-too-long": "error",
            "a11y/img-alt": "warning",
            "links/orphan-page": "info",
            "canonical/missing": "off"
        }
    }"#;
    let (_stdout, _stderr, code) = run_audit(dir.path(), config);
    assert_ne!(code, 2, "All severity level names must be accepted");
}

// ==========================================================================
// A11y: Landmark structure (#20)
// ==========================================================================

#[test]
fn a11y_landmark_main_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "a11y/landmark-main-missing"),
        "Missing <main> should be reported"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_landmark_main_duplicate() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>First</h1></main><main><p>Second</p></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "a11y/landmark-main-duplicate"),
        "Two <main> elements should be reported"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_landmark_nav_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><main><h1>Test</h1></main></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "a11y/landmark-nav-missing"),
        "Missing <nav> should be a warning"
    );
}

#[test]
fn a11y_landmark_all_present_no_landmark_errors() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    let landmark_errors: Vec<_> = findings
        .iter()
        .filter(|f| {
            f["rule_id"]
                .as_str()
                .unwrap_or("")
                .starts_with("a11y/landmark-")
        })
        .collect();
    assert!(
        landmark_errors.is_empty(),
        "Complete landmark structure should produce no landmark findings"
    );
}

// ==========================================================================
// A11y: Duplicate ID detection (#21)
// ==========================================================================

#[test]
fn a11y_duplicate_id_detected() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><div id="foo">First</div><div id="foo">Second</div></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings.iter().any(|f| f["rule_id"] == "a11y/duplicate-id"),
        "Duplicate id should be reported"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_duplicate_id_aria_ref() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><span id="label">Label</span><span id="label">Dupe</span><button aria-labelledby="label">Click</button></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "a11y/duplicate-id-aria"),
        "Duplicate id referenced by ARIA should be reported as duplicate-id-aria"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_unique_ids_pass() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><div id="unique1">A</div><div id="unique2">B</div></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"]
            .as_str()
            .unwrap_or("")
            .starts_with("a11y/duplicate-id")),
        "Unique ids should produce no duplicate-id findings"
    );
}

// ==========================================================================
// A11y: ARIA role validation (#22)
// ==========================================================================

#[test]
fn a11y_aria_role_invalid() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><div role="buttton">Typo</div></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "a11y/aria-role-invalid"),
        "Typo in role name should be reported"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_aria_role_abstract() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><div role="widget">Abstract</div></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "a11y/aria-role-abstract"),
        "Abstract role should be reported"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_aria_checkbox_missing_checked() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><div role="checkbox">No aria-checked</div></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "a11y/aria-required-attr"),
        "role=checkbox without aria-checked should be reported"
    );
    assert_eq!(code, 1);
}

#[test]
fn a11y_aria_valid_roles_pass() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><div role="alert">Valid</div><span role="status">Also valid</span><div role="checkbox" aria-checked="false">Checked</div></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"]
            .as_str()
            .unwrap_or("")
            .starts_with("a11y/aria-role")),
        "Valid ARIA roles should produce no role findings"
    );
}

// ==========================================================================
// SEO: Open Graph extended (#23)
// ==========================================================================

#[test]
fn og_type_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"opengraph":{"require_og_type":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "opengraph/type-missing"),
        "Missing og:type should be reported when required"
    );
}

#[test]
fn og_url_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"opengraph":{"require_og_url":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "opengraph/url-missing"),
        "Missing og:url should be reported when required"
    );
}

#[test]
fn og_image_relative_url() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><meta property="og:image" content="/images/hero.jpg"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "opengraph/image-not-absolute"),
        "Relative og:image should be reported as error"
    );
    assert_eq!(code, 1);
}

#[test]
fn twitter_card_invalid_value() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><meta name="twitter:card" content="large_image"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "opengraph/twitter-card-invalid"),
        "Invalid twitter:card value should be reported"
    );
    assert_eq!(code, 1);
}

#[test]
fn og_type_present_pass() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><meta property="og:type" content="website"><meta property="og:url" content="https://example.com/"><meta name="twitter:card" content="summary_large_image"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"opengraph":{"require_og_type":true,"require_og_url":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "opengraph/type-missing"),
        "Present og:type should not be reported"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"] == "opengraph/twitter-card-invalid"),
        "Valid twitter:card should not be reported"
    );
}

// ==========================================================================
// Structured Data: property completeness (#24)
// ==========================================================================

#[test]
fn structured_data_article_no_author() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><script type="application/ld+json">{"@context":"https://schema.org","@type":"Article","headline":"Test"}</script></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "structured-data/article-missing-author"),
        "Article without author should be reported"
    );
}

#[test]
fn structured_data_faqpage_no_answer() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><script type="application/ld+json">{"@context":"https://schema.org","@type":"FAQPage","mainEntity":[{"@type":"Question","name":"What?"}]}</script></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "structured-data/faq-missing-answer"),
        "FAQPage without acceptedAnswer should be reported"
    );
    assert_eq!(code, 1);
}

#[test]
fn structured_data_breadcrumb_no_position() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"><script type="application/ld+json">{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","name":"Home"}]}</script></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"structured_data":{"check_json_ld":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "structured-data/breadcrumb-missing-position"),
        "BreadcrumbList item without position should be reported"
    );
    assert_eq!(code, 1);
}

// ==========================================================================
// Images: CLS & efficiency (#25)
// ==========================================================================

#[test]
fn image_missing_width_height() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><img src="/photo.jpg" alt="Photo"></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "images/missing-dimensions"),
        "Image without width/height should be reported as error"
    );
    assert_eq!(code, 1);
}

#[test]
fn image_missing_lazy_loading() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><img src="/hero.jpg" alt="Hero" width="800" height="600"><img src="/second.jpg" alt="Second" width="400" height="300"></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "images/missing-lazy"),
        "Second image without loading=lazy should be warned"
    );
}

#[test]
fn image_format_hint_jpeg() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><img src="/photo.jpg" alt="Photo" width="400" height="300" loading="lazy"></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"images":{"format_hints":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "images/legacy-format"),
        "JPEG image should suggest WebP when format_hints is enabled"
    );
}

#[test]
fn image_complete_pass() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><img src="/hero.webp" alt="Hero" width="800" height="600" loading="eager" srcset="/hero-400.webp 400w, /hero.webp 800w"></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"images":{"format_hints":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"].as_str().unwrap_or("").starts_with("images/")),
        "Complete image markup should produce no image findings"
    );
}

// ==========================================================================
// robots.txt: extended audit (#26)
// ==========================================================================

#[test]
fn robots_global_disallow_all() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    fs::write(
        dir.path().join("robots.txt"),
        "User-agent: *\nDisallow: /\n",
    )
    .unwrap();
    let (json, code) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "robots-txt/disallow-all"),
        "Global Disallow: / should be reported as error"
    );
    assert_eq!(code, 1);
}

#[test]
fn robots_ai_citation_bot_blocked() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    fs::write(
        dir.path().join("robots.txt"),
        "User-agent: *\nAllow: /\n\nUser-agent: GPTBot\nDisallow: /\n",
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"robots_txt":{"ai_bot_policy":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "robots-txt/ai-citation-bot-blocked"),
        "Blocked AI citation bot should be warned when ai_bot_policy is true"
    );
}

#[test]
fn robots_clean_pass() {
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
    assert!(
        !findings.iter().any(|f| f["rule_id"]
            .as_str()
            .unwrap_or("")
            .starts_with("robots-txt/")),
        "Clean robots.txt should produce no findings"
    );
}

// ==========================================================================
// AI Visibility module (#27)
// ==========================================================================

#[test]
fn ai_visibility_disabled_by_default() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"]
            .as_str()
            .unwrap_or("")
            .starts_with("ai-visibility/")),
        "AI visibility checks should not run without aiVisibility:true"
    );
}

#[test]
fn ai_visibility_rich_page_pass() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test Article</title><link rel="canonical" href="https://example.com/"><meta property="og:title" content="Test Article"><meta property="og:description" content="A rich article with good AI visibility signals."><script type="application/ld+json">{"@context":"https://schema.org","@type":"Article","headline":"Test","author":{"@type":"Person","name":"Author"}}</script></head><body><header><nav><a href="/">Home</a></nav></header><main><article><h1>Test Article</h1><h2>Section 1</h2><p>This is a well structured article with enough content to satisfy AI visibility requirements. It has multiple paragraphs and clear semantic structure with headings. The content is substantive and provides real value to readers.</p><h2>Section 2</h2><p>More content here to ensure the word count is above the minimum threshold for AI citation systems to consider this page worth indexing and referencing.</p></article></main><footer><a href="/">Home</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"ai_visibility":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    let ai_errors: Vec<_> = findings
        .iter()
        .filter(|f| {
            f["rule_id"]
                .as_str()
                .unwrap_or("")
                .starts_with("ai-visibility/")
                && f["level"] == "error"
        })
        .collect();
    assert!(
        ai_errors.is_empty(),
        "Rich page should produce no AI visibility errors"
    );
}

// ==========================================================================
// UX Heuristics module (#28)
// ==========================================================================

#[test]
fn ux_disabled_by_default() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, _) = run_audit_json(dir.path(), r#"{"site":{"base_url":"https://example.com"}}"#);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings
            .iter()
            .any(|f| f["rule_id"].as_str().unwrap_or("").starts_with("ux/")),
        "UX checks should not run without uxHeuristics:true"
    );
}

#[test]
fn ux_no_cta_found() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><p>No calls to action here.</p></main><footer><a href="/impressum/">Impressum</a></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"ux_heuristics":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings.iter().any(|f| f["rule_id"] == "ux/no-cta"),
        "Page without CTA should be reported"
    );
}

#[test]
fn ux_no_trust_signals() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><header><nav><a href="/">Home</a></nav></header><main><h1>Test</h1><a href="/products/">Buy now</a></main><footer></footer></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        r#"{"site":{"base_url":"https://example.com"},"ux_heuristics":{"enabled":true}}"#,
    );
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule_id"] == "ux/no-trust-signals"),
        "Page without trust signal links should be reported"
    );
}
