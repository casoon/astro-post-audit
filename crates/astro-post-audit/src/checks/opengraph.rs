use std::path::{Path, PathBuf};

use rayon::prelude::*;
use scraper::Selector;
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

const VALID_TWITTER_CARD_VALUES: &[&str] = &["summary", "summary_large_image", "app", "player"];

/// Recommended Open Graph image dimensions (Facebook/Twitter large card).
const OG_IMAGE_REC_WIDTH: usize = 1200;
const OG_IMAGE_REC_HEIGHT: usize = 630;

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let og = &config.opengraph;
    let any_active = og.require_og_title
        || og.require_og_description
        || og.require_og_image
        || og.require_twitter_card
        || og.require_og_type
        || og.require_og_url
        || og.og_image_absolute_url
        || og.require_twitter_image
        || og.twitter_card_valid_values
        || og.og_title_consistency
        || og.check_image_exists
        || og.check_image_dimensions
        || og.og_image_max_size_kb.is_some();
    if !any_active {
        return Vec::new();
    }

    let og_title_sel = Selector::parse("meta[property='og:title']").unwrap();
    let og_desc_sel = Selector::parse("meta[property='og:description']").unwrap();
    let og_image_sel = Selector::parse("meta[property='og:image']").unwrap();
    let og_type_sel = Selector::parse("meta[property='og:type']").unwrap();
    let og_url_sel = Selector::parse("meta[property='og:url']").unwrap();
    let twitter_sel = Selector::parse("meta[name='twitter:card']").unwrap();
    let twitter_image_sel = Selector::parse("meta[name='twitter:image']").unwrap();
    let title_sel = Selector::parse("title").unwrap();

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            if og.require_og_title {
                let has = html
                    .select(&og_title_sel)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "opengraph/title-missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "Missing og:title meta tag".into(),
                        help: "Add <meta property=\"og:title\" content=\"...\">".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
            }

            if og.require_og_description {
                let has = html
                    .select(&og_desc_sel)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "opengraph/description-missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "Missing og:description meta tag".into(),
                        help: "Add <meta property=\"og:description\" content=\"...\">".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
            }

            // og:image — existence check + absolute URL validation
            let og_image_content = html
                .select(&og_image_sel)
                .next()
                .and_then(|el| el.value().attr("content"))
                .map(|v| v.trim().to_string());

            if og.require_og_image && og_image_content.is_none() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "opengraph/image-missing".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Missing og:image meta tag".into(),
                    help: "Add <meta property=\"og:image\" content=\"https://...\">".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            if og.og_image_absolute_url {
                if let Some(ref img_url) = og_image_content {
                    if !img_url.is_empty()
                        && !img_url.starts_with("https://")
                        && !img_url.starts_with("http://")
                    {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "opengraph/image-not-absolute".into(),
                            file: page.rel_path.clone(),
                            selector: "meta[property='og:image']".into(),
                            message: format!(
                                "og:image URL is not absolute: \"{}\"",
                                img_url
                            ),
                            help: "og:image must be an absolute URL (https://...) so social platforms can fetch it".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }
            }

            // og:image — local file existence, dimensions, and weight
            if og.check_image_exists
                || og.check_image_dimensions
                || og.og_image_max_size_kb.is_some()
            {
                if let Some(ref img_url) = og_image_content {
                    if !img_url.is_empty() {
                        if let Some(local_path) =
                            resolve_local_og_image(img_url, &page.rel_path, index)
                        {
                            if !local_path.exists() {
                                if og.check_image_exists {
                                    findings.push(Finding {
                                        level: Level::Error,
                                        rule_id: "opengraph/image-broken".into(),
                                        file: page.rel_path.clone(),
                                        selector: "meta[property='og:image']".into(),
                                        message: format!(
                                            "og:image '{}' does not exist in the build output",
                                            img_url
                                        ),
                                        help: "Fix the og:image path or add the missing image file.".into(),
                                        suggestion: None,
                                        source_hint: None,
                                        confidence: None,
                                    });
                                }
                            } else {
                                if og.check_image_dimensions {
                                    if let Ok(dim) = imagesize::size(&local_path) {
                                        if dim.width < OG_IMAGE_REC_WIDTH
                                            || dim.height < OG_IMAGE_REC_HEIGHT
                                        {
                                            findings.push(Finding {
                                                level: Level::Warning,
                                                rule_id: "opengraph/image-invalid-dimensions".into(),
                                                file: page.rel_path.clone(),
                                                selector: "meta[property='og:image']".into(),
                                                message: format!(
                                                    "og:image is {}x{}, below the recommended {}x{}",
                                                    dim.width, dim.height,
                                                    OG_IMAGE_REC_WIDTH, OG_IMAGE_REC_HEIGHT
                                                ),
                                                help: "Use a 1200x630 image so social platforms render a large preview card.".into(),
                                                suggestion: None,
                                                source_hint: None,
                                                confidence: None,
                                            });
                                        }
                                    }
                                }
                                if let Some(max_kb) = og.og_image_max_size_kb {
                                    let size_kb = std::fs::metadata(&local_path)
                                        .map(|m| m.len() / 1024)
                                        .unwrap_or(0);
                                    if size_kb > max_kb {
                                        findings.push(Finding {
                                            level: Level::Warning,
                                            rule_id: "opengraph/image-too-large".into(),
                                            file: page.rel_path.clone(),
                                            selector: "meta[property='og:image']".into(),
                                            message: format!(
                                                "og:image is {}KB (max: {}KB)",
                                                size_kb, max_kb
                                            ),
                                            help: "Compress the social preview image to keep it small and fast to fetch.".into(),
                                            suggestion: None,
                                            source_hint: None,
                                            confidence: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // og:type
            if og.require_og_type {
                let has = html
                    .select(&og_type_sel)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "opengraph/type-missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "Missing og:type meta tag".into(),
                        help: "Add <meta property=\"og:type\" content=\"website\"> (or \"article\", \"product\", etc.)".into(),
                        suggestion: Some("<meta property=\"og:type\" content=\"website\">".into()),
                        source_hint: None,
                        confidence: None,
                    });
                }
            }

            // og:url
            if og.require_og_url {
                let has = html
                    .select(&og_url_sel)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "opengraph/url-missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "Missing og:url meta tag".into(),
                        help: "Add <meta property=\"og:url\" content=\"https://...\"> with the canonical URL".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
            }

            // twitter:card — existence + value validation
            let twitter_card_content = html
                .select(&twitter_sel)
                .next()
                .and_then(|el| el.value().attr("content"))
                .map(|v| v.trim().to_string());

            if og.require_twitter_card && twitter_card_content.is_none() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "opengraph/twitter-card-missing".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Missing twitter:card meta tag".into(),
                    help: "Add <meta name=\"twitter:card\" content=\"summary_large_image\">".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            if og.twitter_card_valid_values {
                if let Some(ref card_val) = twitter_card_content {
                    if !card_val.is_empty() && !VALID_TWITTER_CARD_VALUES.contains(&card_val.as_str()) {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "opengraph/twitter-card-invalid".into(),
                            file: page.rel_path.clone(),
                            selector: "meta[name='twitter:card']".into(),
                            message: format!(
                                "Invalid twitter:card value \"{}\". Allowed: {}",
                                card_val,
                                VALID_TWITTER_CARD_VALUES.join(", ")
                            ),
                            help: "Use one of: summary, summary_large_image, app, player".into(),
                            suggestion: Some("summary_large_image".into()),
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }
            }

            // twitter:image
            if og.require_twitter_image {
                let has = html
                    .select(&twitter_image_sel)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "opengraph/twitter-image-missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "Missing twitter:image meta tag".into(),
                        help: "Add <meta name=\"twitter:image\" content=\"https://...\">".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
            }

            // og:title ≈ <title> consistency
            if og.og_title_consistency {
                let og_title_val = html
                    .select(&og_title_sel)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let page_title = html
                    .select(&title_sel)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();
                let page_title = page_title.trim();

                if !og_title_val.is_empty() && !page_title.is_empty() {
                    let og_len = og_title_val.chars().count();
                    let title_len = page_title.chars().count();
                    let max_len = og_len.max(title_len);
                    let diff = og_len.abs_diff(title_len);
                    // Warn if length difference is >50% of the longer title
                    if max_len > 0 && diff * 2 > max_len {
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "opengraph/title-inconsistent".into(),
                            file: page.rel_path.clone(),
                            selector: "meta[property='og:title']".into(),
                            message: format!(
                                "og:title ({} chars) and <title> ({} chars) differ significantly",
                                og_len, title_len
                            ),
                            help: "Keep og:title and <title> similar for consistent sharing previews".into(),
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

/// Resolve an og:image value to a local file path inside dist, or None if the
/// image is hosted on an external origin (which can't be verified statically).
fn resolve_local_og_image(img_url: &str, page_rel: &str, index: &SiteIndex) -> Option<PathBuf> {
    let dist = &index.dist_path;

    if let Ok(parsed) = Url::parse(img_url) {
        // Absolute URL: only resolvable if it points at our own configured site.
        let host = parsed.host_str()?;
        let base_host = index
            .base_url
            .as_deref()
            .and_then(|b| Url::parse(b).ok())
            .and_then(|u| u.host_str().map(|h| h.to_string()))?;
        if host != base_host {
            return None;
        }
        let path = parsed.path().trim_start_matches('/');
        return Some(dist.join(path));
    }

    // Relative reference.
    let clean = img_url.split('?').next().unwrap_or(img_url);
    let clean = clean.split('#').next().unwrap_or(clean);
    if clean.starts_with('/') {
        Some(dist.join(clean.trim_start_matches('/')))
    } else {
        let page_dir = Path::new(page_rel).parent().unwrap_or(Path::new(""));
        Some(dist.join(page_dir).join(clean))
    }
}
