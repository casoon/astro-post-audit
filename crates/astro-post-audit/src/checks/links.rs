use rayon::prelude::*;
use scraper::Selector;
use std::collections::HashSet;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();

    if config.links.check_internal {
        findings.extend(check_internal_links(index, config));
    }

    if config.links.detect_orphan_pages {
        findings.extend(check_orphan_pages(index, config));
    }

    findings
}

fn check_internal_links(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            let sel = match Selector::parse("a[href]") {
                Ok(s) => s,
                Err(_) => return findings,
            };

            // Collect all IDs on this page for fragment checks
            let page_ids: HashSet<String> = if config.links.check_fragments {
                let id_sel = Selector::parse("[id]").unwrap();
                html.select(&id_sel)
                    .filter_map(|el| el.value().attr("id").map(|s| s.to_string()))
                    .collect()
            } else {
                HashSet::new()
            };

            for element in html.select(&sel) {
                let href = match element.value().attr("href") {
                    Some(h) => h,
                    None => continue,
                };

                // Skip non-internal links
                if !normalize::is_internal(href, index.base_url.as_deref()) {
                    continue;
                }

                // Skip mailto, tel, javascript links
                if href.starts_with("mailto:")
                    || href.starts_with("tel:")
                    || href.starts_with("javascript:")
                {
                    continue;
                }

                // Fragment-only links: check if target ID exists on same page
                if let Some(fragment) = href.strip_prefix('#') {
                    if config.links.check_fragments
                        && !fragment.is_empty()
                        && !page_ids.contains(fragment)
                    {
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "links/broken-fragment".into(),
                            file: page.rel_path.clone(),
                            selector: format!("a[href='{}']", href),
                            message: format!(
                                "Fragment target '{}' not found on this page",
                                fragment
                            ),
                            help: "Add an element with the matching id, or fix the fragment"
                                .into(),
                        });
                    }
                    continue;
                }

                // Check query params
                if config.links.forbid_query_params_internal && normalize::has_query_params(href) {
                    findings.push(Finding {
                        level: Level::Error,
                        rule_id: "links/query-params".into(),
                        file: page.rel_path.clone(),
                        selector: format!("a[href='{}']", href),
                        message: format!(
                            "Internal link contains query parameters: '{}'",
                            href
                        ),
                        help: "Remove query parameters from internal links to avoid duplicate content signals".into(),
                    });
                }

                // Check mixed content: absolute http:// internal links
                if config.links.check_mixed_content && href.starts_with("http://") {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "links/mixed-content".into(),
                        file: page.rel_path.clone(),
                        selector: format!("a[href='{}']", href),
                        message: format!("Internal link uses HTTP instead of HTTPS: '{}'", href),
                        help: "Use HTTPS for all internal links".into(),
                    });
                }

                // Resolve and check if target exists
                if let Some(resolved) =
                    normalize::resolve_href(href, &page.route, index.base_url.as_deref())
                {
                    let normalized =
                        normalize::normalize_path(&resolved, &config.url_normalization);

                    // Check if route exists or if it's a file asset
                    if !index.route_exists(&normalized) {
                        // Also check raw path as file
                        let file_check = resolved.trim_start_matches('/');
                        if !index.file_exists(file_check) {
                            let level = if config.links.fail_on_broken {
                                Level::Error
                            } else {
                                Level::Warning
                            };
                            findings.push(Finding {
                                level,
                                rule_id: "links/broken".into(),
                                file: page.rel_path.clone(),
                                selector: format!("a[href='{}']", href),
                                message: format!(
                                    "Broken internal link '{}' -> '{}' (not found in dist)",
                                    href, normalized
                                ),
                                help: "Fix the href to point to an existing page".into(),
                            });
                        }
                    }

                    // Check fragment on cross-page links
                    if config.links.check_fragments {
                        if let Some(fragment) = href.split('#').nth(1) {
                            if !fragment.is_empty() {
                                // Find the target page and check for the ID
                                if let Some(&target_idx) = index.route_to_index.get(&normalized) {
                                    let target_page = &index.pages[target_idx];
                                    let target_html = target_page.parse_html();
                                    let id_selector_str = format!("[id='{}']", fragment);
                                    let id_sel = Selector::parse(&id_selector_str);
                                    if let Ok(sel) = id_sel {
                                        if target_html.select(&sel).next().is_none() {
                                            findings.push(Finding {
                                                level: Level::Warning,
                                                rule_id: "links/broken-fragment".into(),
                                                file: page.rel_path.clone(),
                                                selector: format!("a[href='{}']", href),
                                                message: format!(
                                                    "Fragment '{}' not found on target page '{}'",
                                                    fragment, normalized
                                                ),
                                                help: "Fix the fragment or add the target id".into(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            findings
        })
        .collect()
}

fn check_orphan_pages(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    // Collect all routes that are linked to from any page (parallel)
    let per_page_routes: Vec<HashSet<String>> = index
        .pages
        .par_iter()
        .map(|page| {
            let html = page.parse_html();
            let sel = Selector::parse("a[href]").unwrap();
            let mut routes = HashSet::new();

            for el in html.select(&sel) {
                if let Some(href) = el.value().attr("href") {
                    if normalize::is_internal(href, index.base_url.as_deref())
                        && !href.starts_with('#')
                        && !href.starts_with("mailto:")
                        && !href.starts_with("tel:")
                    {
                        if let Some(resolved) =
                            normalize::resolve_href(href, &page.route, index.base_url.as_deref())
                        {
                            let normalized =
                                normalize::normalize_path(&resolved, &config.url_normalization);
                            routes.insert(normalized);
                        }
                    }
                }
            }
            routes
        })
        .collect();

    let mut linked_routes: HashSet<String> = HashSet::new();
    linked_routes.insert("/".to_string()); // Root is never orphan
    for routes in per_page_routes {
        linked_routes.extend(routes);
    }

    // Find pages that are never linked to
    index
        .pages
        .iter()
        .filter(|page| !linked_routes.contains(&page.route))
        .map(|page| Finding {
            level: Level::Warning,
            rule_id: "links/orphan-page".into(),
            file: page.rel_path.clone(),
            selector: String::new(),
            message: format!(
                "Orphan page '{}' is not linked from any other page",
                page.route
            ),
            help: "Add internal links to this page or remove it if unneeded".into(),
        })
        .collect()
}
