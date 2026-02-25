use rayon::prelude::*;
use scraper::Selector;
use std::path::Path;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();

    if config.assets.check_broken_assets {
        findings.extend(check_broken_assets(index, config));
    }

    if config.assets.max_image_size_kb.is_some()
        || config.assets.max_js_size_kb.is_some()
        || config.assets.max_css_size_kb.is_some()
    {
        findings.extend(check_asset_sizes(index, config));
    }

    findings
}

fn check_broken_assets(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            // Check img[src]
            let img_sel = Selector::parse("img[src]").unwrap();
            for el in html.select(&img_sel) {
                if let Some(src) = el.value().attr("src") {
                    if should_check_asset(src) {
                        check_asset_exists(
                            &index.dist_path,
                            src,
                            &page.rel_path,
                            "img[src]",
                            &mut findings,
                        );
                    }
                }
            }

            // Check script[src]
            let script_sel = Selector::parse("script[src]").unwrap();
            for el in html.select(&script_sel) {
                if let Some(src) = el.value().attr("src") {
                    if should_check_asset(src) {
                        check_asset_exists(
                            &index.dist_path,
                            src,
                            &page.rel_path,
                            "script[src]",
                            &mut findings,
                        );
                    }
                }
            }

            // Check link[href] (stylesheets)
            let link_sel = Selector::parse("link[rel='stylesheet'][href]").unwrap();
            for el in html.select(&link_sel) {
                if let Some(href) = el.value().attr("href") {
                    if should_check_asset(href) {
                        check_asset_exists(
                            &index.dist_path,
                            href,
                            &page.rel_path,
                            "link[href]",
                            &mut findings,
                        );
                    }
                }
            }

            // Check source[srcset] / img[srcset]
            let srcset_sel = Selector::parse("[srcset]").unwrap();
            for el in html.select(&srcset_sel) {
                if let Some(srcset) = el.value().attr("srcset") {
                    for entry in srcset.split(',') {
                        let src = entry.split_whitespace().next().unwrap_or("");
                        if !src.is_empty() && should_check_asset(src) {
                            check_asset_exists(
                                &index.dist_path,
                                src,
                                &page.rel_path,
                                "srcset",
                                &mut findings,
                            );
                        }
                    }
                }
            }

            // Check img width/height for CLS
            if config.assets.check_image_dimensions {
                let img_all = Selector::parse("img").unwrap();
                for el in html.select(&img_all) {
                    let has_width = el.value().attr("width").is_some();
                    let has_height = el.value().attr("height").is_some();
                    if !has_width || !has_height {
                        let src = el.value().attr("src").unwrap_or("(unknown)");
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "assets/img-dimensions".into(),
                            file: page.rel_path.clone(),
                            selector: format!("img[src='{}']", src),
                            message: format!(
                                "Image missing width/height attributes: src='{}'",
                                src
                            ),
                            help: "Add explicit width and height to prevent layout shift (CLS)"
                                .into(),
                        });
                    }
                }
            }

            findings
        })
        .collect()
}

fn should_check_asset(src: &str) -> bool {
    // Skip external URLs, data URIs, protocol-relative
    !src.starts_with("http://")
        && !src.starts_with("https://")
        && !src.starts_with("//")
        && !src.starts_with("data:")
}

fn check_asset_exists(
    dist_path: &Path,
    src: &str,
    page_file: &str,
    selector_hint: &str,
    findings: &mut Vec<Finding>,
) {
    let clean = src.split('?').next().unwrap_or(src);
    let clean = clean.split('#').next().unwrap_or(clean);
    let asset_path = if clean.starts_with('/') {
        dist_path.join(clean.trim_start_matches('/'))
    } else {
        // Relative to page directory
        let page_dir = Path::new(page_file).parent().unwrap_or(Path::new(""));
        dist_path.join(page_dir).join(clean)
    };

    if !asset_path.exists() {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "assets/broken".into(),
            file: page_file.to_string(),
            selector: format!("{}='{}'", selector_hint, src),
            message: format!("Broken asset reference: '{}'", src),
            help: "Fix the path or add the missing asset file".into(),
        });
    }
}

fn check_asset_sizes(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    use walkdir::WalkDir;

    let mut findings = Vec::new();

    for entry in WalkDir::new(&index.dist_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let size_kb = entry.metadata().map(|m| m.len() / 1024).unwrap_or(0);
        let rel = path
            .strip_prefix(&index.dist_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "svg" => {
                if let Some(max) = config.assets.max_image_size_kb {
                    if size_kb > max {
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "assets/large-image".into(),
                            file: rel,
                            selector: String::new(),
                            message: format!("Image is {}KB (max: {}KB)", size_kb, max),
                            help: "Optimize/compress the image or use a more efficient format"
                                .into(),
                        });
                    }
                }
            }
            "js" | "mjs" => {
                if let Some(max) = config.assets.max_js_size_kb {
                    if size_kb > max {
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "assets/large-js".into(),
                            file: rel,
                            selector: String::new(),
                            message: format!("JavaScript file is {}KB (max: {}KB)", size_kb, max),
                            help: "Consider code splitting or tree-shaking to reduce bundle size"
                                .into(),
                        });
                    }
                }
            }
            "css" => {
                if let Some(max) = config.assets.max_css_size_kb {
                    if size_kb > max {
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "assets/large-css".into(),
                            file: rel,
                            selector: String::new(),
                            message: format!("CSS file is {}KB (max: {}KB)", size_kb, max),
                            help: "Consider splitting CSS or removing unused styles".into(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    findings
}
