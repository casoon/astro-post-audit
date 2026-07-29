use rayon::prelude::*;
use scraper::Selector;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::discovery::SiteIndex;

/// Properties collected for a single page.
#[derive(Debug, Clone, Serialize)]
pub struct PageProperties {
    pub file: String,
    pub route: String,
    pub title: Option<String>,
    pub meta_description: Option<String>,
    pub has_canonical: bool,
    pub canonical_url: Option<String>,
    pub has_og_title: bool,
    pub has_og_description: bool,
    pub has_og_image: bool,
    pub h1_count: usize,
    pub h1_text: Option<String>,
    pub has_lang_attr: bool,
    pub lang_value: Option<String>,
    pub has_json_ld: bool,
    pub json_ld_types: Vec<String>,
    pub has_skip_link: bool,
    pub noindex: bool,
}

/// Aggregate statistics across all pages.
#[derive(Debug, Clone, Serialize)]
pub struct OverviewStats {
    pub total_pages: usize,
    pub pages_with_title: usize,
    pub pages_with_description: usize,
    pub pages_with_canonical: usize,
    pub pages_with_og_title: usize,
    pub pages_with_og_description: usize,
    pub pages_with_og_image: usize,
    pub pages_with_h1: usize,
    pub pages_with_lang: usize,
    pub pages_with_json_ld: usize,
    pub pages_with_skip_link: usize,
    pub pages_with_noindex: usize,
    pub json_ld_type_counts: Vec<(String, usize)>,
}

/// Complete overview result.
#[derive(Debug, Clone, Serialize)]
pub struct PageOverview {
    pub pages: Vec<PageProperties>,
    pub stats: OverviewStats,
}

static TITLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("title").expect("valid selector"));
static DESC_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[name='description']").expect("valid selector"));
static OG_TITLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:title']").expect("valid selector"));
static OG_DESC_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:description']").expect("valid selector"));
static OG_IMG_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:image']").expect("valid selector"));
static H1_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h1").expect("valid selector"));
static HTML_LANG_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("html[lang]").expect("valid selector"));
static LD_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("script[type='application/ld+json']").expect("valid selector")
});
static SKIP_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a[href^='#']").expect("valid selector"));

/// Collect page properties from all pages in the site index.
pub fn collect(index: &SiteIndex) -> PageOverview {
    let mut pages: Vec<PageProperties> = index
        .pages
        .par_iter()
        .map(|page| {
            let html = page.parse_html();

            // Title
            let title = html
                .select(&TITLE_SEL)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            // Meta description
            let meta_description = html
                .select(&DESC_SEL)
                .next()
                .and_then(|el| el.value().attr("content"))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            // Canonical (reuse from PageInfo)
            let has_canonical = page.canonical.is_some();
            let canonical_url = page.canonical.clone();

            // OG tags
            let has_og_title = html
                .select(&OG_TITLE_SEL)
                .next()
                .and_then(|el| el.value().attr("content"))
                .is_some_and(|v| !v.trim().is_empty());

            let has_og_description = html
                .select(&OG_DESC_SEL)
                .next()
                .and_then(|el| el.value().attr("content"))
                .is_some_and(|v| !v.trim().is_empty());

            let has_og_image = html
                .select(&OG_IMG_SEL)
                .next()
                .and_then(|el| el.value().attr("content"))
                .is_some_and(|v| !v.trim().is_empty());

            // H1
            let h1s: Vec<_> = html.select(&H1_SEL).collect();
            let h1_count = h1s.len();
            let h1_text = h1s
                .first()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            // Lang attr
            let lang_el = html.select(&HTML_LANG_SEL).next();
            let has_lang_attr = lang_el.is_some();
            let lang_value = lang_el
                .and_then(|el| el.value().attr("lang"))
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());

            // JSON-LD
            let ld_scripts: Vec<_> = html.select(&LD_SEL).collect();
            let has_json_ld = !ld_scripts.is_empty();
            let mut json_ld_types = Vec::new();
            for script in &ld_scripts {
                let content: String = script.text().collect();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(content.trim()) {
                    extract_types(&val, &mut json_ld_types);
                }
            }

            // Skip link
            let has_skip_link = html.select(&SKIP_SEL).any(|el| {
                let href = el.value().attr("href").unwrap_or("");
                let target = href.trim_start_matches('#').to_lowercase();
                matches!(
                    target.as_str(),
                    "main" | "main-content" | "maincontent" | "content" | "skip" | "inhalt"
                )
            });

            PageProperties {
                file: page.rel_path.clone(),
                route: page.route.clone(),
                title,
                meta_description,
                has_canonical,
                canonical_url,
                has_og_title,
                has_og_description,
                has_og_image,
                h1_count,
                h1_text,
                has_lang_attr,
                lang_value,
                has_json_ld,
                json_ld_types,
                has_skip_link,
                noindex: page.noindex,
            }
        })
        .collect();

    pages.sort_by(|a, b| a.file.cmp(&b.file));
    let stats = build_stats(&pages);
    PageOverview { pages, stats }
}

/// Recursively extract @type values from JSON-LD, handling @graph arrays.
fn extract_types(val: &serde_json::Value, out: &mut Vec<String>) {
    match val {
        serde_json::Value::Object(map) => {
            if let Some(t) = map.get("@type") {
                match t {
                    serde_json::Value::String(s) => out.push(s.clone()),
                    serde_json::Value::Array(arr) => {
                        for item in arr {
                            if let serde_json::Value::String(s) = item {
                                out.push(s.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            if let Some(serde_json::Value::Array(graph)) = map.get("@graph") {
                for item in graph {
                    extract_types(item, out);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                extract_types(item, out);
            }
        }
        _ => {}
    }
}

fn build_stats(pages: &[PageProperties]) -> OverviewStats {
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for p in pages {
        for t in &p.json_ld_types {
            *type_counts.entry(t.clone()).or_default() += 1;
        }
    }
    let mut json_ld_type_counts: Vec<(String, usize)> = type_counts.into_iter().collect();
    json_ld_type_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    OverviewStats {
        total_pages: pages.len(),
        pages_with_title: pages.iter().filter(|p| p.title.is_some()).count(),
        pages_with_description: pages
            .iter()
            .filter(|p| p.meta_description.is_some())
            .count(),
        pages_with_canonical: pages.iter().filter(|p| p.has_canonical).count(),
        pages_with_og_title: pages.iter().filter(|p| p.has_og_title).count(),
        pages_with_og_description: pages.iter().filter(|p| p.has_og_description).count(),
        pages_with_og_image: pages.iter().filter(|p| p.has_og_image).count(),
        pages_with_h1: pages.iter().filter(|p| p.h1_count > 0).count(),
        pages_with_lang: pages.iter().filter(|p| p.has_lang_attr).count(),
        pages_with_json_ld: pages.iter().filter(|p| p.has_json_ld).count(),
        pages_with_skip_link: pages.iter().filter(|p| p.has_skip_link).count(),
        pages_with_noindex: pages.iter().filter(|p| p.noindex).count(),
        json_ld_type_counts,
    }
}
