use rayon::prelude::*;
use scraper::Selector;
use std::sync::LazyLock;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};

const LEGACY_IMAGE_EXTENSIONS: &[&str] = &[".jpg", ".jpeg", ".png", ".gif"];

static IMG_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("img").expect("valid selector"));

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let img = &config.images;
    if !img.check_missing_dimensions
        && !img.warn_missing_lazy
        && !img.info_missing_srcset
        && !img.format_hints
    {
        return Vec::new();
    }

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();
            let images: Vec<_> = html.select(&IMG_SEL).collect();

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
                        findings.push(Finding::fail("images/missing-dimensions", format!(
                                "Image missing {} attribute (causes CLS): src='{}'",
                                match (has_width, has_height) {
                                    (false, false) => "width and height",
                                    (false, true) => "width",
                                    (true, false) => "height",
                                    _ => unreachable!(),
                                },
                                src
                            ))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector(format!("img[src='{}']", src)))
.with_help("Add explicit width and height attributes to prevent Cumulative Layout Shift. Use <Image> from astro:assets to get them automatically.")
.with_suggestion("width=\"...\" height=\"...\""));
                    }
                }

                // Missing loading="lazy" — skip first image (likely above-fold)
                if img.warn_missing_lazy && i > 0 {
                    let loading = attrs.attr("loading").unwrap_or("");
                    if loading.is_empty() {
                        findings.push(Finding::fail("images/missing-lazy", format!(
                                "Image #{} has no loading attribute: src='{}'",
                                i + 1,
                                src
                            ))
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector(format!("img[src='{}']", src)))
.with_help("Add loading=\"lazy\" to defer off-screen images. Use <Image> from astro:assets to get this automatically.")
.with_suggestion("loading=\"lazy\""));
                    }
                }

                // Missing srcset
                if img.info_missing_srcset && !is_svg {
                    let has_srcset = attrs.attr("srcset").is_some();
                    if !has_srcset {
                        findings.push(Finding::fail("images/missing-srcset", format!(
                                "Image has no srcset (no responsive image markup): src='{}'",
                                src
                            ))
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector(format!("img[src='{}']", src)))
.with_help("Use <Image> or <Picture> from astro:assets to generate responsive srcset automatically."));
                    }
                }

                // Legacy format hints
                if img.format_hints && !src.starts_with("data:") {
                    let src_lower = src.to_lowercase();
                    let is_legacy = LEGACY_IMAGE_EXTENSIONS
                        .iter()
                        .any(|ext| src_lower.ends_with(ext));
                    if is_legacy {
                        findings.push(Finding::fail("images/legacy-format", format!(
                                "Image uses legacy format — consider WebP or AVIF: src='{}'",
                                src
                            ))
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector(format!("img[src='{}']", src)))
.with_help("Use <Image> from astro:assets to automatically convert to WebP/AVIF for better compression."));
                    }
                }
            }

            findings
        })
        .collect()
}
