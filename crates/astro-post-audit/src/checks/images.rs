use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

const LEGACY_IMAGE_EXTENSIONS: &[&str] = &[".jpg", ".jpeg", ".png", ".gif"];

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let img = &config.images;
    if !img.check_missing_dimensions
        && !img.warn_missing_lazy
        && !img.info_missing_srcset
        && !img.format_hints
    {
        return Vec::new();
    }

    let img_sel = Selector::parse("img").unwrap();

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();
            let images: Vec<_> = html.select(&img_sel).collect();

            for (i, el) in images.iter().enumerate() {
                let attrs = el.value();
                let src = attrs.attr("src").unwrap_or("(unknown)");

                // Skip SVG inline or data URIs for dimension checks
                let is_svg = src.starts_with("data:image/svg") || src.ends_with(".svg");

                // Missing width/height → Error (CLS)
                if img.check_missing_dimensions && !is_svg {
                    let has_width = attrs.attr("width").is_some();
                    let has_height = attrs.attr("height").is_some();
                    if !has_width || !has_height {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "images/missing-dimensions".into(),
                            file: page.rel_path.clone(),
                            selector: format!("img[src='{}']", src),
                            message: format!(
                                "Image missing {} attribute (causes CLS): src='{}'",
                                match (has_width, has_height) {
                                    (false, false) => "width and height",
                                    (false, true) => "width",
                                    (true, false) => "height",
                                    _ => unreachable!(),
                                },
                                src
                            ),
                            help: "Add explicit width and height attributes to prevent Cumulative Layout Shift. Use <Image> from astro:assets to get them automatically.".into(),
                            suggestion: Some("width=\"...\" height=\"...\"".into()),
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }

                // Missing loading="lazy" — skip first image (likely above-fold)
                if img.warn_missing_lazy && i > 0 {
                    let loading = attrs.attr("loading").unwrap_or("");
                    if loading.is_empty() {
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "images/missing-lazy".into(),
                            file: page.rel_path.clone(),
                            selector: format!("img[src='{}']", src),
                            message: format!(
                                "Image #{} has no loading attribute: src='{}'",
                                i + 1,
                                src
                            ),
                            help: "Add loading=\"lazy\" to defer off-screen images. Use <Image> from astro:assets to get this automatically.".into(),
                            suggestion: Some("loading=\"lazy\"".into()),
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }

                // Missing srcset
                if img.info_missing_srcset && !is_svg {
                    let has_srcset = attrs.attr("srcset").is_some();
                    if !has_srcset {
                        findings.push(Finding {
                            level: Level::Info,
                            rule_id: "images/missing-srcset".into(),
                            file: page.rel_path.clone(),
                            selector: format!("img[src='{}']", src),
                            message: format!(
                                "Image has no srcset (no responsive image markup): src='{}'",
                                src
                            ),
                            help: "Use <Image> or <Picture> from astro:assets to generate responsive srcset automatically.".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }

                // Legacy format hints
                if img.format_hints && !src.starts_with("data:") {
                    let src_lower = src.to_lowercase();
                    let is_legacy = LEGACY_IMAGE_EXTENSIONS
                        .iter()
                        .any(|ext| src_lower.ends_with(ext));
                    if is_legacy {
                        findings.push(Finding {
                            level: Level::Info,
                            rule_id: "images/legacy-format".into(),
                            file: page.rel_path.clone(),
                            selector: format!("img[src='{}']", src),
                            message: format!(
                                "Image uses legacy format — consider WebP or AVIF: src='{}'",
                                src
                            ),
                            help: "Use <Image> from astro:assets to automatically convert to WebP/AVIF for better compression.".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }
            }

            findings
        })
        .collect()
}
