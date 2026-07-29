use std::collections::HashSet;

use scraper::Selector;
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Confidence, Finding, Level};

use std::sync::LazyLock;

static SCRIPT_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("head script[src]").expect("valid selector"));
static STYLESHEET_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("head link[rel='stylesheet'][href]").expect("valid selector"));
static PRELOAD_STYLE_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("head link[rel='preload'][as='style'][href]").expect("valid selector")
});
static PRECONNECT_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("head link[rel='preconnect'][href], head link[rel='dns-prefetch'][href]")
        .expect("valid selector")
});
static CRITICAL_RESOURCE_SELS: LazyLock<Vec<Selector>> = LazyLock::new(|| {
    [
        "head script[src]",
        "head link[rel='stylesheet'][href]",
        "head link[rel='preload'][href]",
    ]
    .iter()
    .filter_map(|s| Selector::parse(s).ok())
    .collect()
});

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.render_blocking.enabled {
        return Vec::new();
    }

    let mut findings = Vec::new();

    for page in &index.pages {
        let html = page.parse_html();
        let mut sync_scripts = 0usize;
        for script in html.select(&SCRIPT_SEL) {
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
                source_hint: None,
                confidence: Some(Confidence::Medium),
            });
        }

        let preload_styles: HashSet<String> = html
            .select(&PRELOAD_STYLE_SEL)
            .filter_map(|el| el.value().attr("href").map(|s| s.to_string()))
            .collect();
        for style in html.select(&STYLESHEET_SEL) {
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
                        source_hint: None,
                        confidence: Some(Confidence::Medium),
                    });
                }
            }
        }

        let known_preconnects: HashSet<String> = html
            .select(&PRECONNECT_SEL)
            .filter_map(|el| el.value().attr("href"))
            .filter_map(origin_from_href)
            .collect();

        let mut critical_third_party_origins: HashSet<String> = HashSet::new();
        for s in CRITICAL_RESOURCE_SELS.iter() {
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
                    source_hint: None,
                    confidence: Some(Confidence::Medium),
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
