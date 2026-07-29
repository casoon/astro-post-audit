use rayon::prelude::*;
use regex::Regex;
use scraper::Selector;
use std::sync::LazyLock;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

static H2_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h2").expect("valid selector"));
static H3_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h3").expect("valid selector"));
static ARTICLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("article, section, nav").expect("valid selector"));
static OG_TITLE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:title']").expect("valid selector"));
static OG_DESC_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[property='og:description']").expect("valid selector"));
static CANONICAL_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("link[rel='canonical']").expect("valid selector"));
static LD_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("script[type='application/ld+json']").expect("valid selector")
});
static LANG_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("html[lang]").expect("valid selector"));
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
            site_findings.push(Finding {
                level: Level::Info,
                rule_id: "ai-visibility/missing-llms-txt".into(),
                file: "llms.txt".into(),
                selector: "root".into(),
                message: "No llms.txt or llms-full.txt found in dist/ — AI crawlers use this file for site context".into(),
                help: "Create public/llms.txt with key site links and summaries for LLM crawlers.".into(),
                suggestion: None,
                source_hint: None,
                confidence: None,
            });
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

            // Word count
            let body_text = html
                .root_element()
                .text()
                .collect::<String>();
            let word_count = body_text.split_whitespace().count();
            if word_count < 300 {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "ai-visibility/low-word-count".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: format!(
                        "Page has only ~{} words — AI systems prefer content-rich pages (300+ words)",
                        word_count
                    ),
                    help: "Add more substantive content to improve AI citation probability.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            // lang attribute
            if html.select(&LANG_SEL).next().is_none() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "ai-visibility/lang-missing".into(),
                    file: page.rel_path.clone(),
                    selector: "html".into(),
                    message: "Missing lang attribute on <html> — AI systems use language signals for relevance".into(),
                    help: "Add lang=\"en\" (or your language) to the <html> element.".into(),
                    suggestion: Some("lang=\"en\"".into()),
                    source_hint: None,
                    confidence: None,
                });
            }

            // === Dimension 2: Citability ===

            let has_og_title = html
                .select(&OG_TITLE_SEL)
                .next()
                .and_then(|el| el.value().attr("content"))
                .is_some_and(|v| !v.trim().is_empty());

            let has_og_desc = html
                .select(&OG_DESC_SEL)
                .next()
                .and_then(|el| el.value().attr("content"))
                .is_some_and(|v| !v.trim().is_empty());

            let has_canonical = html
                .select(&CANONICAL_SEL)
                .next()
                .and_then(|el| el.value().attr("href"))
                .is_some_and(|v| !v.trim().is_empty());

            if !has_og_title {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "ai-visibility/missing-og-title".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Missing og:title — AI systems use this as the citation title".into(),
                    help: "Add <meta property=\"og:title\" content=\"...\"> for better AI citations.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            if !has_og_desc {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "ai-visibility/missing-og-description".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Missing og:description — AI systems use this as the citation snippet".into(),
                    help: "Add <meta property=\"og:description\" content=\"...\"> for AI citation snippets.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            if !has_canonical {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "ai-visibility/missing-canonical".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Missing canonical URL — AI systems need a definitive URL for citations".into(),
                    help: "Add <link rel=\"canonical\" href=\"https://...\"> to each page.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

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
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "ai-visibility/missing-author-schema".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "No author or publisher in JSON-LD — reduces AI citation authority".into(),
                    help: "Add an Article or Person schema with \"author\": {\"@type\": \"Person\", \"name\": \"...\"}".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            // === Dimension 3: Chunk Quality (RAG) ===

            let semantic_count = html.select(&ARTICLE_SEL).count();
            if semantic_count == 0 {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "ai-visibility/no-semantic-sections".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: "No <article> or <section> elements found — semantic HTML improves AI chunking".into(),
                    help: "Wrap main content in <article> or <section> elements for better RAG embedding.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            let h2_count = html.select(&H2_SEL).count();
            let h3_count = html.select(&H3_SEL).count();
            if word_count > 600 && h2_count == 0 && h3_count == 0 {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "ai-visibility/no-subheadings".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: format!(
                        "Page has {} words but no H2/H3 subheadings — limits AI content chunking",
                        word_count
                    ),
                    help: "Add H2/H3 headings to structure long content for better AI comprehension and RAG chunking.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            // === Dimension 5: AI Policy ===

            // noindex check
            let is_noindex = html.select(&META_ROBOTS_SEL).any(|el| {
                el.value()
                    .attr("content")
                    .is_some_and(|c| c.to_lowercase().contains("noindex"))
            });
            if is_noindex {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "ai-visibility/noindex-page".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Page has noindex — AI crawlers may also skip this page".into(),
                    help: "Remove noindex if you want AI systems to index and cite this page.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
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
        .map(|href| Finding {
            level: Level::Warning,
            rule_id: "ai-visibility/llms-txt-broken-link".into(),
            file: file.into(),
            selector: "markdown link".into(),
            message: format!("llms.txt links to a missing internal target: '{href}'"),
            help: "Update the link or publish the referenced page or asset.".into(),
            suggestion: None,
            source_hint: None,
            confidence: None,
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
