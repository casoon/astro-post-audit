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
    /// Absolute file system path (used by asset checks for file-level operations)
    #[allow(dead_code)]
    pub abs_path: PathBuf,
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
    /// Whether <meta name="viewport"> exists.
    pub has_viewport: bool,
    /// Number of <h1> elements on the page.
    pub h1_count: usize,
    /// Heading levels in document order (h1..h6).
    pub heading_levels: Vec<u8>,
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
    /// Sitemap entries (absolute URLs) if sitemap.xml exists
    pub sitemap_urls: HashSet<String>,
    /// Path to the dist directory
    pub dist_path: PathBuf,
    /// Base URL (if provided)
    pub base_url: Option<String>,
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

        // Discover HTML files
        let html_files: Vec<(String, PathBuf)> = WalkDir::new(&dist_path)
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

                // Apply include/exclude filters (patterns always use forward slashes)
                if let Some(ref inc) = include_set {
                    if !inc.is_match(&rel) {
                        return None;
                    }
                }
                if let Some(ref exc) = exclude_set {
                    if exc.is_match(&rel) {
                        return None;
                    }
                }

                Some((rel, abs))
            })
            .collect();

        let base_url = config.site.base_url.clone();
        let norm_config = config.url_normalization.clone();
        let canonical_sel = Selector::parse("link[rel='canonical']").ok();
        let robots_sel = Selector::parse("meta[name='robots']").ok();
        let anchor_sel = Selector::parse("a[href]").ok();
        let id_sel = Selector::parse("[id]").ok();
        let lang_sel = Selector::parse("html[lang]").ok();
        let title_sel = Selector::parse("title").ok();
        let desc_sel = Selector::parse("meta[name='description']").ok();
        let viewport_sel = Selector::parse("meta[name='viewport']").ok();
        let h1_sel = Selector::parse("h1").ok();
        let headings_sel = Selector::parse("h1, h2, h3, h4, h5, h6").ok();

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
                let has_viewport = viewport_sel
                    .as_ref()
                    .is_some_and(|sel| html.select(sel).next().is_some());
                let h1_count = h1_sel
                    .as_ref()
                    .map(|sel| html.select(sel).count())
                    .unwrap_or(0);
                let heading_levels = headings_sel
                    .as_ref()
                    .map(|sel| {
                        html.select(sel)
                            .filter_map(|el| {
                                el.value()
                                    .name()
                                    .strip_prefix('h')
                                    .and_then(|n| n.parse::<u8>().ok())
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let route = normalize::file_path_to_route(rel, &norm_config);
                let absolute_url = base_url
                    .as_ref()
                    .and_then(|base| normalize::to_absolute(&route, base));

                Some(PageInfo {
                    rel_path: rel.clone(),
                    abs_path: abs.clone(),
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
                    has_viewport,
                    h1_count,
                    heading_levels,
                })
            })
            .collect();

        // Build route index
        let mut route_to_index = HashMap::new();
        for (i, page) in pages.iter().enumerate() {
            route_to_index.insert(page.route.clone(), i);
        }

        // Parse sitemap
        let sitemap_path = dist_path.join("sitemap.xml");
        let sitemap_urls: HashSet<String> = if sitemap_path.exists() {
            parse_sitemap(&sitemap_path)
                .unwrap_or_default()
                .into_iter()
                .collect()
        } else {
            HashSet::new()
        };

        Ok(Self {
            pages,
            route_to_index,
            sitemap_urls,
            dist_path,
            base_url,
        })
    }

    /// Check if a normalized route exists in the site index.
    pub fn route_exists(&self, route: &str) -> bool {
        self.route_to_index.contains_key(route)
    }

    /// Check if a file (relative path) exists in dist.
    pub fn file_exists(&self, rel_path: &str) -> bool {
        self.dist_path.join(rel_path).exists()
    }
}

fn extract_canonical(html: &Html, sel: &Selector) -> Option<String> {
    let element = html.select(&sel).next()?;
    element.value().attr("href").map(|s| s.to_string())
}

fn has_noindex(html: &Html, sel: &Selector) -> bool {
    html.select(&sel).any(|el| {
        el.value()
            .attr("content")
            .is_some_and(|c| c.to_lowercase().contains("noindex"))
    })
}

fn parse_sitemap(path: &Path) -> Result<Vec<String>> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);

    let mut urls = Vec::new();
    let mut in_loc = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"loc" => {
                in_loc = true;
            }
            Ok(Event::Text(ref e)) if in_loc => {
                if let Ok(text) = e.unescape() {
                    urls.push(text.trim().to_string());
                }
                in_loc = false;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"loc" => {
                in_loc = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    Ok(urls)
}
