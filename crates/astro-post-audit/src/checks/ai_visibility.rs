use rayon::prelude::*;
use regex::Regex;
use scraper::Selector;
use std::sync::LazyLock;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Location, Severity};

/// Elements whose text is code or data, not words a reader sees.
const NON_VISIBLE: &[&str] = &["script", "style", "noscript", "template"];

static H2_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h2").expect("valid selector"));
static H3_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h3").expect("valid selector"));
static ARTICLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("article, section, nav").expect("valid selector"));
static LD_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("script[type='application/ld+json']").expect("valid selector")
});
static META_ROBOTS_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[name='robots']").expect("valid selector"));
static MARKDOWN_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[[^\]]*\]\(([^)\s]+)").expect("valid regex"));

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.ai_visibility.enabled {
        return Vec::new();
    }

    let mut site_findings = Vec::new();

    // Check dist/llms.txt existence and internal link integrity.
    if config.ai_visibility.require_llms_txt {
        let llms_path = index.dist_path.join("llms.txt");
        let llms_full_path = index.dist_path.join("llms-full.txt");
        if !llms_path.exists() && !llms_full_path.exists() {
            site_findings.push(Finding::fail("ai-visibility/missing-llms-txt","No llms.txt or llms-full.txt found in dist/")
.with_severity(Severity::Low)
.at(Location::file("llms.txt").with_selector("root"))
.with_help("llms.txt is a proposed convention that not every AI system reads. If you want to offer one, create public/llms.txt with key site links and summaries."));
        }
        for path in [&llms_path, &llms_full_path] {
            if let Ok(content) = std::fs::read_to_string(path) {
                site_findings.extend(check_llms_links(index, path, &content));
            }
        }
    }

    let page_findings: Vec<Finding> = index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            // === Dimension 1: LLM Readability ===

            // Word count of the visible text: script, style and similar
            // elements hold code or data, not words a reader sees.
            let word_count: usize = html
                .root_element()
                .descendants()
                .filter_map(|node| {
                    let text = node.value().as_text()?;
                    let in_code = node
                        .parent()
                        .and_then(|p| p.value().as_element())
                        .is_some_and(|el| NON_VISIBLE.contains(&el.name()));
                    (!in_code).then(|| text.split_whitespace().count())
                })
                .sum();
            if word_count < 300 {
                findings.push(Finding::review("ai-visibility/low-word-count", format!(
                        "Page has ~{} words of visible text; the 300-word guideline is a rule of thumb and short pages can be intentional",
                        word_count
                    ))
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("If this page is meant to be cited on its own, consider whether it says enough to stand alone."));
            }

            // === Dimension 2: Citability ===
            // (lang, og:title, og:description and canonical are reported by the
            // html_basics, opengraph and seo modules.)

            // Author / publisher schema
            let has_author_schema = html.select(&LD_SEL).any(|script| {
                let content: String = script.text().collect();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    has_author_or_publisher(&json)
                } else {
                    false
                }
            });

            if !has_author_schema {
                findings.push(Finding::review("ai-visibility/missing-author-schema","No author or publisher in JSON-LD; whether a page needs one depends on its type")
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("For articles and other attributed content, add an Article or Person schema with \"author\": {\"@type\": \"Person\", \"name\": \"...\"}. Landing and utility pages often need none."));
            }

            // === Dimension 3: Chunk Quality (RAG) ===

            let semantic_count = html.select(&ARTICLE_SEL).count();
            if semantic_count == 0 {
                findings.push(Finding::review("ai-visibility/no-semantic-sections","No <article>, <section> or <nav> element found; the page has no sectioning markup")
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Sectioning elements give tools that split pages into chunks clear boundaries. A single <main> with headings can be enough."));
            }

            let h2_count = html.select(&H2_SEL).count();
            let h3_count = html.select(&H3_SEL).count();
            if word_count > 600 && h2_count == 0 && h3_count == 0 {
                findings.push(Finding::review("ai-visibility/no-subheadings", format!(
                        "Page has ~{} words of visible text but no H2/H3 subheadings",
                        word_count
                    ))
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Long text without subheadings is harder to scan and to split into sections. Add H2/H3 headings where the content has distinct parts."));
            }

            // === Dimension 5: AI Policy ===

            // noindex check
            let is_noindex = html.select(&META_ROBOTS_SEL).any(|el| {
                el.value()
                    .attr("content")
                    .is_some_and(|c| c.to_lowercase().contains("noindex"))
            });
            if is_noindex {
                findings.push(Finding::fail("ai-visibility/noindex-page","Page has a noindex robots meta tag")
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Remove noindex if this page should be indexed; crawlers that honour it will skip the page."));
            }

            findings
        })
        .collect();

    site_findings.extend(page_findings);
    site_findings
}

fn check_llms_links(index: &SiteIndex, path: &std::path::Path, content: &str) -> Vec<Finding> {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("llms.txt");

    MARKDOWN_LINK_RE
        .captures_iter(content)
        .filter_map(|captures| captures.get(1).map(|capture| capture.as_str()))
        .filter(|href| is_internal_llms_link(href, index))
        .filter(|href| !llms_link_exists(href, index))
        .map(|href| {
            Finding::fail(
                "ai-visibility/llms-txt-broken-link",
                format!("llms.txt links to a missing internal target: '{href}'"),
            )
            .with_severity(Severity::Medium)
            .at(Location::file(file).with_selector("markdown link"))
            .with_help("Update the link or publish the referenced page or asset.")
        })
        .collect()
}

fn is_internal_llms_link(href: &str, index: &SiteIndex) -> bool {
    !href.starts_with('#')
        && !href.starts_with("mailto:")
        && !href.starts_with("tel:")
        && normalize::is_internal(href, index.base_url.as_deref())
}

fn llms_link_exists(href: &str, index: &SiteIndex) -> bool {
    let Some(route) = normalize::resolve_href(href, "/", index.base_url.as_deref()) else {
        return true;
    };
    if index.route_to_index.contains_key(&route) {
        return true;
    }

    index
        .dist_path
        .join(normalize::strip_fragment_and_query(&route).trim_start_matches('/'))
        .exists()
}

fn has_author_or_publisher(json: &serde_json::Value) -> bool {
    if let Some(graph) = json.get("@graph").and_then(|g| g.as_array()) {
        return graph.iter().any(entity_has_author);
    }
    entity_has_author(json)
}

fn entity_has_author(entity: &serde_json::Value) -> bool {
    entity.get("author").is_some()
        || entity.get("publisher").is_some()
        || entity.get("creator").is_some()
}
