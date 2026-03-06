use std::collections::{HashMap, HashSet};

use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.crawl_budget.enabled {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let mut incoming_links: HashMap<String, usize> = HashMap::new();

    for page in &index.pages {
        let mut query_variant_count = 0usize;
        let mut non_canonical_variant_hrefs: HashSet<String> = HashSet::new();

        for href in &page.anchor_hrefs {
            if !normalize::is_internal(href, index.base_url.as_deref()) {
                continue;
            }
            if normalize::has_query_params(href) {
                query_variant_count += 1;
            }

            if let Some(resolved) =
                normalize::resolve_href(href, &page.route, index.base_url.as_deref())
            {
                let normalized = normalize::normalize_path(&resolved, &config.url_normalization);
                *incoming_links.entry(normalized.clone()).or_insert(0) += 1;
                if resolved != normalized {
                    non_canonical_variant_hrefs.insert(href.clone());
                }
            }
        }

        if query_variant_count > 0 {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "crawl-budget/query-variants".into(),
                file: page.rel_path.clone(),
                selector: "a[href*='?']".into(),
                message: format!(
                    "Found {} internal links with query parameters (crawl budget dilution risk)",
                    query_variant_count
                ),
                help: "Use canonical internal URLs without tracking/query params".into(),
                suggestion: None,
            });
        }

        if !non_canonical_variant_hrefs.is_empty() {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "crawl-budget/non-canonical-link-variant".into(),
                file: page.rel_path.clone(),
                selector: "a".into(),
                message: format!(
                    "Found {} internal links using non-canonical URL variants (trailing slash/index.html)",
                    non_canonical_variant_hrefs.len()
                ),
                help: "Link consistently to canonical URL variants only".into(),
                suggestion: None,
            });
        }

        let html = page.parse_html();
        check_meta_refresh_targets(page, &html, index, config, &mut findings);
    }

    let mut canonical_to_pages: HashMap<String, Vec<String>> = HashMap::new();
    for page in &index.pages {
        if let Some(canonical) = &page.canonical {
            canonical_to_pages
                .entry(canonical.clone())
                .or_default()
                .push(page.rel_path.clone());
        }
    }
    for (canonical, pages) in canonical_to_pages {
        if pages.len() > 1 {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "crawl-budget/duplicate-cluster".into(),
                file: pages.first().cloned().unwrap_or_default(),
                selector: "link[rel='canonical']".into(),
                message: format!(
                    "{} pages share canonical '{}', which can waste crawl budget",
                    pages.len(),
                    canonical
                ),
                help: "Consolidate duplicates or ensure only one canonical target page remains indexable"
                    .into(),
                suggestion: None,
            });
        }
    }

    if !index.sitemap_urls.is_empty() {
        for page in &index.pages {
            if !page.noindex {
                continue;
            }
            let Some(abs) = &page.absolute_url else {
                continue;
            };
            if index.sitemap_urls.contains(abs) {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "crawl-budget/noindex-in-sitemap".into(),
                    file: page.rel_path.clone(),
                    selector: "meta[name='robots']".into(),
                    message: "Noindex page appears in sitemap.xml".into(),
                    help: "Remove noindex URLs from sitemap to avoid mixed indexability signals"
                        .into(),
                    suggestion: None,
                });
            }
        }
    }

    for page in &index.pages {
        if page.noindex {
            let incoming = incoming_links.get(&page.route).copied().unwrap_or(0);
            if incoming > 0 {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "crawl-budget/noindex-with-internal-demand".into(),
                    file: page.rel_path.clone(),
                    selector: "meta[name='robots']".into(),
                    message: format!("Noindex page receives {} internal links", incoming),
                    help:
                        "Consider de-linking or changing indexability to keep crawl paths focused"
                            .into(),
                    suggestion: None,
                });
            }
        }
    }

    findings
}

fn check_meta_refresh_targets(
    page: &crate::discovery::PageInfo,
    html: &scraper::Html,
    index: &SiteIndex,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    let sel = Selector::parse("meta[http-equiv='refresh'][content]").unwrap();
    for el in html.select(&sel) {
        let Some(content) = el.value().attr("content") else {
            continue;
        };
        let lower = content.to_lowercase();
        let Some(url_pos) = lower.find("url=") else {
            continue;
        };
        let target = content[(url_pos + 4)..]
            .trim()
            .trim_matches('\'')
            .trim_matches('"');
        if target.is_empty() {
            continue;
        }

        if let Some(resolved) =
            normalize::resolve_href(target, &page.route, index.base_url.as_deref())
        {
            let normalized = normalize::normalize_path(&resolved, &config.url_normalization);
            if !index.route_exists(&normalized) {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "crawl-budget/redirect-target-missing".into(),
                    file: page.rel_path.clone(),
                    selector: "meta[http-equiv='refresh']".into(),
                    message: format!("Meta refresh target '{}' does not exist in dist", target),
                    help: "Point redirects to existing canonical targets".into(),
                    suggestion: None,
                });
            }
        }
    }
}
