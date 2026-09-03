use std::borrow::Cow;
use std::collections::HashMap;

use html_conform::Severity;
use rayon::prelude::*;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Confidence, Finding, Level};

const ASTRO_ISLAND_RUNTIME_STYLE: &str =
    "<style>astro-island,astro-slot,astro-static-slot{display:contents}</style>";

fn html_for_validation(html: &str) -> Cow<'_, str> {
    if !html.contains(ASTRO_ISLAND_RUNTIME_STYLE) {
        return Cow::Borrowed(html);
    }

    // Astro injects this exact style next to its first hydrated island. It is
    // framework runtime output, not author markup. Blanking it at equal byte
    // length keeps all source locations from html-conform stable.
    Cow::Owned(html.replace(
        ASTRO_ISLAND_RUNTIME_STYLE,
        &" ".repeat(ASTRO_ISLAND_RUNTIME_STYLE.len()),
    ))
}

/// Native HTML5 conformance validation via `html-conform` (vnu-comparable,
/// pure Rust, no JVM/subprocess/network). Covers tree-construction errors,
/// RELAX NG content-model schema, ARIA co-constraints, attribute
/// microsyntaxes (srcset, datetime, CSP, lang, ...), import-map/speculation
/// JSON, CSP `meta` enforcement, and table cell-grid integrity.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.html_validation.enabled {
        return Vec::new();
    }

    let max_per_page = config.html_validation.max_per_page.unwrap_or(20);

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let validation_html = html_for_validation(&page.html_content);
            let report = match html_conform::check(&validation_html) {
                Ok(report) => report,
                Err(error) => {
                    return vec![Finding::new(
                        Level::Error,
                        "html/validator-error",
                        page.rel_path.clone(),
                        "",
                        format!("HTML conformance validation failed: {error}"),
                        "The HTML validator could not initialize. Reinstall or update astro-post-audit before trusting this audit result.",
                        Some(Confidence::Medium),
                    )];
                }
            };
            if report.findings.is_empty() {
                return Vec::new();
            }

            // Deduplicate identical (rule, message) pairs while preserving
            // first-seen order.
            let mut order: Vec<(String, String, Level, Option<html_conform::SourceLocation>)> =
                Vec::new();
            let mut counts: HashMap<(String, String), usize> = HashMap::new();
            for finding in &report.findings {
                let key = (finding.rule_id.clone(), finding.message.clone());
                if !counts.contains_key(&key) {
                    order.push((
                        finding.rule_id.clone(),
                        finding.message.clone(),
                        match finding.severity {
                            Severity::Error => Level::Error,
                            Severity::Warning => Level::Warning,
                            Severity::Info => Level::Info,
                        },
                        finding.location,
                    ));
                }
                *counts.entry(key).or_insert(0) += 1;
            }

            order
                .into_iter()
                .take(max_per_page)
                .map(|(rule_id, message, level, location)| {
                    let count = counts
                        .get(&(rule_id.clone(), message.clone()))
                        .copied()
                        .unwrap_or(1);
                    let occurrences = if count > 1 {
                        format!(" ({count} occurrences)")
                    } else {
                        String::new()
                    };
                    let location = location
                        .map(|location| format!(" at line {location}"))
                        .unwrap_or_default();
                    Finding {
                        level,
                        rule_id: format!("html/{rule_id}"),
                        file: page.rel_path.clone(),
                        selector: String::new(),
                        message: format!(
                            "HTML conformance{location}: {message}{occurrences}"
                        ),
                        help: "Fix the markup issue reported by conformance validation (tree construction, content-model schema, ARIA constraints, or attribute microsyntax). Browsers often recover silently, but it can break hydration, accessibility, or interoperability.".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: Some(Confidence::Medium),
                    }
                })
                .collect()
        })
        .collect()
}
