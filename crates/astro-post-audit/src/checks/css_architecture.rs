use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Confidence, Finding, Level};

static LINK_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("link[href]").expect("valid selector"));
static STYLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("style").expect("valid selector"));

struct RouteCss {
    file: String,
    route: String,
    bytes: u64,
    stylesheet_count: usize,
}

/// Measure CSS that is directly referenced by each generated page. The check
/// deliberately stays static: external stylesheets and CSS loaded later by
/// JavaScript cannot be weighed from dist alone.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.css_architecture.enabled {
        return Vec::new();
    }

    let mut size_cache: HashMap<PathBuf, u64> = HashMap::new();
    let routes = index
        .pages
        .iter()
        .map(|page| {
            let html = page.parse_html();
            let mut bytes = 0;
            let mut seen = HashSet::new();

            for link in html.select(&LINK_SEL) {
                let rel = link.value().attr("rel").unwrap_or("");
                if !rel
                    .split_ascii_whitespace()
                    .any(|value| value.eq_ignore_ascii_case("stylesheet"))
                {
                    continue;
                }
                let Some(href) = link.value().attr("href") else {
                    continue;
                };
                let Some(path) = resolve_local_stylesheet(href, &page.rel_path, &index.dist_path)
                else {
                    continue;
                };
                if !seen.insert(path.clone()) {
                    continue;
                }
                bytes += *size_cache
                    .entry(path.clone())
                    .or_insert_with(|| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0));
            }

            for style in html.select(&STYLE_SEL) {
                bytes += style.text().map(|text| text.len() as u64).sum::<u64>();
            }

            RouteCss {
                file: page.rel_path.clone(),
                route: page.route.clone(),
                bytes,
                stylesheet_count: seen.len(),
            }
        })
        .collect::<Vec<_>>();

    let mut findings = Vec::new();
    let max_bytes = config.css_architecture.max_route_kb.saturating_mul(1024);
    for route in &routes {
        if route.bytes > max_bytes {
            findings.push(payload_finding(route, config.css_architecture.max_route_kb));
        }
    }

    if config.css_architecture.detect_route_outliers && routes.len() >= 3 {
        let mut sizes = routes.iter().map(|route| route.bytes).collect::<Vec<_>>();
        sizes.sort_unstable();
        let median = sizes[sizes.len() / 2];
        let minimum = config.css_architecture.min_outlier_kb.saturating_mul(1024);
        for route in &routes {
            let exceeds_ratio = median == 0
                || route.bytes as f64 > median as f64 * config.css_architecture.outlier_factor;
            if route.bytes >= minimum && route.bytes > median && exceeds_ratio {
                findings.push(outlier_finding(
                    route,
                    median,
                    config.css_architecture.outlier_factor,
                ));
            }
        }
    }

    findings
}

fn payload_finding(route: &RouteCss, max_kb: u64) -> Finding {
    Finding::new(
        Level::Warning,
        "css-architecture/route-payload",
        route.file.clone(),
        "link[rel~='stylesheet'], style",
        format!(
            "Route '{}' loads {:.1}KB of local CSS across {} stylesheet(s) (max: {}KB)",
            route.route,
            route.bytes as f64 / 1024.0,
            route.stylesheet_count,
            max_kb
        ),
        "Split route-specific styles or remove CSS that is not needed by this route.",
        None,
    )
}

fn outlier_finding(route: &RouteCss, median: u64, factor: f64) -> Finding {
    Finding::new(
        Level::Info,
        "css-architecture/route-outlier",
        route.file.clone(),
        "link[rel~='stylesheet'], style",
        format!(
            "Route '{}' loads {:.1}KB of local CSS; the route median is {:.1}KB (outlier factor: {:.1}x)",
            route.route,
            route.bytes as f64 / 1024.0,
            median as f64 / 1024.0,
            factor
        ),
        "Review the stylesheets unique to this route and confirm that the difference is intentional.",
        Some(Confidence::Medium),
    )
}

fn resolve_local_stylesheet(href: &str, page_rel: &str, dist: &Path) -> Option<PathBuf> {
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("//")
        || href.starts_with("data:")
    {
        return None;
    }
    let clean = href.split(['?', '#']).next().unwrap_or(href);
    if clean.is_empty() {
        return None;
    }
    let candidate = if clean.starts_with('/') {
        dist.join(clean.trim_start_matches('/'))
    } else {
        let page_dir = Path::new(page_rel).parent().unwrap_or(Path::new(""));
        dist.join(page_dir).join(clean)
    };
    let resolved = candidate.canonicalize().ok()?;
    resolved.starts_with(dist).then_some(resolved)
}
