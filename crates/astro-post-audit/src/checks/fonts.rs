use rayon::prelude::*;
use scraper::Selector;
use std::path::Path;
use std::sync::LazyLock;

use crate::config::Config;
use crate::discovery::{PageInfo, SiteIndex};
use crate::normalize;
use crate::report::{Finding, Level};

static STYLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("style").expect("valid selector"));
static FONT_PRELOAD_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("head link[rel='preload'][as='font']").expect("valid selector")
});
static LINK_STYLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("head link[rel='stylesheet'][href]").expect("valid selector"));

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.fonts.enabled {
        return Vec::new();
    }

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let html = page.parse_html();
            let css_sources = css_sources(index, page, &html);
            let mut findings = Vec::new();

            if config.fonts.check_font_display
                && css_sources
                    .iter()
                    .any(|css| has_font_face_without_display(css))
            {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "fonts/missing-font-display".into(),
                    file: page.rel_path.clone(),
                    selector: "@font-face".into(),
                    message: "An @font-face block is missing font-display (e.g., font-display: swap)"
                        .into(),
                    help: "Add font-display: swap (or optional) to avoid invisible text during font loading (FOIT).".into(),
                    suggestion: Some("font-display: swap;".into()),
                    source_hint: None,
                    confidence: None,
                });
            }

            if config.fonts.require_font_preload
                && css_sources.iter().any(|css| has_self_hosted_font_face(css))
                && html.select(&FONT_PRELOAD_SEL).next().is_none()
            {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "fonts/missing-preload".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Page uses self-hosted webfonts but has no critical font preload in <head>"
                        .into(),
                    help: "Consider adding <link rel=\"preload\" href=\"/fonts/font.woff2\" as=\"font\" type=\"font/woff2\" crossorigin> for critical webfonts.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            findings
        })
        .collect()
}

fn css_sources(index: &SiteIndex, page: &PageInfo, html: &scraper::Html) -> Vec<String> {
    let mut sources: Vec<String> = html
        .select(&STYLE_SEL)
        .map(|style| style.text().collect())
        .collect();

    for stylesheet in html.select(&LINK_STYLE_SEL) {
        let Some(href) = stylesheet.value().attr("href") else {
            continue;
        };
        let Some(path) = local_asset_path(&index.dist_path, &page.rel_path, href) else {
            continue;
        };
        if let Ok(css) = std::fs::read_to_string(path) {
            sources.push(css);
        }
    }

    sources
}

fn local_asset_path(dist_path: &Path, page_file: &str, href: &str) -> Option<std::path::PathBuf> {
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("//")
        || href.starts_with("data:")
    {
        return None;
    }

    let clean = normalize::strip_fragment_and_query(href);
    let candidate = if clean.starts_with('/') {
        dist_path.join(clean.trim_start_matches('/'))
    } else {
        let page_dir = Path::new(page_file)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        dist_path.join(page_dir).join(clean)
    };
    let canonical = candidate.canonicalize().ok()?;
    canonical.starts_with(dist_path).then_some(canonical)
}

fn has_font_face_without_display(css: &str) -> bool {
    font_face_blocks(css).any(|block| !block.to_ascii_lowercase().contains("font-display"))
}

fn has_self_hosted_font_face(css: &str) -> bool {
    font_face_blocks(css).any(|block| {
        block
            .split("url(")
            .skip(1)
            .filter_map(|part| part.split_once(')'))
            .map(|(url, _)| url.trim().trim_matches(['\'', '"']))
            .any(is_self_hosted_font_url)
    })
}

fn font_face_blocks(css: &str) -> impl Iterator<Item = &str> {
    css.match_indices("@font-face")
        .filter_map(move |(start, _)| {
            let after_name = &css[start..];
            let open = after_name.find('{')?;
            let block_start = start + open + 1;
            let block_end = css[block_start..].find('}')? + block_start;
            Some(&css[block_start..block_end])
        })
}

fn is_self_hosted_font_url(url: &str) -> bool {
    let clean = normalize::strip_fragment_and_query(url);
    !clean.starts_with("http://")
        && !clean.starts_with("https://")
        && !clean.starts_with("//")
        && !clean.starts_with("data:")
        && matches!(
            clean.rsplit_once('.').map(|(_, extension)| extension),
            Some("woff") | Some("woff2") | Some("ttf") | Some("otf")
        )
}

#[cfg(test)]
mod tests {
    use super::{has_font_face_without_display, has_self_hosted_font_face};

    #[test]
    fn checks_each_font_face_block() {
        let css = "@font-face { src: url('/one.woff2'); font-display: swap; } @font-face { src: url('/two.woff2'); }";
        assert!(has_font_face_without_display(css));
    }

    #[test]
    fn distinguishes_self_hosted_font_urls() {
        assert!(has_self_hosted_font_face(
            "@font-face { src: url('/font.woff2'); }"
        ));
        assert!(!has_self_hosted_font_face(
            "@font-face { src: url('https://fonts.example/font.woff2'); }"
        ));
    }
}
