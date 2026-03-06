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

    let og_title_sel = og
        .require_og_title
        .then(|| Selector::parse("meta[property='og:title']").unwrap());
    let og_desc_sel = og
        .require_og_description
        .then(|| Selector::parse("meta[property='og:description']").unwrap());
    let og_image_sel = og
        .require_og_image
        .then(|| Selector::parse("meta[property='og:image']").unwrap());
    let twitter_sel = og
        .require_twitter_card
        .then(|| Selector::parse("meta[name='twitter:card']").unwrap());

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            if og.require_og_title {
                let has = html
                    .select(
                        og_title_sel
                            .as_ref()
                            .expect("og:title selector initialized"),
                    )
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
                    });
                }
            }

            if og.require_og_description {
                let has = html
                    .select(
                        og_desc_sel
                            .as_ref()
                            .expect("og:description selector initialized"),
                    )
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
                    });
                }
            }

            if og.require_og_image {
                let has = html
                    .select(
                        og_image_sel
                            .as_ref()
                            .expect("og:image selector initialized"),
                    )
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
                        suggestion: None,
                    });
                }
            }

            if og.require_twitter_card {
                let has = html
                    .select(
                        twitter_sel
                            .as_ref()
                            .expect("twitter:card selector initialized"),
                    )
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
                        suggestion: None,
                    });
                }
            }

            findings
        })
        .collect()
}
