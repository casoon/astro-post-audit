use std::collections::HashSet;

use scraper::Selector;
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.render_blocking.enabled {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let script_sel = Selector::parse("head script[src]").unwrap();
    let stylesheet_sel = Selector::parse("head link[rel='stylesheet'][href]").unwrap();
    let preload_style_sel = Selector::parse("head link[rel='preload'][as='style'][href]").unwrap();
    let preconnect_sel =
        Selector::parse("head link[rel='preconnect'][href], head link[rel='dns-prefetch'][href]")
            .unwrap();
    let critical_resource_sels: Vec<Selector> = [
        "head script[src]",
        "head link[rel='stylesheet'][href]",
        "head link[rel='preload'][href]",
    ]
    .iter()
    .filter_map(|s| Selector::parse(s).ok())
    .collect();

    for page in &index.pages {
        let html = page.parse_html();
        let mut sync_scripts = 0usize;
        for script in html.select(&script_sel) {
            let attrs = script.value();
            let is_module = attrs
                .attr("type")
                .is_some_and(|t| t.eq_ignore_ascii_case("module"));
            let is_async = attrs.attr("async").is_some();
            let is_defer = attrs.attr("defer").is_some();
            if !is_module && !is_async && !is_defer {
                sync_scripts += 1;
            }
        }
        if sync_scripts > 0 {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "render-blocking/sync-head-scripts".into(),
                file: page.rel_path.clone(),
                selector: "head script[src]".into(),
                message: format!(
                    "Found {} synchronous head script(s) that can block rendering",
                    sync_scripts
                ),
                help: "Use defer/async (or type=module) for non-critical scripts in <head>".into(),
                suggestion: None,
            });
        }

        let preload_styles: HashSet<String> = html
            .select(&preload_style_sel)
            .filter_map(|el| el.value().attr("href").map(|s| s.to_string()))
            .collect();
        for style in html.select(&stylesheet_sel) {
            if let Some(href) = style.value().attr("href") {
                if !preload_styles.contains(href) {
                    findings.push(Finding {
                        level: Level::Info,
                        rule_id: "render-blocking/missing-style-preload".into(),
                        file: page.rel_path.clone(),
                        selector: format!("link[rel='stylesheet'][href='{}']", href),
                        message: format!("Stylesheet '{}' is not preloaded", href),
                        help:
                            "Preload critical above-the-fold styles when they are render-critical"
                                .into(),
                        suggestion: None,
                    });
                }
            }
        }

        let known_preconnects: HashSet<String> = html
            .select(&preconnect_sel)
            .filter_map(|el| el.value().attr("href"))
            .filter_map(origin_from_href)
            .collect();

        let mut critical_third_party_origins: HashSet<String> = HashSet::new();
        for s in &critical_resource_sels {
            for el in html.select(s) {
                let href = el
                    .value()
                    .attr("src")
                    .or_else(|| el.value().attr("href"))
                    .unwrap_or("");
                let Some(origin) = origin_from_href(href) else {
                    continue;
                };
                if is_third_party_origin(&origin, index.base_url.as_deref()) {
                    critical_third_party_origins.insert(origin);
                }
            }
        }

        for origin in critical_third_party_origins {
            if !known_preconnects.contains(&origin) {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "render-blocking/missing-preconnect".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: format!("Missing preconnect/dns-prefetch for critical origin '{}'", origin),
                    help: "Add <link rel=\"preconnect\"> (or dns-prefetch) for critical third-party origins"
                        .into(),
                    suggestion: None,
                });
            }
        }
    }

    findings
}

fn origin_from_href(href: &str) -> Option<String> {
    let parsed = Url::parse(href).ok()?;
    let host = parsed.host_str()?;
    Some(format!("{}://{}", parsed.scheme(), host))
}

fn is_third_party_origin(origin: &str, base_url: Option<&str>) -> bool {
    let Some(base_url) = base_url else {
        return true;
    };
    let Ok(base) = Url::parse(base_url) else {
        return true;
    };
    let Ok(other) = Url::parse(origin) else {
        return true;
    };
    base.host_str() != other.host_str()
}
