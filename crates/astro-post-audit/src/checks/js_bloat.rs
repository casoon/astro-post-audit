use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Confidence, Finding, Level};

/// Detect client-side JS bloat per route. Astro ships zero JS by default, so a
/// page that loads a lot of script bytes usually means a heavy island
/// (`client:load`/`client:only`) was added where static HTML would do.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.js_bloat.enabled {
        return Vec::new();
    }

    let max_kb = config.js_bloat.max_kb;
    let script_sel = Selector::parse("script[src]").unwrap();
    let inline_script_sel = Selector::parse(
        "script:not([src]):not([type='application/ld+json']):not([type='application/json'])",
    )
    .unwrap();
    let island_sel = Selector::parse("astro-island").unwrap();

    // Cache file sizes — the same hashed chunk is referenced by many pages.
    let mut size_cache: HashMap<PathBuf, u64> = HashMap::new();
    let mut findings = Vec::new();

    for page in &index.pages {
        let html = page.parse_html();

        let mut total_bytes: u64 = 0;
        let mut seen: HashSet<PathBuf> = HashSet::new();

        for el in html.select(&script_sel) {
            let Some(src) = el.value().attr("src") else {
                continue;
            };
            let Some(path) = resolve_local_script(src, &page.rel_path, &index.dist_path) else {
                continue;
            };
            if !seen.insert(path.clone()) {
                continue;
            }
            let size = *size_cache
                .entry(path.clone())
                .or_insert_with(|| std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0));
            total_bytes += size;
        }

        // Inline scripts ship their full text to the client too.
        for el in html.select(&inline_script_sel) {
            total_bytes += el.text().map(|t| t.len() as u64).sum::<u64>();
        }

        let total_kb = total_bytes / 1024;
        if total_kb > max_kb {
            let island_count = html.select(&island_sel).count();
            let island_note = if island_count > 0 {
                format!(" across {island_count} Astro island(s)")
            } else {
                String::new()
            };
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "performance/js-bloat".into(),
                file: page.rel_path.clone(),
                selector: "script".into(),
                message: format!(
                    "Route '{}' loads {}KB of client-side JavaScript{} (max: {}KB)",
                    page.route, total_kb, island_note, max_kb
                ),
                help: "Consider using `client:visible`/`client:idle`, or removing interactivity if the content can be static.".into(),
                suggestion: None,
                source_hint: None,
                confidence: Some(Confidence::Medium),
            });
        }
    }

    findings
}

/// Resolve a `<script src>` to a local file path inside dist, or None for
/// external/data URLs that cannot be weighed locally.
fn resolve_local_script(src: &str, page_rel: &str, dist: &Path) -> Option<PathBuf> {
    if src.starts_with("http://")
        || src.starts_with("https://")
        || src.starts_with("//")
        || src.starts_with("data:")
    {
        return None;
    }
    let clean = src.split('?').next().unwrap_or(src);
    let clean = clean.split('#').next().unwrap_or(clean);
    if clean.is_empty() {
        return None;
    }
    if clean.starts_with('/') {
        Some(dist.join(clean.trim_start_matches('/')))
    } else {
        let page_dir = Path::new(page_rel).parent().unwrap_or(Path::new(""));
        Some(dist.join(page_dir).join(clean))
    }
}
