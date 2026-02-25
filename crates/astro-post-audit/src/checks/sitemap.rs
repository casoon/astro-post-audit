use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

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
            });
        }
        return findings;
    }

    if index.sitemap_urls.is_empty() {
        return findings;
    }

    // Check: canonical URLs should be in sitemap
    // Normalize both canonical and sitemap URLs for reliable comparison
    if config.sitemap.canonical_must_be_in_sitemap {
        let normalized_sitemap: std::collections::HashSet<String> = index
            .sitemap_urls
            .iter()
            .filter_map(|u| {
                Url::parse(u).ok().map(|parsed| {
                    let norm_path =
                        normalize::normalize_path(parsed.path(), &config.url_normalization);
                    let mut rebuilt = parsed.clone();
                    rebuilt.set_path(&norm_path);
                    rebuilt.to_string()
                })
            })
            .collect();

        for page in &index.pages {
            if page.noindex {
                continue; // noindex pages shouldn't be in sitemap
            }
            if let Some(ref canonical) = page.canonical {
                // Normalize the canonical URL too
                let norm_canonical = if let Ok(parsed) = Url::parse(canonical) {
                    let norm_path =
                        normalize::normalize_path(parsed.path(), &config.url_normalization);
                    let mut rebuilt = parsed.clone();
                    rebuilt.set_path(&norm_path);
                    rebuilt.to_string()
                } else {
                    canonical.clone()
                };

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
                    });
                }
            }
        }
    }

    // Check: sitemap entries should exist in dist
    if config.sitemap.entries_must_exist_in_dist {
        for url_str in &index.sitemap_urls {
            if let Ok(parsed) = Url::parse(url_str) {
                let route = normalize::normalize_path(parsed.path(), &config.url_normalization);
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
                    });
                }
            }
        }
    }

    // Check: sitemap should not contain non-canonical URLs
    if config.sitemap.forbid_noncanonical_in_sitemap {
        for url_str in &index.sitemap_urls {
            if let Ok(parsed) = Url::parse(url_str) {
                let route = normalize::normalize_path(parsed.path(), &config.url_normalization);
                // Find the page for this route
                if let Some(&idx) = index.route_to_index.get(&route) {
                    let page = &index.pages[idx];
                    if let Some(ref canonical) = page.canonical {
                        if canonical != url_str {
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
                            });
                        }
                    }
                }
            }
        }
    }

    findings
}
