use std::collections::HashMap;

use scraper::Selector;
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.hreflang.check_hreflang {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let norm_cfg = &config.url_normalization;
    let route_by_abs_url: HashMap<String, String> = index
        .pages
        .iter()
        .filter_map(|p| {
            p.absolute_url
                .as_ref()
                .map(|u| (normalize_url_like(u, norm_cfg), p.route.clone()))
        })
        .collect();

    // Collect all hreflang declarations across pages
    // Map: page_route -> Vec<(lang, href)>
    let mut all_hreflangs: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for page in &index.pages {
        let html = page.parse_html();
        let sel = Selector::parse("link[rel='alternate'][hreflang]").unwrap();

        let entries: Vec<(String, String)> = html
            .select(&sel)
            .filter_map(|el| {
                let lang = el.value().attr("hreflang")?.to_string();
                let href = el.value().attr("href")?.to_string();
                Some((lang, href))
            })
            .collect();

        if entries.is_empty() {
            continue;
        }

        // Check x-default presence
        if config.hreflang.require_x_default {
            let has_x_default = entries.iter().any(|(lang, _)| lang == "x-default");
            if !has_x_default {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "hreflang/no-x-default".into(),
                    file: page.rel_path.clone(),
                    selector: "link[rel='alternate'][hreflang]".into(),
                    message: "Hreflang tags present but no x-default".into(),
                    help: "Add <link rel=\"alternate\" hreflang=\"x-default\" href=\"...\">".into(),
                    suggestion: None,
                });
            }
        }

        // Check self-reference
        if config.hreflang.require_self_reference {
            let page_url_norm = page
                .absolute_url
                .as_ref()
                .map(|u| normalize_url_like(u, norm_cfg));
            let has_self = entries.iter().any(|(_, href)| {
                page_url_norm
                    .as_ref()
                    .is_some_and(|page_url| normalize_url_like(href, norm_cfg) == *page_url)
            });
            if !has_self {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "hreflang/no-self-reference".into(),
                    file: page.rel_path.clone(),
                    selector: "link[rel='alternate'][hreflang]".into(),
                    message: "Hreflang tags don't include a self-reference".into(),
                    help: "Include the current page URL in hreflang annotations".into(),
                    suggestion: None,
                });
            }
        }

        all_hreflangs.insert(page.route.clone(), entries);
    }

    // Check reciprocal references
    if config.hreflang.require_reciprocal {
        for (source_route, entries) in &all_hreflangs {
            for (lang, href) in entries {
                if lang == "x-default" {
                    continue;
                }
                // Try to find the target page and check it links back
                let target_route = route_by_abs_url.get(&normalize_url_like(href, norm_cfg));

                if let Some(target_route) = target_route {
                    if let Some(target_entries) = all_hreflangs.get(target_route) {
                        let source_url = index
                            .pages
                            .iter()
                            .find(|p| p.route == *source_route)
                            .and_then(|p| p.absolute_url.as_ref())
                            .map(|u| normalize_url_like(u, norm_cfg));

                        let has_reciprocal = source_url.is_some_and(|source_url| {
                            target_entries
                                .iter()
                                .any(|(_, h)| normalize_url_like(h, norm_cfg) == source_url)
                        });

                        if !has_reciprocal {
                            let source_file = index
                                .pages
                                .iter()
                                .find(|p| p.route == *source_route)
                                .map(|p| p.rel_path.as_str())
                                .unwrap_or("(unknown)");
                            findings.push(Finding {
                                level: Level::Warning,
                                rule_id: "hreflang/no-reciprocal".into(),
                                file: source_file.to_string(),
                                selector: format!("link[hreflang='{}'][href='{}']", lang, href),
                                message: format!(
                                    "Hreflang target '{}' (lang='{}') doesn't link back",
                                    href, lang
                                ),
                                help: "Add reciprocal hreflang link on the target page".into(),
                                suggestion: None,
                            });
                        }
                    }
                }
            }
        }
    }

    findings
}

fn normalize_url_like(url_or_path: &str, norm: &crate::config::UrlNormalizationConfig) -> String {
    if let Ok(parsed) = Url::parse(url_or_path) {
        let mut rebuilt = parsed.clone();
        rebuilt.set_path(&normalize::normalize_path(parsed.path(), norm));
        rebuilt.set_query(None);
        rebuilt.set_fragment(None);
        rebuilt.to_string()
    } else {
        normalize::normalize_path(url_or_path, norm)
    }
}
