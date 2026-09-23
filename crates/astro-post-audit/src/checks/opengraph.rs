use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use rayon::prelude::*;
use scraper::Selector;
use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};

static OG_TITLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:title']").expect("valid selector"));
static OG_DESC_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:description']").expect("valid selector"));
static OG_IMAGE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:image']").expect("valid selector"));
static OG_TYPE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:type']").expect("valid selector"));
static OG_URL_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:url']").expect("valid selector"));
static TWITTER_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[name='twitter:card']").expect("valid selector"));
static TWITTER_IMAGE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[name='twitter:image']").expect("valid selector"));
static TITLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("title").expect("valid selector"));

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

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            if og.require_og_title {
                let has = html
                    .select(&OG_TITLE_SEL)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding::fail("opengraph/title-missing","Missing og:title meta tag")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Add <meta property=\"og:title\" content=\"...\">"));
                }
            }

            if og.require_og_description {
                let has = html
                    .select(&OG_DESC_SEL)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding::fail("opengraph/description-missing","Missing og:description meta tag")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Add <meta property=\"og:description\" content=\"...\">"));
                }
            }

            // og:image — existence check + absolute URL validation
            let og_image_content = html
                .select(&OG_IMAGE_SEL)
                .next()
                .and_then(|el| el.value().attr("content"))
                .map(|v| v.trim().to_string());

            if og.require_og_image && og_image_content.is_none() {
                findings.push(Finding::fail("opengraph/image-missing","Missing og:image meta tag")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Add <meta property=\"og:image\" content=\"https://...\">"));
            }

            if og.og_image_absolute_url {
                if let Some(ref img_url) = og_image_content {
                    if !img_url.is_empty()
                        && !img_url.starts_with("https://")
                        && !img_url.starts_with("http://")
                    {
                        findings.push(Finding::fail("opengraph/image-not-absolute", format!(
                                "og:image URL is not absolute: \"{}\"",
                                img_url
                            ))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector("meta[property='og:image']"))
.with_help("og:image must be an absolute URL (https://...) so social platforms can fetch it"));
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
                                    findings.push(Finding::fail("opengraph/image-broken", format!(
                                            "og:image '{}' does not exist in the build output",
                                            img_url
                                        ))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector("meta[property='og:image']"))
.with_help("Fix the og:image path or add the missing image file."));
                                }
                            } else {
                                if og.check_image_dimensions {
                                    if let Ok(dim) = imagesize::size(&local_path) {
                                        if dim.width < OG_IMAGE_REC_WIDTH
                                            || dim.height < OG_IMAGE_REC_HEIGHT
                                        {
                                            findings.push(Finding::fail("opengraph/image-invalid-dimensions", format!(
                                                    "og:image is {}x{}, below the recommended {}x{}",
                                                    dim.width, dim.height,
                                                    OG_IMAGE_REC_WIDTH, OG_IMAGE_REC_HEIGHT
                                                ))
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("meta[property='og:image']"))
.with_help("Use a 1200x630 image so social platforms render a large preview card."));
                                        }
                                    }
                                }
                                if let Some(max_kb) = og.og_image_max_size_kb {
                                    let size_kb = std::fs::metadata(&local_path)
                                        .map(|m| m.len() / 1024)
                                        .unwrap_or(0);
                                    if size_kb > max_kb {
                                        findings.push(Finding::fail("opengraph/image-too-large", format!(
                                                "og:image is {}KB (max: {}KB)",
                                                size_kb, max_kb
                                            ))
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("meta[property='og:image']"))
.with_help("Compress the social preview image to keep it small and fast to fetch."));
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
                    .select(&OG_TYPE_SEL)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding::fail("opengraph/type-missing","Missing og:type meta tag")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Add <meta property=\"og:type\" content=\"website\"> (or \"article\", \"product\", etc.)")
.with_suggestion("<meta property=\"og:type\" content=\"website\">"));
                }
            }

            // og:url
            if og.require_og_url {
                let has = html
                    .select(&OG_URL_SEL)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding::fail("opengraph/url-missing","Missing og:url meta tag")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Add <meta property=\"og:url\" content=\"https://...\"> with the canonical URL"));
                }
            }

            // twitter:card — existence + value validation
            let twitter_card_content = html
                .select(&TWITTER_SEL)
                .next()
                .and_then(|el| el.value().attr("content"))
                .map(|v| v.trim().to_string());

            if og.require_twitter_card && twitter_card_content.is_none() {
                findings.push(Finding::fail("opengraph/twitter-card-missing","Missing twitter:card meta tag")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Add <meta name=\"twitter:card\" content=\"summary_large_image\">"));
            }

            if og.twitter_card_valid_values {
                if let Some(ref card_val) = twitter_card_content {
                    if !card_val.is_empty() && !VALID_TWITTER_CARD_VALUES.contains(&card_val.as_str()) {
                        findings.push(Finding::fail("opengraph/twitter-card-invalid", format!(
                                "Invalid twitter:card value \"{}\". Allowed: {}",
                                card_val,
                                VALID_TWITTER_CARD_VALUES.join(", ")
                            ))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector("meta[name='twitter:card']"))
.with_help("Use one of: summary, summary_large_image, app, player")
.with_suggestion("summary_large_image"));
                    }
                }
            }

            // twitter:image
            if og.require_twitter_image {
                let has = html
                    .select(&TWITTER_IMAGE_SEL)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding::fail("opengraph/twitter-image-missing","Missing twitter:image meta tag")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Add <meta name=\"twitter:image\" content=\"https://...\">"));
                }
            }

            // og:title ≈ <title> consistency
            if og.og_title_consistency {
                let og_title_val = html
                    .select(&OG_TITLE_SEL)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let page_title = html
                    .select(&TITLE_SEL)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_else(String::new);
                let page_title = page_title.trim();

                if !og_title_val.is_empty() && !page_title.is_empty() {
                    let og_len = og_title_val.chars().count();
                    let title_len = page_title.chars().count();
                    let max_len = og_len.max(title_len);
                    let diff = og_len.abs_diff(title_len);
                    // Warn if length difference is >50% of the longer title
                    if max_len > 0 && diff * 2 > max_len {
                        findings.push(Finding::fail("opengraph/title-inconsistent", format!(
                                "og:title ({} chars) and <title> ({} chars) differ significantly",
                                og_len, title_len
                            ))
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("meta[property='og:title']"))
.with_help("Keep og:title and <title> similar for consistent sharing previews"));
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
