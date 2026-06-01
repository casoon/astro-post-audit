use std::collections::HashMap;

use rayon::prelude::*;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Confidence, Finding, Level};

/// Native HTML5 syntax validation. Reuses the parse errors that the html5ever
/// tokenizer/tree-builder (via `scraper`) collects through its `parse_error`
/// callbacks — fully local and offline, no external validator required.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.html_validation.enabled {
        return Vec::new();
    }

    let max_per_page = config.html_validation.max_per_page.unwrap_or(20);

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let html = page.parse_html();
            if html.errors.is_empty() {
                return Vec::new();
            }

            // Deduplicate identical messages while preserving first-seen order.
            let mut order: Vec<String> = Vec::new();
            let mut counts: HashMap<String, usize> = HashMap::new();
            for err in &html.errors {
                let msg = err.to_string();
                if !counts.contains_key(&msg) {
                    order.push(msg.clone());
                }
                *counts.entry(msg).or_insert(0) += 1;
            }

            order
                .into_iter()
                .take(max_per_page)
                .map(|msg| {
                    let count = counts[&msg];
                    let occurrences = if count > 1 {
                        format!(" ({count} occurrences)")
                    } else {
                        String::new()
                    };
                    Finding {
                        level: Level::Warning,
                        rule_id: "html/syntax-error".into(),
                        file: page.rel_path.clone(),
                        selector: String::new(),
                        message: format!("HTML5 syntax error: {msg}{occurrences}"),
                        help: "Fix the malformed markup (unclosed tags, invalid nesting, or stray characters). Browsers recover silently, but it can break hydration and accessibility.".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: Some(Confidence::Medium),
                    }
                })
                .collect()
        })
        .collect()
}
