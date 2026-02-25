use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            // target="_blank" without rel="noopener"
            if config.security.check_target_blank {
                let sel = Selector::parse("a[target='_blank']").unwrap();
                for el in html.select(&sel) {
                    let rel = el.value().attr("rel").unwrap_or("");
                    if !rel.contains("noopener") && !rel.contains("noreferrer") {
                        let href = el.value().attr("href").unwrap_or("(no href)");
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "security/target-blank-noopener".into(),
                            file: page.rel_path.clone(),
                            selector: format!("a[href='{}'][target='_blank']", href),
                            message: format!(
                                "Link with target=\"_blank\" missing rel=\"noopener\": '{}'",
                                href
                            ),
                            help: "Add rel=\"noopener noreferrer\" to external links with target=\"_blank\"".into(),
                        });
                    }
                }
            }

            // Mixed content: http:// resources on (presumably) https page
            if config.security.check_mixed_content {
                check_mixed_content(page, &html, &mut findings);
            }

            // Inline scripts
            if config.security.warn_inline_scripts {
                let sel = Selector::parse("script:not([src]):not([type='application/ld+json']):not([type='application/json'])").unwrap();
                let inline_count = html.select(&sel).count();
                if inline_count > 0 {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "security/inline-scripts".into(),
                        file: page.rel_path.clone(),
                        selector: "script".into(),
                        message: format!(
                            "Found {} inline script(s) - may conflict with CSP",
                            inline_count
                        ),
                        help: "Move inline scripts to external files for better CSP compatibility"
                            .into(),
                    });
                }
            }

            findings
        })
        .collect()
}

fn check_mixed_content(
    page: &crate::discovery::PageInfo,
    html: &scraper::Html,
    findings: &mut Vec<Finding>,
) {
    let selectors = [
        ("img[src]", "src"),
        ("script[src]", "src"),
        ("link[href]", "href"),
        ("source[src]", "src"),
        ("video[src]", "src"),
        ("audio[src]", "src"),
        ("iframe[src]", "src"),
    ];

    for (selector_str, attr) in &selectors {
        let sel = match Selector::parse(selector_str) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for el in html.select(&sel) {
            if let Some(value) = el.value().attr(attr) {
                if value.starts_with("http://") {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "security/mixed-content".into(),
                        file: page.rel_path.clone(),
                        selector: format!("{}='{}'", selector_str, value),
                        message: format!("HTTP resource on potentially HTTPS page: '{}'", value),
                        help: "Use HTTPS URLs or protocol-relative URLs for all resources".into(),
                    });
                }
            }
        }
    }
}
