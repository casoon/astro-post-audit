use anyhow::Result;
use globset::{Glob, GlobSetBuilder};
use rayon::prelude::*;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::normalize;

/// Metadata for a single HTML page (Send-safe: stores raw HTML, not parsed DOM).
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Path relative to dist root (e.g., "about/index.html")
    pub rel_path: String,
    /// Normalized route URL (e.g., "/about/")
    pub route: String,
    /// Absolute URL if base_url is set (e.g., "https://example.com/about/")
    pub absolute_url: Option<String>,
    /// Raw HTML content (parsed on-demand per check for thread safety)
    pub html_content: String,
    /// Canonical URL found in the page (if any)
    pub canonical: Option<String>,
    /// Whether page has noindex meta
    pub noindex: bool,
    /// All canonical href values found on the page (in document order).
    pub canonical_hrefs: Vec<String>,
    /// All anchor href attributes found on the page.
    pub anchor_hrefs: Vec<String>,
    /// All element IDs found on the page.
    pub element_ids: HashSet<String>,
    /// Value of <html lang>, if present.
    pub html_lang: Option<String>,
    /// Text content of <title>, if present.
    pub title_text: Option<String>,
    /// Content of <meta name="description">, if present.
    pub meta_description: Option<String>,
    /// Target URL of a `<meta http-equiv="refresh" content="0;url=...">`, if the page is a redirect.
    pub meta_refresh_target: Option<String>,
}

impl PageInfo {
    /// Parse the HTML content on demand. Call this within each check's thread.
    pub fn parse_html(&self) -> Html {
        Html::parse_document(&self.html_content)
    }
}

/// In-memory index of all HTML pages in the dist directory.
#[derive(Debug)]
pub struct SiteIndex {
    /// All discovered pages
    pub pages: Vec<PageInfo>,
    /// Map from normalized route -> index into pages vec
    pub route_to_index: HashMap<String, usize>,
    /// Sitemap entries (absolute URL -> sitemap file in dist that lists it).
    /// Child sitemaps of a sitemap index are resolved offline.
    pub sitemap_urls: HashMap<String, String>,
    /// The root sitemap in dist (`sitemap-index.xml` or `sitemap.xml`), if any.
    pub sitemap_file: Option<String>,
    /// Parse error for the sitemap or one of its children, if any.
    pub sitemap_parse_error: Option<String>,
    /// Path to the dist directory
    pub dist_path: PathBuf,
    /// Base URL (if provided)
    pub base_url: Option<String>,
    /// Astro `base` without trailing slash (e.g. `/docs`), empty for a root deployment.
    /// Routes include it; files in dist do not.
    pub base_path: String,
    /// Total HTML files found in dist before include/exclude filtering (diagnostics).
    pub html_total: usize,
    /// HTML files remaining after include/exclude filtering (diagnostics).
    pub html_matched: usize,
}

impl SiteIndex {
    pub fn build(
        dist_path: &Path,
        config: &Config,
        include: &[String],
        exclude: &[String],
    ) -> Result<Self> {
        let dist_path = dist_path.canonicalize()?;

        // Build glob matchers for include/exclude
        let include_set = if include.is_empty() {
            None
        } else {
            let mut builder = GlobSetBuilder::new();
            for pattern in include {
                builder.add(Glob::new(pattern)?);
            }
            Some(builder.build()?)
        };

        let exclude_set = if exclude.is_empty() {
            None
        } else {
            let mut builder = GlobSetBuilder::new();
            for pattern in exclude {
                builder.add(Glob::new(pattern)?);
            }
            Some(builder.build()?)
        };

        // Discover all HTML files first (for diagnostics), then apply include/exclude.
        let all_html: Vec<(String, PathBuf)> = WalkDir::new(&dist_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "html" || ext == "htm")
            })
            .filter_map(|e| {
                let abs = e.path().to_path_buf();
                // Normalize path separators to forward slash for cross-platform glob matching
                let rel = abs
                    .strip_prefix(&dist_path)
                    .ok()?
                    .to_string_lossy()
                    .replace('\\', "/");
                Some((rel, abs))
            })
            .collect();
        let html_total = all_html.len();

        let html_files: Vec<(String, PathBuf)> = all_html
            .into_iter()
            .filter(|(rel, _)| {
                // Apply include/exclude filters (patterns always use forward slashes)
                if let Some(ref inc) = include_set {
                    if !inc.is_match(rel) {
                        return false;
                    }
                }
                if let Some(ref exc) = exclude_set {
                    if exc.is_match(rel) {
                        return false;
                    }
                }
                true
            })
            .collect();
        let html_matched = html_files.len();

        let base_url = config.site.base_url.clone();
        let base_path = normalize_base_path(config.site.base_path.as_deref());
        let norm_config = config.url_normalization.clone();
        let canonical_sel = Selector::parse("link[rel='canonical']").ok();
        let robots_sel = Selector::parse("meta[name='robots']").ok();
        let anchor_sel = Selector::parse("a[href]").ok();
        let id_sel = Selector::parse("[id]").ok();
        let lang_sel = Selector::parse("html[lang]").ok();
        let title_sel = Selector::parse("title").ok();
        let desc_sel = Selector::parse("meta[name='description']").ok();
        let meta_refresh_sel = Selector::parse("meta[http-equiv='refresh'][content]").ok();

        // Read and pre-extract metadata in parallel
        let pages: Vec<PageInfo> = html_files
            .par_iter()
            .filter_map(|(rel, abs)| {
                let content = match std::fs::read_to_string(abs) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Warning: could not read '{}': {}", rel, e);
                        return None;
                    }
                };

                // Parse once and extract reusable metadata.
                let html = Html::parse_document(&content);
                let canonical = canonical_sel
                    .as_ref()
                    .and_then(|sel| extract_canonical(&html, sel));
                let canonical_hrefs = canonical_sel
                    .as_ref()
                    .map(|sel| {
                        html.select(sel)
                            .filter_map(|el| el.value().attr("href"))
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                let noindex = robots_sel
                    .as_ref()
                    .is_some_and(|sel| has_noindex(&html, sel));
                let anchor_hrefs = anchor_sel
                    .as_ref()
                    .map(|sel| {
                        html.select(sel)
                            .filter_map(|el| el.value().attr("href"))
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                let element_ids = id_sel
                    .as_ref()
                    .map(|sel| {
                        html.select(sel)
                            .filter_map(|el| el.value().attr("id"))
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                let html_lang = lang_sel
                    .as_ref()
                    .and_then(|sel| html.select(sel).next())
                    .and_then(|el| el.value().attr("lang"))
                    .map(|s| s.to_string());
                let title_text = title_sel
                    .as_ref()
                    .and_then(|sel| html.select(sel).next())
                    .map(|el| el.text().collect::<String>().trim().to_string());
                let meta_description = desc_sel
                    .as_ref()
                    .and_then(|sel| html.select(sel).next())
                    .and_then(|el| el.value().attr("content"))
                    .map(|s| s.trim().to_string());
                let meta_refresh_target = meta_refresh_sel
                    .as_ref()
                    .and_then(|sel| html.select(sel).next())
                    .and_then(|el| el.value().attr("content"))
                    .and_then(parse_meta_refresh_target);

                let route = normalize::file_path_to_route(rel, &norm_config);
                let route = if base_path.is_empty() {
                    route
                } else {
                    normalize::normalize_path(&format!("{base_path}{route}"), &norm_config)
                };
                let absolute_url = base_url
                    .as_ref()
                    .and_then(|base| normalize::to_absolute(&route, base));

                Some(PageInfo {
                    rel_path: rel.clone(),
                    route,
                    absolute_url,
                    html_content: content,
                    canonical,
                    noindex,
                    canonical_hrefs,
                    anchor_hrefs,
                    element_ids,
                    html_lang,
                    title_text,
                    meta_description,
                    meta_refresh_target,
                })
            })
            .collect();

        // Build route index
        let mut route_to_index = HashMap::new();
        for (i, page) in pages.iter().enumerate() {
            route_to_index.insert(page.route.clone(), i);
        }

        // Parse sitemap (@astrojs/sitemap writes sitemap-index.xml plus children)
        let sitemap_file = ["sitemap-index.xml", "sitemap.xml"]
            .into_iter()
            .find(|name| dist_path.join(name).is_file())
            .map(str::to_string);
        let mut sitemap_urls = HashMap::new();
        let mut sitemap_parse_error = None;
        if let Some(file) = &sitemap_file {
            load_sitemap(
                &dist_path,
                file,
                &base_path,
                0,
                &mut HashSet::new(),
                &mut sitemap_urls,
                &mut sitemap_parse_error,
            );
        }

        Ok(Self {
            pages,
            route_to_index,
            sitemap_urls,
            sitemap_file,
            sitemap_parse_error,
            dist_path,
            base_url,
            base_path,
            html_total,
            html_matched,
        })
    }

    /// Check if a normalized route exists in the site index.
    pub fn route_exists(&self, route: &str) -> bool {
        self.route_to_index.contains_key(route)
    }

    /// Map a root-relative public URL path to its file in dist. `None` when the
    /// path lies outside the Astro `base`, so this build does not serve it.
    pub fn dist_path_for(&self, url_path: &str) -> Option<PathBuf> {
        dist_relative(url_path, &self.base_path).map(|rel| self.dist_path.join(rel))
    }
}

/// `/docs/` or `docs` -> `/docs`; `/` or nothing -> empty.
fn normalize_base_path(base: Option<&str>) -> String {
    let trimmed = base.unwrap_or("").trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
}

/// The dist-relative part of a root-relative public path, with `base_path`
/// stripped; `None` if the path does not start with the base.
fn dist_relative<'a>(url_path: &'a str, base_path: &str) -> Option<&'a str> {
    let rest = url_path.strip_prefix(base_path)?;
    if !rest.is_empty() && !rest.starts_with('/') {
        return None;
    }
    Some(rest.trim_start_matches('/'))
}

/// Parse the redirect target from a `meta http-equiv="refresh"` content value
/// (e.g. `"0;url=/target/"` -> `Some("/target/")`).
fn parse_meta_refresh_target(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    let pos = lower.find("url=")?;
    let target = content[(pos + 4)..]
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

fn extract_canonical(html: &Html, sel: &Selector) -> Option<String> {
    let element = html.select(sel).next()?;
    element.value().attr("href").map(|s| s.to_string())
}

fn has_noindex(html: &Html, sel: &Selector) -> bool {
    html.select(sel).any(|el| {
        el.value()
            .attr("content")
            .is_some_and(|c| c.to_lowercase().contains("noindex"))
    })
}

/// Nesting limit for sitemap indexes (an index of indexes is already unusual).
const MAX_SITEMAP_DEPTH: usize = 3;

/// Read the sitemap `file` (relative to dist) into `urls`; a sitemap index
/// pulls in its children from dist. The first problem ends up in `error`.
fn load_sitemap(
    dist: &Path,
    file: &str,
    base_path: &str,
    depth: usize,
    visited: &mut HashSet<String>,
    urls: &mut HashMap<String, String>,
    error: &mut Option<String>,
) {
    if !visited.insert(file.to_string()) {
        return;
    }
    let sitemap = match parse_sitemap(&dist.join(file)) {
        Ok(sitemap) => sitemap,
        Err(e) => {
            error.get_or_insert_with(|| format!("{file}: {e}"));
            return;
        }
    };
    if !sitemap.is_index {
        for url in sitemap.locs {
            urls.entry(url).or_insert_with(|| file.to_string());
        }
        return;
    }
    if depth >= MAX_SITEMAP_DEPTH {
        error.get_or_insert_with(|| format!("{file}: sitemap indexes nested too deeply"));
        return;
    }
    for loc in sitemap.locs {
        match child_sitemap_file(dist, base_path, &loc) {
            Some(child) => load_sitemap(dist, &child, base_path, depth + 1, visited, urls, error),
            None => {
                error.get_or_insert_with(|| {
                    format!("{file}: child sitemap '{loc}' not found in dist")
                });
            }
        }
    }
}

/// Map the URL of a child sitemap to its file in dist (Astro `base` stripped).
fn child_sitemap_file(dist: &Path, base_path: &str, loc: &str) -> Option<String> {
    let path = url::Url::parse(loc)
        .map(|u| u.path().to_string())
        .unwrap_or_else(|_| loc.to_string());
    let rel = dist_relative(&path, base_path)?;
    (!rel.is_empty() && dist.join(rel).is_file()).then(|| rel.to_string())
}

struct ParsedSitemap {
    is_index: bool,
    locs: Vec<String>,
}

fn parse_sitemap(path: &Path) -> Result<ParsedSitemap> {
    use quick_xml::escape::resolve_predefined_entity;
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);

    let mut urls = Vec::new();
    let mut is_index = false;
    let mut root_seen = false;
    // Text of the open <loc>; entities arrive as separate events since quick-xml 0.38.
    let mut loc: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if !root_seen => {
                root_seen = true;
                is_index = e.local_name().as_ref() == "sitemapindex";
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == "loc" => {
                loc = Some(String::new());
            }
            Ok(Event::Text(ref e)) => {
                if let Some(text) = loc.as_mut() {
                    text.push_str(&e.xml10_content());
                }
            }
            Ok(Event::CData(ref e)) => {
                if let Some(text) = loc.as_mut() {
                    text.push_str(&e.xml10_content());
                }
            }
            Ok(Event::GeneralRef(ref e)) => {
                if let Some(text) = loc.as_mut() {
                    if let Ok(Some(c)) = e.resolve_char_ref() {
                        text.push(c);
                    } else if let Some(entity) = resolve_predefined_entity(&e.xml10_content()) {
                        text.push_str(entity);
                    }
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == "loc" => {
                if let Some(text) = loc.take() {
                    urls.push(text.trim().to_string());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("invalid XML: {e}"),
            _ => {}
        }
    }

    Ok(ParsedSitemap {
        is_index,
        locs: urls,
    })
}
