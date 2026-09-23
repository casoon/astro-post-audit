use std::collections::HashMap;

use rayon::prelude::*;
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Location, Severity};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let mut findings: Vec<Finding> = index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();

            // Canonical checks
            if config.canonical.require {
                check_canonical(page, index, config, &mut findings);
            }

            // Robots meta checks
            check_robots(page, config, &mut findings);

            findings
        })
        .collect();

    // Cross-page: canonical cluster detection
    if config.canonical.detect_clusters {
        findings.extend(check_canonical_clusters(index));
    }

    findings
}

fn check_canonical(
    page: &crate::discovery::PageInfo,
    index: &SiteIndex,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    let canonicals = &page.canonical_hrefs;

    if canonicals.is_empty() {
        findings.push(Finding::fail("canonical/missing","Missing canonical tag")
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Set `site` in astro.config.mjs and render <link rel=\"canonical\" href={new URL(Astro.url.pathname, Astro.site)} /> in your BaseHead component")
.with_suggestion("<link rel=\"canonical\" href=\"https://...\">"));
        return;
    }

    if canonicals.len() > 1 {
        findings.push(
            Finding::fail(
                "canonical/multiple",
                format!(
                    "Found {} canonical tags (expected exactly 1)",
                    canonicals.len()
                ),
            )
            .with_severity(Severity::High)
            .at(Location::file(page.rel_path.clone()).with_selector("link[rel='canonical']"))
            .with_help("Remove duplicate canonical tags, keep only one"),
        );
    }

    let href = canonicals[0].as_str();
    if href.trim().is_empty() {
        findings.push(
            Finding::fail("canonical/empty", "Canonical tag has empty href")
                .with_severity(Severity::High)
                .at(Location::file(page.rel_path.clone()).with_selector("link[rel='canonical']"))
                .with_help("Set the href to the canonical URL of this page"),
        );
        return;
    }

    // Check if absolute
    if config.canonical.absolute && Url::parse(href).is_err() {
        findings.push(
            Finding::fail("canonical/not-absolute", "Canonical URL is not absolute")
                .with_severity(Severity::High)
                .at(Location::file(page.rel_path.clone())
                    .with_selector(format!("link[rel='canonical'][href='{}']", href)))
                .with_help("Use a full URL including protocol and domain"),
        );
        return;
    }

    // Check same origin
    if config.canonical.same_origin {
        if let Some(ref base) = index.base_url {
            if let (Ok(base_parsed), Ok(href_parsed)) = (Url::parse(base), Url::parse(href)) {
                if href_parsed.origin() != base_parsed.origin() {
                    findings.push(
                        Finding::fail(
                            "canonical/cross-origin",
                            format!(
                                "Canonical URL points to different origin '{}' (expected '{}')",
                                href_parsed.origin().ascii_serialization(),
                                base_parsed.origin().ascii_serialization()
                            ),
                        )
                        .with_severity(Severity::High)
                        .at(Location::file(page.rel_path.clone())
                            .with_selector(format!("link[rel='canonical'][href='{}']", href)))
                        .with_help("Canonical should point to the same origin as --site"),
                    );
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
                findings.push(
                    Finding::fail(
                        "canonical/not-self",
                        format!(
                            "Canonical URL '{}' does not match page URL '{}'",
                            href, page_url
                        ),
                    )
                    .with_severity(Severity::Medium)
                    .at(Location::file(page.rel_path.clone())
                        .with_selector(format!("link[rel='canonical'][href='{}']", href)))
                    .with_help("If this page should self-canonicalize, update the canonical href"),
                );
            }
        }
    }

    // Check canonical target exists in dist
    if let Ok(parsed) = Url::parse(href) {
        let target_path = normalize::normalize_path(parsed.path(), &config.url_normalization);
        if !index.route_exists(&target_path) {
            findings.push(
                Finding::fail(
                    "canonical/target-missing",
                    format!(
                        "Canonical URL '{}' target route '{}' not found in dist",
                        href, target_path
                    ),
                )
                .with_severity(Severity::Medium)
                .at(Location::file(page.rel_path.clone())
                    .with_selector(format!("link[rel='canonical'][href='{}']", href)))
                .with_help("Ensure the canonical URL points to an existing page"),
            );
        }
    }
}

fn check_robots(page: &crate::discovery::PageInfo, config: &Config, findings: &mut Vec<Finding>) {
    if page.noindex {
        if config.robots_meta.fail_if_noindex {
            findings.push(
                Finding::fail("robots/noindex", "Page has noindex directive")
                    .with_severity(Severity::High)
                    .at(Location::file(page.rel_path.clone()).with_selector("meta[name='robots']"))
                    .with_help("Remove noindex if this page should be indexed"),
            );
        } else if !config.robots_meta.allow_noindex {
            findings.push(
                Finding::fail("robots/noindex", "Page has noindex directive")
                    .with_severity(Severity::Medium)
                    .at(Location::file(page.rel_path.clone()).with_selector("meta[name='robots']"))
                    .with_help("Remove noindex if this page should be indexed"),
            );
        }
    }
}

/// Detect canonical clusters: multiple pages pointing to the same canonical URL.
/// This is often a copy-paste error, but can be intentional (AMP, hreflang variants).
fn check_canonical_clusters(index: &SiteIndex) -> Vec<Finding> {
    let mut canonical_to_pages: HashMap<&str, Vec<&str>> = HashMap::new();

    for page in &index.pages {
        if let Some(ref canonical) = page.canonical {
            canonical_to_pages
                .entry(canonical.as_str())
                .or_default()
                .push(&page.rel_path);
        }
    }

    let mut findings = Vec::new();
    for (canonical, pages) in &canonical_to_pages {
        if pages.len() > 1 {
            for page in pages {
                let others: Vec<&str> = pages.iter().filter(|p| *p != page).copied().collect();
                let others_display = if others.len() <= 3 {
                    others.join(", ")
                } else {
                    format!("{} and {} more", others[..3].join(", "), others.len() - 3)
                };
                findings.push(Finding::fail("canonical/cluster", format!(
                        "{} pages share canonical URL '{}' (also: {})",
                        pages.len(),
                        canonical,
                        others_display
                    ))
.with_severity(Severity::Medium)
.at(Location::file(page.to_string()).with_selector(format!("link[rel='canonical'][href='{}']", canonical)))
.with_help("Multiple pages pointing to the same canonical may indicate a copy-paste error. If intentional (AMP, variants), disable with detect_clusters: false"));
            }
        }
    }

    findings
}
