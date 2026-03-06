use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

/// Collect all unique external URLs across all pages, then check them via HEAD requests.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.external_links.enabled {
        return Vec::new();
    }

    let timeout = Duration::from_millis(config.external_links.timeout_ms);

    // Phase 1: Collect all external URLs and which pages reference them
    let mut url_to_pages: HashMap<String, Vec<String>> = HashMap::new();

    for page in &index.pages {
        let html = page.parse_html();
        let sel = match Selector::parse("a[href]") {
            Ok(s) => s,
            Err(_) => continue,
        };

        for el in html.select(&sel) {
            let href = match el.value().attr("href") {
                Some(h) => h.trim(),
                None => continue,
            };

            // Only external links (http/https, not internal)
            if !href.starts_with("http://") && !href.starts_with("https://") {
                continue;
            }
            if normalize::is_internal(href, index.base_url.as_deref()) {
                continue;
            }

            // Strip fragment for checking
            let check_url = href.split('#').next().unwrap_or(href).to_string();
            if check_url.is_empty() {
                continue;
            }

            // Apply domain filters
            if let Ok(parsed) = url::Url::parse(&check_url) {
                if let Some(host) = parsed.host_str() {
                    if !config.external_links.allow_domains.is_empty()
                        && !config
                            .external_links
                            .allow_domains
                            .iter()
                            .any(|d| host == d || host.ends_with(&format!(".{}", d)))
                    {
                        continue;
                    }
                    if config
                        .external_links
                        .block_domains
                        .iter()
                        .any(|d| host == d || host.ends_with(&format!(".{}", d)))
                    {
                        continue;
                    }
                }
            }

            url_to_pages
                .entry(check_url)
                .or_default()
                .push(page.rel_path.clone());
        }
    }

    if url_to_pages.is_empty() {
        return Vec::new();
    }

    // Phase 2: Check URLs in parallel (bounded by max_concurrent via rayon thread pool)
    let urls: Vec<(String, Vec<String>)> = url_to_pages.into_iter().collect();
    let findings = Mutex::new(Vec::new());

    // Use a custom thread pool to limit concurrency
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.external_links.max_concurrent)
        .build()
        .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

    pool.install(|| {
        urls.par_iter().for_each(|(url, pages)| {
            let result = check_url(url, timeout);
            if let Some((status, message)) = result {
                let level = if config.external_links.fail_on_broken {
                    Level::Error
                } else {
                    Level::Warning
                };

                let mut f = findings.lock().unwrap();
                for page in pages {
                    f.push(Finding {
                        level: level.clone(),
                        rule_id: "external-links/broken".into(),
                        file: page.clone(),
                        selector: format!("a[href='{}']", url),
                        message: format!("{} (status: {})", message, status),
                        help: "Fix or remove this broken external link".into(),
                        suggestion: None,
                    });
                }
            }
        });
    });

    findings.into_inner().unwrap()
}

/// Check a single URL. Returns Some((status_code, message)) if broken, None if OK.
fn check_url(url: &str, timeout: Duration) -> Option<(String, String)> {
    let agent = ureq::config::Config::builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .new_agent();

    // Try HEAD first, fall back to GET if HEAD is not allowed (405)
    match agent.head(url).call() {
        Ok(response) => {
            let status = response.status();
            if status.as_u16() == 405 {
                // HEAD not allowed, try GET
                return check_url_get(&agent, url);
            }
            if status.as_u16() >= 400 {
                Some((
                    status.as_u16().to_string(),
                    format!("External link '{}' returned HTTP {}", url, status.as_u16()),
                ))
            } else {
                None
            }
        }
        Err(e) => Some((
            "timeout/error".into(),
            format!("External link '{}' failed: {}", url, e),
        )),
    }
}

fn check_url_get(agent: &ureq::Agent, url: &str) -> Option<(String, String)> {
    match agent.get(url).call() {
        Ok(response) => {
            let status = response.status();
            if status.as_u16() >= 400 {
                Some((
                    status.as_u16().to_string(),
                    format!("External link '{}' returned HTTP {}", url, status.as_u16()),
                ))
            } else {
                None
            }
        }
        Err(e) => Some((
            "timeout/error".into(),
            format!("External link '{}' failed: {}", url, e),
        )),
    }
}
