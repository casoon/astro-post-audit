use std::borrow::Cow;
use std::collections::HashMap;

use html_conform::Severity as ConformSeverity;
use rayon::prelude::*;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};

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
                    return vec![Finding::review("html/validator-error", format!("HTML conformance validation failed: {error}"))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()))
.with_help("The HTML validator could not initialize. Reinstall or update astro-post-audit before trusting this audit result.")];
                }
            };
            if report.findings.is_empty() {
                return Vec::new();
            }

            // Deduplicate identical (rule, message) pairs while preserving
            // first-seen order.
            let mut order: Vec<(
                String,
                String,
                Severity,
                Option<html_conform::SourceLocation>,
            )> = Vec::new();
            let mut counts: HashMap<(String, String), usize> = HashMap::new();
            for finding in &report.findings {
                let key = (finding.rule_id.clone(), finding.message.clone());
                if !counts.contains_key(&key) {
                    order.push((
                        finding.rule_id.clone(),
                        finding.message.clone(),
                        match finding.severity {
                            ConformSeverity::Error => Severity::High,
                            ConformSeverity::Warning => Severity::Medium,
                            ConformSeverity::Info => Severity::Low,
                        },
                        finding.location,
                    ));
                }
                *counts.entry(key).or_insert(0) += 1;
            }

            order
                .into_iter()
                .take(max_per_page)
                .map(|(rule_id, message, schwere, location)| {
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
                    Finding::review(
                        format!("html/{rule_id}"),
                        format!("HTML conformance{location}: {message}{occurrences}"),
                    )
                    .with_severity(schwere)
                    .at(Location::file(page.rel_path.clone()))
                    .with_help("Fix the markup issue reported by conformance validation (tree construction, content-model schema, ARIA constraints, or attribute microsyntax). Browsers often recover silently, but it can break hydration, accessibility, or interoperability.")
                })
                .collect()
        })
        .collect()
}
