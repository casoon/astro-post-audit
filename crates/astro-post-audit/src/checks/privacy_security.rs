use std::collections::HashSet;

use scraper::Selector;
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

const TRACKER_DOMAINS: &[&str] = &[
    "google-analytics.com",
    "googletagmanager.com",
    "doubleclick.net",
    "facebook.net",
    "connect.facebook.net",
    "analytics.tiktok.com",
    "hotjar.com",
];

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.privacy_security.enabled {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let url_sel = Selector::parse("[src], [href]").unwrap();
    let external_script_sel =
        Selector::parse("script[src^='http://'], script[src^='https://']").unwrap();
    let external_style_sel = Selector::parse(
        "link[rel='stylesheet'][href^='http://'], link[rel='stylesheet'][href^='https://']",
    )
    .unwrap();
    let inline_script_sel = Selector::parse(
        "script:not([src]):not([type='application/ld+json']):not([type='application/json'])",
    )
    .unwrap();

    for page in &index.pages {
        let html = page.parse_html();
        let mut third_party_domains: HashSet<String> = HashSet::new();

        for el in html.select(&url_sel) {
            let value = el
                .value()
                .attr("src")
                .or_else(|| el.value().attr("href"))
                .unwrap_or("");
            let Some(host) = host_from_url(value) else {
                continue;
            };
            if is_third_party_host(&host, index.base_url.as_deref()) {
                third_party_domains.insert(host);
            }
        }

        if !third_party_domains.is_empty() {
            findings.push(Finding {
                level: Level::Info,
                rule_id: "privacy-security/third-party-domains".into(),
                file: page.rel_path.clone(),
                selector: "head, body".into(),
                message: format!(
                    "Page loads resources from {} third-party domain(s): {}",
                    third_party_domains.len(),
                    join_limited(&third_party_domains, 4)
                ),
                help: "Review third-party dependencies for privacy and security impact".into(),
                suggestion: None,
            });
        }

        for el in html.select(&external_script_sel) {
            if el.value().attr("integrity").is_none() {
                let src = el.value().attr("src").unwrap_or("");
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "privacy-security/missing-sri-script".into(),
                    file: page.rel_path.clone(),
                    selector: format!("script[src='{}']", src),
                    message: format!("External script '{}' has no SRI integrity attribute", src),
                    help: "Add integrity + crossorigin for external scripts where possible".into(),
                    suggestion: None,
                });
            }
        }
        for el in html.select(&external_style_sel) {
            if el.value().attr("integrity").is_none() {
                let href = el.value().attr("href").unwrap_or("");
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "privacy-security/missing-sri-stylesheet".into(),
                    file: page.rel_path.clone(),
                    selector: format!("link[rel='stylesheet'][href='{}']", href),
                    message: format!(
                        "External stylesheet '{}' has no SRI integrity attribute",
                        href
                    ),
                    help: "Add integrity + crossorigin for external stylesheets where possible"
                        .into(),
                    suggestion: None,
                });
            }
        }

        let inline_script_count = html.select(&inline_script_sel).count();
        if inline_script_count > 0 {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "privacy-security/csp-readiness-inline-script".into(),
                file: page.rel_path.clone(),
                selector: "script".into(),
                message: format!(
                    "Found {} inline script(s), which weakens strict CSP readiness",
                    inline_script_count
                ),
                help: "Move inline scripts to external files or use CSP nonces/hashes".into(),
                suggestion: None,
            });
        }

        let tracker_present = third_party_domains.iter().any(|d| {
            TRACKER_DOMAINS
                .iter()
                .any(|t| d == t || d.ends_with(&format!(".{}", t)))
        });
        if tracker_present && !has_consent_indicator(&html) {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "privacy-security/missing-consent-indicator".into(),
                file: page.rel_path.clone(),
                selector: "body".into(),
                message:
                    "Tracking-related third-party domains detected without consent/CMP indicator"
                        .into(),
                help: "Ensure tracking scripts are gated behind a consent mechanism".into(),
                suggestion: None,
            });
        }
    }

    findings
}

fn host_from_url(value: &str) -> Option<String> {
    Url::parse(value)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_lowercase()))
}

fn is_third_party_host(host: &str, base_url: Option<&str>) -> bool {
    let Some(base_url) = base_url else {
        return true;
    };
    let Ok(base) = Url::parse(base_url) else {
        return true;
    };
    let Some(base_host) = base.host_str() else {
        return true;
    };
    host != base_host && !host.ends_with(&format!(".{}", base_host))
}

/// Known CMP (Consent Management Platform) indicators.
/// Each pattern must be specific enough to avoid false positives on unrelated
/// content (e.g. a `.cookie-recipe` class on a food blog).
const CONSENT_ID_CLASS_PATTERNS: &[&str] = &[
    "cookie-consent",
    "cookie-banner",
    "cookie-notice",
    "cookie-policy",
    "cookie-popup",
    "cookie-modal",
    "cookie-bar",
    "cookieconsent",
    "cookiebanner",
    "cookienotice",
    "consent-banner",
    "consent-modal",
    "consent-popup",
    "consent-notice",
    "consent-manager",
    "gdpr",
    "cmp-container",
    "cc-banner",
    "cc-window",
];

fn has_consent_indicator(html: &scraper::Html) -> bool {
    let Ok(sel) = Selector::parse(
        "[data-consent], [data-cookie-consent], [data-cookieconsent], script[src], script[type]",
    ) else {
        return false;
    };

    // Check data-attributes and script elements
    if html.select(&sel).any(|el| {
        // data-consent / data-cookie-consent attributes are strong signals by themselves
        if el.value().attr("data-consent").is_some()
            || el.value().attr("data-cookie-consent").is_some()
            || el.value().attr("data-cookieconsent").is_some()
        {
            return true;
        }
        let src = el.value().attr("src").unwrap_or("").to_lowercase();
        let typ = el.value().attr("type").unwrap_or("").to_lowercase();
        src.contains("onetrust")
            || src.contains("cookiebot")
            || src.contains("consentmanager")
            || src.contains("cookie-consent")
            || src.contains("cookieconsent")
            || typ.contains("consent")
    }) {
        return true;
    }

    // Check id/class with specific CMP-related patterns (not bare "cookie")
    let Ok(id_class_sel) = Selector::parse("[id], [class]") else {
        return false;
    };
    html.select(&id_class_sel).any(|el| {
        let id = el.value().attr("id").unwrap_or("").to_lowercase();
        let class = el.value().attr("class").unwrap_or("").to_lowercase();
        CONSENT_ID_CLASS_PATTERNS
            .iter()
            .any(|pattern| id.contains(pattern) || class.contains(pattern))
    })
}

fn join_limited(values: &HashSet<String>, max: usize) -> String {
    let mut vec: Vec<&str> = values.iter().map(|s| s.as_str()).collect();
    vec.sort_unstable();
    if vec.len() <= max {
        vec.join(", ")
    } else {
        format!("{}, ...", vec[..max].join(", "))
    }
}
