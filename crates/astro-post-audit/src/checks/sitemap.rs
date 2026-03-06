use url::Url;

use crate::config::{Config, UrlNormalizationConfig};
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

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
            findings.push(Finding {
                level: Level::Error,
                rule_id: "sitemap/missing".into(),
                file: "sitemap.xml".into(),
                selector: String::new(),
                message: "sitemap.xml not found in dist directory".into(),
                help: "Configure Astro to generate a sitemap (e.g., @astrojs/sitemap)".into(),
                suggestion: None,
            });
        }
        return findings;
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
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "sitemap/canonical-missing".into(),
                        file: page.rel_path.clone(),
                        selector: format!("link[rel='canonical'][href='{}']", canonical),
                        message: format!(
                            "Canonical URL '{}' is not listed in sitemap.xml",
                            canonical
                        ),
                        help: "Add this URL to your sitemap or check the canonical".into(),
                        suggestion: None,
                    });
                }
            }
        }
    }

    // Check: sitemap entries should exist in dist
    if config.sitemap.entries_must_exist_in_dist {
        for url_str in &index.sitemap_urls {
            if let Ok(parsed) = Url::parse(url_str) {
                let route = normalize::normalize_path(parsed.path(), norm);
                if !index.route_exists(&route) {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "sitemap/entry-not-in-dist".into(),
                        file: "sitemap.xml".into(),
                        selector: format!("<loc>{}</loc>", url_str),
                        message: format!(
                            "Sitemap entry '{}' (route '{}') not found in dist",
                            url_str, route
                        ),
                        help: "Remove stale entries from sitemap or add the missing page".into(),
                        suggestion: None,
                    });
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
                if let Some(&idx) = index.route_to_index.get(&route) {
                    let page = &index.pages[idx];
                    if let Some(ref canonical) = page.canonical {
                        // Compare normalized forms to avoid false positives
                        let norm_sitemap_url = normalize_url(url_str, norm);
                        let norm_canonical = normalize_url(canonical, norm);
                        if norm_canonical != norm_sitemap_url {
                            findings.push(Finding {
                                level: Level::Warning,
                                rule_id: "sitemap/non-canonical-entry".into(),
                                file: "sitemap.xml".into(),
                                selector: format!("<loc>{}</loc>", url_str),
                                message: format!(
                                    "Sitemap contains '{}' but page canonical is '{}'",
                                    url_str, canonical
                                ),
                                help: "Use the canonical URL in the sitemap".into(),
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
