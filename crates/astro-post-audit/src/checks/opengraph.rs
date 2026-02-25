use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let og = &config.opengraph;
    if !og.require_og_title
        && !og.require_og_description
        && !og.require_og_image
        && !og.require_twitter_card
    {
        return Vec::new();
    }

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            if og.require_og_title {
                let sel = Selector::parse("meta[property='og:title']").unwrap();
                let has = html
                    .select(&sel)
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
                    });
                }
            }

            if og.require_og_description {
                let sel = Selector::parse("meta[property='og:description']").unwrap();
                let has = html
                    .select(&sel)
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
                    });
                }
            }

            if og.require_og_image {
                let sel = Selector::parse("meta[property='og:image']").unwrap();
                let has = html
                    .select(&sel)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "opengraph/image-missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "Missing og:image meta tag".into(),
                        help: "Add <meta property=\"og:image\" content=\"...\">".into(),
                    });
                }
            }

            if og.require_twitter_card {
                let sel = Selector::parse("meta[name='twitter:card']").unwrap();
                let has = html
                    .select(&sel)
                    .next()
                    .and_then(|el| el.value().attr("content"))
                    .is_some_and(|v| !v.trim().is_empty());
                if !has {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "opengraph/twitter-card-missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "Missing twitter:card meta tag".into(),
                        help: "Add <meta name=\"twitter:card\" content=\"summary_large_image\">"
                            .into(),
                    });
                }
            }

            findings
        })
        .collect()
}
