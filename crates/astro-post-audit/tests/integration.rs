//! Integration tests for all check modules.
//!
//! Strategy: use tempfile to create dist directories with specific HTML content,
//! build a SiteIndex, run checks, and assert on the findings.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

// We import from the binary crate's library modules.
// Since astro-post-audit is a bin crate, integration tests need to test via CLI
// or we restructure. Instead we'll test via CLI invocations.

/// Helper: run the binary against a given dist directory with optional extra args.
fn run_audit(dist_path: &Path, args: &[&str]) -> (String, String, i32) {
    let bin = env!("CARGO_BIN_EXE_astro-post-audit");
    let mut cmd = std::process::Command::new(bin);
    cmd.arg(dist_path.to_str().unwrap());
    for arg in args {
        cmd.arg(arg);
    }
    // Force no color for deterministic output
    cmd.env("NO_COLOR", "1");
    let output = cmd.output().expect("failed to execute binary");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(2);
    (stdout, stderr, code)
}

/// Helper: run the binary and return JSON output parsed.
fn run_audit_json(dist_path: &Path, args: &[&str]) -> (serde_json::Value, i32) {
    let mut full_args = vec!["--format", "json"];
    full_args.extend_from_slice(args);
    let (stdout, _stderr, code) = run_audit(dist_path, &full_args);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "Failed to parse JSON output: {}\nOutput was:\n{}",
            e, stdout
        );
    });
    (json, code)
}

/// Create a minimal valid page in a temp dir.
fn write_valid_page(dir: &Path, rel_path: &str, title: &str, h1: &str, canonical_path: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        &full,
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="canonical" href="https://example.com{canonical_path}">
</head>
<body>
  <h1>{h1}</h1>
</body>
</html>"#
        ),
    )
    .unwrap();
}

// ==========================================================================
// Good fixtures: zero findings under default config
// ==========================================================================

#[test]
fn good_fixtures_pass_clean() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/good");
    let (json, code) = run_audit_json(&fixture_path, &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(&fixture_path, &[]);
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
    let (json, code) = run_audit_json(dir.path(), &[]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &[]);
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
    let (json, code) = run_audit_json(dir.path(), &[]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
fn a11y_aria_hidden_focusable() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1><button aria-hidden="true">Bad</button><span aria-hidden="true">OK</span><div tabindex="0" aria-hidden="true">Bad div</div><div tabindex="-1" aria-hidden="true">OK div</div></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    // For now just test with --check-security (which doesn't toggle inline scripts)
    let (json, _) = run_audit_json(
        dir.path(),
        &["--site", "https://example.com", "--check-security"],
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
        &["--site", "https://example.com", "--check-structured-data"],
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
        &["--site", "https://example.com", "--check-structured-data"],
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
        &["--site", "https://example.com", "--check-structured-data"],
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
        &["--site", "https://example.com", "--check-duplicates"],
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
        &["--site", "https://example.com", "--check-duplicates"],
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
        &["--site", "https://example.com", "--check-assets"],
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
        &["--site", "https://example.com", "--check-assets"],
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
        &["--site", "https://example.com", "--check-assets"],
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
    // Create a config that requires robots.txt
    let config_path = dir.path().join("rules.toml");
    fs::write(&config_path, "[robots_txt]\nrequire = true\n").unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        &[
            "--site",
            "https://example.com",
            "--config",
            config_path.to_str().unwrap(),
        ],
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
    let config_path = dir.path().join("rules.toml");
    fs::write(
        &config_path,
        "[robots_txt]\nrequire = true\nrequire_sitemap_link = true\n",
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        &[
            "--site",
            "https://example.com",
            "--config",
            config_path.to_str().unwrap(),
        ],
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
    let config_path = dir.path().join("rules.toml");
    fs::write(
        &config_path,
        "[robots_txt]\nrequire = true\nrequire_sitemap_link = true\n",
    )
    .unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        &[
            "--site",
            "https://example.com",
            "--config",
            config_path.to_str().unwrap(),
        ],
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
    let (json, code) = run_audit_json(dir.path(), &[]);
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
    let (_json, code) = run_audit_json(dir.path(), &[]);
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
    let (_json, _code) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
}

#[test]
fn edge_case_no_doctype() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("index.html"),
        r#"<html><head><title>No DOCTYPE</title></head><body><h1>Test</h1></body></html>"#,
    )
    .unwrap();
    let (json, code) = run_audit_json(dir.path(), &[]);
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
    // Without --strict: exit 0 (only warnings)
    let (_, _, code_normal) = run_audit(
        dir.path(),
        &["--site", "https://example.com", "--format", "json"],
    );
    assert_eq!(code_normal, 0, "Warnings should exit 0 without --strict");

    // With --strict: exit 1
    let (_, _, code_strict) = run_audit(
        dir.path(),
        &[
            "--site",
            "https://example.com",
            "--strict",
            "--format",
            "json",
        ],
    );
    assert_eq!(code_strict, 1, "Warnings should exit 1 with --strict");
}

// ==========================================================================
// CLI: --format json produces valid JSON
// ==========================================================================

#[test]
fn json_output_structure() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
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
    let (_, stderr, code) = run_audit(Path::new("/nonexistent/dist/path"), &[]);
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
        &["--site", "https://example.com", "--exclude", "bad.html"],
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
        &["--site", "https://example.com", "--include", "index.html"],
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
    let config_path = dir.path().join("rules.toml");
    fs::write(&config_path, "[sitemap]\nrequire = true\n").unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        &[
            "--site",
            "https://example.com",
            "--config",
            config_path.to_str().unwrap(),
        ],
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
    let (json, _) = run_audit_json(dir.path(), &["--site", "https://example.com"]);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule_id"] == "sitemap/entry-not-in-dist"));
}

#[test]
fn no_sitemap_check_flag() {
    let dir = TempDir::new().unwrap();
    write_valid_page(dir.path(), "index.html", "Home", "Home", "/");
    let config_path = dir.path().join("rules.toml");
    fs::write(&config_path, "[sitemap]\nrequire = true\n").unwrap();
    let (json, code) = run_audit_json(
        dir.path(),
        &[
            "--site",
            "https://example.com",
            "--config",
            config_path.to_str().unwrap(),
            "--no-sitemap-check",
        ],
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
    // Create a page with many errors
    fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head></head><body><img src="/a.jpg"><img src="/b.jpg"><a href="/x/"></a><button></button><input type="text" name="q"></body></html>"#,
    ).unwrap();
    let (json, _) = run_audit_json(dir.path(), &["--max-errors", "2"]);
    let findings = json["findings"].as_array().unwrap();
    // With --max-errors=2, total findings should be capped at 2
    assert!(
        findings.len() <= 2,
        "Should cap at 2 findings, got {}",
        findings.len()
    );
}

// ==========================================================================
// Config file auto-discovery
// ==========================================================================

#[test]
fn config_auto_discovery_in_dist_parent() {
    // Create a structure like: project/dist/index.html + project/rules.toml
    let project_dir = TempDir::new().unwrap();
    let dist = project_dir.path().join("dist");
    fs::create_dir_all(&dist).unwrap();
    // Page with missing lang - normally an error
    fs::write(
        dist.join("index.html"),
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Test</title><link rel="canonical" href="https://example.com/"></head><body><h1>Test</h1></body></html>"#,
    ).unwrap();
    // Config in project root disables lang check
    fs::write(
        project_dir.path().join("rules.toml"),
        "[html_basics]\nlang_attr_required = false\n",
    )
    .unwrap();
    // Run against dist/ without --config
    let (json, _) = run_audit_json(&dist, &["--site", "https://example.com"]);
    let findings = json["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["rule_id"] == "html/lang-missing"),
        "Auto-discovered config should disable lang check"
    );
}

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
    let config_path = dir.path().join("rules.toml");
    fs::write(&config_path, "[html_basics]\nlang_attr_required = false\n").unwrap();
    let (json, _) = run_audit_json(
        dir.path(),
        &[
            "--site",
            "https://example.com",
            "--config",
            config_path.to_str().unwrap(),
        ],
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
    let (stdout, _, code) = run_audit(dir.path(), &["--site", "https://example.com"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("All checks passed") || stdout.contains("Summary"));
}

#[test]
fn text_output_with_errors() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("index.html"), "").unwrap();
    let (stdout, _, code) = run_audit(dir.path(), &[]);
    assert_eq!(code, 1);
    assert!(stdout.contains("Summary"));
    assert!(stdout.contains("error"));
}

// ==========================================================================
// JSON snapshot test: verify structure of findings
// ==========================================================================

#[test]
fn json_finding_structure() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("index.html"), "").unwrap();
    let (json, _) = run_audit_json(dir.path(), &[]);
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
