use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.ai_visibility.enabled {
        return Vec::new();
    }

    let h2_sel = Selector::parse("h2").unwrap();
    let h3_sel = Selector::parse("h3").unwrap();
    let article_sel = Selector::parse("article, section, nav").unwrap();
    let og_title_sel = Selector::parse("meta[property='og:title']").unwrap();
    let og_desc_sel = Selector::parse("meta[property='og:description']").unwrap();
    let canonical_sel = Selector::parse("link[rel='canonical']").unwrap();
    let ld_sel = Selector::parse("script[type='application/ld+json']").unwrap();
    let lang_sel = Selector::parse("html[lang]").unwrap();
    let meta_robots_sel = Selector::parse("meta[name='robots']").unwrap();

    index
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
            if html.select(&lang_sel).next().is_none() {
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
                .select(&og_title_sel)
                .next()
                .and_then(|el| el.value().attr("content"))
                .is_some_and(|v| !v.trim().is_empty());

            let has_og_desc = html
                .select(&og_desc_sel)
                .next()
                .and_then(|el| el.value().attr("content"))
                .is_some_and(|v| !v.trim().is_empty());

            let has_canonical = html
                .select(&canonical_sel)
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
            let has_author_schema = html.select(&ld_sel).any(|script| {
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

            let semantic_count = html.select(&article_sel).count();
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

            let h2_count = html.select(&h2_sel).count();
            let h3_count = html.select(&h3_sel).count();
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
            let is_noindex = html.select(&meta_robots_sel).any(|el| {
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
        .collect()
}

fn has_author_or_publisher(json: &serde_json::Value) -> bool {
    if let Some(graph) = json.get("@graph").and_then(|g| g.as_array()) {
        return graph.iter().any(entity_has_author);
    }
    entity_has_author(json)
}

fn entity_has_author(entity: &serde_json::Value) -> bool {
    entity.get("author").is_some() || entity.get("publisher").is_some() || entity.get("creator").is_some()
}
