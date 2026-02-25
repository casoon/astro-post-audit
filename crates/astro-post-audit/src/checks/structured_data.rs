use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.structured_data.check_json_ld && !config.structured_data.require_json_ld {
        return Vec::new();
    }

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            let sel = Selector::parse("script[type='application/ld+json']").unwrap();
            let scripts: Vec<_> = html.select(&sel).collect();

            if scripts.is_empty() {
                if config.structured_data.require_json_ld {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "structured-data/missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "No JSON-LD structured data found".into(),
                        help: "Add <script type=\"application/ld+json\"> with schema.org data"
                            .into(),
                    });
                }
                return findings;
            }

            // Validate JSON syntax of each JSON-LD block
            if config.structured_data.check_json_ld {
                for (i, script) in scripts.iter().enumerate() {
                    let content: String = script.text().collect();
                    let trimmed = content.trim();
                    if trimmed.is_empty() {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "structured-data/empty".into(),
                            file: page.rel_path.clone(),
                            selector: format!("script[type='application/ld+json']:nth({})", i + 1),
                            message: "JSON-LD script is empty".into(),
                            help: "Add valid JSON-LD content or remove the empty script tag".into(),
                        });
                        continue;
                    }

                    if let Err(e) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "structured-data/invalid-json".into(),
                            file: page.rel_path.clone(),
                            selector: format!("script[type='application/ld+json']:nth({})", i + 1),
                            message: format!("Invalid JSON in JSON-LD: {}", e),
                            help: "Fix the JSON syntax in the structured data block".into(),
                        });
                    }
                }
            }

            findings
        })
        .collect()
}
