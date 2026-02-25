use rayon::prelude::*;
use scraper::{Html, Selector};

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            // lang attribute
            if config.html_basics.lang_attr_required {
                check_lang(page, &html, &mut findings);
            }

            // title tag
            if config.html_basics.title_required {
                check_title(page, &html, config, &mut findings);
            }

            // meta description
            if config.html_basics.meta_description_required {
                check_meta_description(page, &html, config, &mut findings);
            }

            // viewport
            if config.html_basics.viewport_required {
                check_viewport(page, &html, &mut findings);
            }

            findings
        })
        .collect()
}

fn check_lang(page: &crate::discovery::PageInfo, html: &Html, findings: &mut Vec<Finding>) {
    let sel = match Selector::parse("html[lang]") {
        Ok(s) => s,
        Err(_) => return,
    };

    let has_lang = html.select(&sel).next().is_some_and(|el| {
        el.value()
            .attr("lang")
            .is_some_and(|v| !v.trim().is_empty())
    });

    if !has_lang {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "html/lang-missing".into(),
            file: page.rel_path.clone(),
            selector: "html".into(),
            message: "Missing lang attribute on <html> element".into(),
            help: "Add lang attribute, e.g., <html lang=\"en\">".into(),
        });
    }
}

fn check_title(
    page: &crate::discovery::PageInfo,
    html: &Html,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    let sel = match Selector::parse("title") {
        Ok(s) => s,
        Err(_) => return,
    };

    let title_el = html.select(&sel).next();
    match title_el {
        None => {
            findings.push(Finding {
                level: Level::Error,
                rule_id: "html/title-missing".into(),
                file: page.rel_path.clone(),
                selector: "head".into(),
                message: "Missing <title> tag".into(),
                help: "Add a <title> tag inside <head>".into(),
            });
        }
        Some(el) => {
            let text: String = el.text().collect();
            let trimmed = text.trim();
            if trimmed.is_empty() {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "html/title-empty".into(),
                    file: page.rel_path.clone(),
                    selector: "title".into(),
                    message: "Title tag is empty".into(),
                    help: "Add descriptive text to the <title> tag".into(),
                });
            } else if let Some(max) = config.html_basics.title_max_length {
                if trimmed.len() > max {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "html/title-too-long".into(),
                        file: page.rel_path.clone(),
                        selector: "title".into(),
                        message: format!(
                            "Title is {} chars (recommended max: {})",
                            trimmed.len(),
                            max
                        ),
                        help: "Shorten the title for better display in search results".into(),
                    });
                }
            }
        }
    }
}

fn check_meta_description(
    page: &crate::discovery::PageInfo,
    html: &Html,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    let sel = match Selector::parse("meta[name='description']") {
        Ok(s) => s,
        Err(_) => return,
    };

    match html.select(&sel).next() {
        None => {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "html/meta-description-missing".into(),
                file: page.rel_path.clone(),
                selector: "head".into(),
                message: "Missing or empty meta description".into(),
                help: "Add <meta name=\"description\" content=\"...\"> to <head>".into(),
            });
        }
        Some(el) => {
            let content = el.value().attr("content").unwrap_or("");
            let trimmed = content.trim();
            if trimmed.is_empty() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "html/meta-description-missing".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Missing or empty meta description".into(),
                    help: "Add <meta name=\"description\" content=\"...\"> to <head>".into(),
                });
            } else if let Some(max) = config.html_basics.meta_description_max_length {
                if trimmed.len() > max {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "html/meta-description-too-long".into(),
                        file: page.rel_path.clone(),
                        selector: "meta[name='description']".into(),
                        message: format!(
                            "Meta description is {} chars (recommended max: {})",
                            trimmed.len(),
                            max
                        ),
                        help: "Shorten the description for better display in search results".into(),
                    });
                }
            }
        }
    }
}

fn check_viewport(page: &crate::discovery::PageInfo, html: &Html, findings: &mut Vec<Finding>) {
    let sel = match Selector::parse("meta[name='viewport']") {
        Ok(s) => s,
        Err(_) => return,
    };

    if html.select(&sel).next().is_none() {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "html/viewport-missing".into(),
            file: page.rel_path.clone(),
            selector: "head".into(),
            message: "Missing viewport meta tag".into(),
            help: "Add <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
                .into(),
        });
    }
}
