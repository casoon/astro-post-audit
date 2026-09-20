use url::Url;

use crate::checks::links::build_known_routes_set;
use crate::config::{Config, UrlNormalizationConfig};
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Location, Severity};

/// Normalize a full URL using the configured normalization rules.
/// Returns the URL with its path normalized (trailing slash, index.html handling).
fn normalize_url(url_str: &str, norm_config: &UrlNormalizationConfig) -> String {
    if let Ok(parsed) = Url::parse(url_str) {
        let norm_path = normalize::normalize_path(parsed.path(), norm_config);
        let mut rebuilt = parsed.clone();
        rebuilt.set_path(&norm_path);
        rebuilt.to_string()
    } else {
        url_str.to_string()
    }
}

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Check if sitemap exists
    let sitemap_path = index.dist_path.join("sitemap.xml");
    if !sitemap_path.exists() {
        if config.sitemap.require {
            findings.push(Finding::fail("sitemap/missing","sitemap.xml not found in dist directory")
.with_severity(Severity::High)
.at(Location::file("sitemap.xml"))
.with_help("Add a sitemap integration to astro.config.mjs (e.g. `@casoon/astro-sitemap` or `@astrojs/sitemap`) and ensure `site` is set"));
        }
        return findings;
    }

    if let Some(parse_error) = &index.sitemap_parse_error {
        findings.push(
            Finding::fail(
                "sitemap/parse-error",
                format!("Could not parse sitemap.xml: {}", parse_error),
            )
            .with_severity(if config.sitemap.require {
                Severity::High
            } else {
                Severity::Medium
            })
            .at(Location::file("sitemap.xml"))
            .with_help("Fix sitemap.xml syntax and regenerate the file"),
        );
    }

    if index.sitemap_urls.is_empty() {
        return findings;
    }

    let norm = &config.url_normalization;

    // Build normalized sitemap URL set (used by multiple checks)
    let normalized_sitemap: std::collections::HashSet<String> = index
        .sitemap_urls
        .iter()
        .map(|u| normalize_url(u, norm))
        .collect();

    // Check: canonical URLs should be in sitemap
    if config.sitemap.canonical_must_be_in_sitemap {
        for page in &index.pages {
            if page.noindex {
                continue; // noindex pages shouldn't be in sitemap
            }
            if let Some(ref canonical) = page.canonical {
                let norm_canonical = normalize_url(canonical, norm);

                if !normalized_sitemap.contains(&norm_canonical) {
                    findings.push(
                        Finding::fail(
                            "sitemap/canonical-missing",
                            format!("Canonical URL '{}' is not listed in sitemap.xml", canonical),
                        )
                        .with_severity(Severity::Medium)
                        .at(Location::file(page.rel_path.clone())
                            .with_selector(format!("link[rel='canonical'][href='{}']", canonical)))
                        .with_help("Add this URL to your sitemap or check the canonical"),
                    );
                }
            }
        }
    }

    // Check: sitemap entries should exist in dist
    if config.sitemap.entries_must_exist_in_dist {
        let known_routes = build_known_routes_set(&config.links.known_routes);
        for url_str in &index.sitemap_urls {
            if let Ok(parsed) = Url::parse(url_str) {
                let route = normalize::normalize_path(parsed.path(), norm);
                let is_known_route = known_routes
                    .as_ref()
                    .is_some_and(|set| set.is_match(&route));
                if !is_known_route && !index.route_exists(&route) {
                    findings.push(
                        Finding::fail(
                            "sitemap/entry-not-in-dist",
                            format!(
                                "Sitemap entry '{}' (route '{}') not found in dist",
                                url_str, route
                            ),
                        )
                        .with_severity(Severity::Medium)
                        .at(Location::file("sitemap.xml")
                            .with_selector(format!("<loc>{}</loc>", url_str)))
                        .with_help("Remove stale entries from sitemap or add the missing page"),
                    );
                }
            }
        }
    }

    // Check: sitemap should not contain non-canonical URLs
    if config.sitemap.forbid_noncanonical_in_sitemap {
        for url_str in &index.sitemap_urls {
            if let Ok(parsed) = Url::parse(url_str) {
                let route = normalize::normalize_path(parsed.path(), norm);
                // Find the page for this route
                if let Some(page) = index
                    .route_to_index
                    .get(&route)
                    .and_then(|&idx| index.pages.get(idx))
                {
                    if let Some(ref canonical) = page.canonical {
                        // Compare normalized forms to avoid false positives
                        let norm_sitemap_url = normalize_url(url_str, norm);
                        let norm_canonical = normalize_url(canonical, norm);
                        if norm_canonical != norm_sitemap_url {
                            findings.push(
                                Finding::fail(
                                    "sitemap/non-canonical-entry",
                                    format!(
                                        "Sitemap contains '{}' but page canonical is '{}'",
                                        url_str, canonical
                                    ),
                                )
                                .with_severity(Severity::Medium)
                                .at(Location::file("sitemap.xml")
                                    .with_selector(format!("<loc>{}</loc>", url_str)))
                                .with_help("Use the canonical URL in the sitemap"),
                            );
                        }
                    }
                }
            }
        }
    }

    findings
}
