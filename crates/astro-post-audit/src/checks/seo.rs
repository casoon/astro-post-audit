use rayon::prelude::*;
use scraper::{Html, Selector};
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            // Canonical checks
            if config.canonical.require {
                check_canonical(page, &html, index, config, &mut findings);
            }

            // Robots meta checks
            check_robots(page, config, &mut findings);

            findings
        })
        .collect()
}

fn check_canonical(
    page: &crate::discovery::PageInfo,
    html: &Html,
    index: &SiteIndex,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    let sel = match Selector::parse("link[rel='canonical']") {
        Ok(s) => s,
        Err(_) => return,
    };

    let canonicals: Vec<_> = html.select(&sel).collect();

    if canonicals.is_empty() {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "canonical/missing".into(),
            file: page.rel_path.clone(),
            selector: "head".into(),
            message: "Missing canonical tag".into(),
            help: "Add <link rel=\"canonical\" href=\"...\"> to <head>".into(),
        });
        return;
    }

    if canonicals.len() > 1 {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "canonical/multiple".into(),
            file: page.rel_path.clone(),
            selector: "link[rel='canonical']".into(),
            message: format!(
                "Found {} canonical tags (expected exactly 1)",
                canonicals.len()
            ),
            help: "Remove duplicate canonical tags, keep only one".into(),
        });
    }

    let href = canonicals[0].value().attr("href").unwrap_or("");
    if href.trim().is_empty() {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "canonical/empty".into(),
            file: page.rel_path.clone(),
            selector: "link[rel='canonical']".into(),
            message: "Canonical tag has empty href".into(),
            help: "Set the href to the canonical URL of this page".into(),
        });
        return;
    }

    // Check if absolute
    if config.canonical.absolute && Url::parse(href).is_err() {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "canonical/not-absolute".into(),
            file: page.rel_path.clone(),
            selector: format!("link[rel='canonical'][href='{}']", href),
            message: "Canonical URL is not absolute".into(),
            help: "Use a full URL including protocol and domain".into(),
        });
        return;
    }

    // Check same origin
    if config.canonical.same_origin {
        if let Some(ref base) = index.base_url {
            if let (Ok(base_parsed), Ok(href_parsed)) = (Url::parse(base), Url::parse(href)) {
                if href_parsed.origin() != base_parsed.origin() {
                    findings.push(Finding {
                        level: Level::Error,
                        rule_id: "canonical/cross-origin".into(),
                        file: page.rel_path.clone(),
                        selector: format!("link[rel='canonical'][href='{}']", href),
                        message: format!(
                            "Canonical URL points to different origin '{}' (expected '{}')",
                            href_parsed.origin().ascii_serialization(),
                            base_parsed.origin().ascii_serialization()
                        ),
                        help: "Canonical should point to the same origin as --site".into(),
                    });
                }
            }
        }
    }

    // Check self-reference
    if config.canonical.self_reference {
        if let Some(ref page_url) = page.absolute_url {
            let normalized_canonical = normalize::normalize_path(href, &config.url_normalization);
            let normalized_page = normalize::normalize_path(page_url, &config.url_normalization);
            if normalized_canonical != normalized_page {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "canonical/not-self".into(),
                    file: page.rel_path.clone(),
                    selector: format!("link[rel='canonical'][href='{}']", href),
                    message: format!(
                        "Canonical URL '{}' does not match page URL '{}'",
                        href, page_url
                    ),
                    help: "If this page should self-canonicalize, update the canonical href".into(),
                });
            }
        }
    }

    // Check canonical target exists in dist
    if let Ok(parsed) = Url::parse(href) {
        let target_path = normalize::normalize_path(parsed.path(), &config.url_normalization);
        if !index.route_exists(&target_path) {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "canonical/target-missing".into(),
                file: page.rel_path.clone(),
                selector: format!("link[rel='canonical'][href='{}']", href),
                message: format!(
                    "Canonical URL '{}' target route '{}' not found in dist",
                    href, target_path
                ),
                help: "Ensure the canonical URL points to an existing page".into(),
            });
        }
    }
}

fn check_robots(page: &crate::discovery::PageInfo, config: &Config, findings: &mut Vec<Finding>) {
    if page.noindex {
        if config.robots_meta.fail_if_noindex {
            findings.push(Finding {
                level: Level::Error,
                rule_id: "robots/noindex".into(),
                file: page.rel_path.clone(),
                selector: "meta[name='robots']".into(),
                message: "Page has noindex directive".into(),
                help: "Remove noindex if this page should be indexed".into(),
            });
        } else if !config.robots_meta.allow_noindex {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "robots/noindex".into(),
                file: page.rel_path.clone(),
                selector: "meta[name='robots']".into(),
                message: "Page has noindex directive".into(),
                help: "Remove noindex if this page should be indexed".into(),
            });
        }
    }
}
