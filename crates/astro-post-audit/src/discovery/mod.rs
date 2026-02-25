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
                let rel = abs
                    .strip_prefix(&dist_path)
                    .ok()?
                    .to_string_lossy()
                    .to_string();

                // Apply include/exclude filters
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

                // Parse once to extract metadata, then drop the DOM
                let html = Html::parse_document(&content);
                let canonical = extract_canonical(&html);
                let noindex = has_noindex(&html);
                drop(html);

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

fn extract_canonical(html: &Html) -> Option<String> {
    let sel = Selector::parse("link[rel='canonical']").ok()?;
    let element = html.select(&sel).next()?;
    element.value().attr("href").map(|s| s.to_string())
}

fn has_noindex(html: &Html) -> bool {
    let sel = match Selector::parse("meta[name='robots']") {
        Ok(s) => s,
        Err(_) => return false,
    };
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
