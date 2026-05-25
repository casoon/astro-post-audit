use rayon::prelude::*;
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.go_live.enabled {
        return Vec::new();
    }

    // Resolve expected origin from go_live.expected_site or site.base_url
    let expected_origin = match resolve_origin(
        config.go_live.expected_site.as_deref(),
        config.site.base_url.as_deref(),
    ) {
        Some(o) => o,
        None => {
            return vec![Finding {
                level: Level::Error,
                rule_id: "golive/config-missing-site".into(),
                file: "astro.config.mjs".into(),
                selector: "goLive".into(),
                message: "go-live enabled but no expected site configured".into(),
                help: "Set `site` in astro.config.mjs or `goLive.expectedSite` in postAudit() options".into(),
                suggestion: None,
                source_hint: None,
                confidence: None,
            }];
        }
    };

    let forbidden = &config.go_live.forbidden_domains;

    let mut findings: Vec<Finding> = index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut f = Vec::new();

            // noindex check
            check_noindex(page, &mut f);

            // canonical origin check
            check_canonical_origin(page, &expected_origin, &mut f);

            // OG url/image origin check
            check_og_origin(page, &expected_origin, &mut f);

            // forbidden domains in absolute links, scripts, and OG
            if !forbidden.is_empty() {
                check_forbidden_domains(page, forbidden, &mut f);
            }

            f
        })
        .collect();

    // Sitemap URL origin check (cross-page, sequential)
    check_sitemap_origins(index, &expected_origin, forbidden, &mut findings);

    // robots.txt global disallow check
    check_robots_txt_blocked(index, &mut findings);

    findings
}

fn resolve_origin(expected_site: Option<&str>, site_base_url: Option<&str>) -> Option<String> {
    let url_str = expected_site.or(site_base_url)?;
    let url = Url::parse(url_str).ok()?;
    let scheme = url.scheme();
    let host = url.host_str()?;
    let port = url.port();
    Some(match port {
        Some(p) => format!("{}://{}:{}", scheme, host, p),
        None => format!("{}://{}", scheme, host),
    })
}

fn check_noindex(page: &crate::discovery::PageInfo, findings: &mut Vec<Finding>) {
    let html = page.parse_html();
    let selector_str = "meta[name='robots']";
    let sel = scraper::Selector::parse(selector_str).unwrap();
    for el in html.select(&sel) {
        let content = el.value().attr("content").unwrap_or("").to_lowercase();
        if content.contains("noindex") {
            findings.push(Finding {
                level: Level::Error,
                rule_id: "golive/noindex".into(),
                file: page.rel_path.clone(),
                selector: selector_str.into(),
                message: "Page has noindex directive — must be removed before going live".into(),
                help: "Remove `noindex` from the robots meta tag or delete it entirely".into(),
                suggestion: None,
                source_hint: None,
                confidence: None,
            });
            return;
        }
    }
}

fn check_canonical_origin(
    page: &crate::discovery::PageInfo,
    expected_origin: &str,
    findings: &mut Vec<Finding>,
) {
    for href in &page.canonical_hrefs {
        if let Ok(url) = Url::parse(href) {
            let actual_origin = match url.port() {
                Some(p) => format!("{}://{}:{}", url.scheme(), url.host_str().unwrap_or(""), p),
                None => format!("{}://{}", url.scheme(), url.host_str().unwrap_or("")),
            };
            if actual_origin != expected_origin {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "golive/canonical-origin".into(),
                    file: page.rel_path.clone(),
                    selector: "link[rel='canonical']".into(),
                    message: format!(
                        "Canonical URL uses '{}' instead of expected production origin '{}'",
                        actual_origin, expected_origin
                    ),
                    help: "Canonical URLs must point to the production origin".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
        }
    }
}

fn check_og_origin(
    page: &crate::discovery::PageInfo,
    expected_origin: &str,
    findings: &mut Vec<Finding>,
) {
    let html = page.parse_html();
    for prop in &["og:url", "og:image"] {
        let sel_str = format!("meta[property='{}']", prop);
        let sel = scraper::Selector::parse(&sel_str).unwrap();
        for el in html.select(&sel) {
            let content = el.value().attr("content").unwrap_or("");
            if let Ok(url) = Url::parse(content) {
                let actual_origin = match url.port() {
                    Some(p) => format!("{}://{}:{}", url.scheme(), url.host_str().unwrap_or(""), p),
                    None => format!("{}://{}", url.scheme(), url.host_str().unwrap_or("")),
                };
                if actual_origin != expected_origin {
                    findings.push(Finding {
                        level: Level::Error,
                        rule_id: "golive/og-origin".into(),
                        file: page.rel_path.clone(),
                        selector: sel_str.clone(),
                        message: format!(
                            "{} uses '{}' instead of expected production origin '{}'",
                            prop, actual_origin, expected_origin
                        ),
                        help: format!("Set {} to use the production origin", prop),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
            }
        }
    }
}

fn is_forbidden(url_str: &str, forbidden: &[String]) -> Option<String> {
    let url = Url::parse(url_str).ok()?;
    let host = url.host_str()?.to_lowercase();
    for f in forbidden {
        let f_lower = f.to_lowercase();
        if host == f_lower || host.ends_with(&format!(".{}", f_lower)) {
            return Some(f.clone());
        }
    }
    None
}

fn check_forbidden_domains(
    page: &crate::discovery::PageInfo,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    let html = page.parse_html();

    // Check absolute links
    let a_sel = scraper::Selector::parse("a[href]").unwrap();
    for el in html.select(&a_sel) {
        let href = el.value().attr("href").unwrap_or("");
        if href.starts_with("http://") || href.starts_with("https://") {
            if let Some(domain) = is_forbidden(href, forbidden) {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "golive/forbidden-domain".into(),
                    file: page.rel_path.clone(),
                    selector: "a[href]".into(),
                    message: format!("Link contains forbidden domain '{}': {}", domain, href),
                    help: "Remove or replace links pointing to staging/dev domains".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
        }
    }

    // Check script src
    let script_sel = scraper::Selector::parse("script[src]").unwrap();
    for el in html.select(&script_sel) {
        let src = el.value().attr("src").unwrap_or("");
        if src.starts_with("http://") || src.starts_with("https://") {
            if let Some(domain) = is_forbidden(src, forbidden) {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "golive/forbidden-domain".into(),
                    file: page.rel_path.clone(),
                    selector: "script[src]".into(),
                    message: format!("Script src contains forbidden domain '{}': {}", domain, src),
                    help: "Remove or replace scripts pointing to staging/dev domains".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
        }
    }

    // Check canonical href
    for href in &page.canonical_hrefs {
        if let Some(domain) = is_forbidden(href, forbidden) {
            findings.push(Finding {
                level: Level::Error,
                rule_id: "golive/forbidden-domain".into(),
                file: page.rel_path.clone(),
                selector: "link[rel='canonical']".into(),
                message: format!("Canonical URL contains forbidden domain '{}': {}", domain, href),
                help: "Update canonical URLs to use the production domain".into(),
                suggestion: None,
                source_hint: None,
                confidence: None,
            });
        }
    }
}

fn check_sitemap_origins(
    index: &SiteIndex,
    expected_origin: &str,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    for url in &index.sitemap_urls {
        if let Ok(parsed) = Url::parse(url) {
            let actual_origin = match parsed.port() {
                Some(p) => format!("{}://{}:{}", parsed.scheme(), parsed.host_str().unwrap_or(""), p),
                None => format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or("")),
            };
            if actual_origin != expected_origin {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "golive/sitemap-origin".into(),
                    file: "sitemap.xml".into(),
                    selector: "<loc>".into(),
                    message: format!(
                        "Sitemap entry '{}' uses '{}' instead of expected production origin '{}'",
                        url, actual_origin, expected_origin
                    ),
                    help: "Regenerate the sitemap with the production `site` URL".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
        }
        if !forbidden.is_empty() {
            if let Some(domain) = is_forbidden(url, forbidden) {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "golive/forbidden-domain".into(),
                    file: "sitemap.xml".into(),
                    selector: "<loc>".into(),
                    message: format!("Sitemap entry contains forbidden domain '{}': {}", domain, url),
                    help: "Regenerate the sitemap with the production `site` URL".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
        }
    }
}

fn check_robots_txt_blocked(index: &SiteIndex, findings: &mut Vec<Finding>) {
    let robots_path = index.dist_path.join("robots.txt");
    let content = match std::fs::read_to_string(&robots_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Look for a block that globally disallows crawling: User-agent: * + Disallow: /
    let mut current_user_agent_is_all = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if lower.starts_with("user-agent:") {
            let agent = lower.trim_start_matches("user-agent:").trim();
            current_user_agent_is_all = agent == "*";
        } else if lower.starts_with("disallow:") && current_user_agent_is_all {
            let path = lower.trim_start_matches("disallow:").trim();
            if path == "/" {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "golive/robots-blocked".into(),
                    file: "robots.txt".into(),
                    selector: "Disallow: /".into(),
                    message: "robots.txt globally blocks all crawlers with 'Disallow: /'".into(),
                    help: "Remove 'Disallow: /' for the '*' user-agent before going live. Use specific path disallows if needed.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
                return;
            }
        }
    }
}
