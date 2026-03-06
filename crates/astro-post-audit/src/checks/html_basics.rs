use rayon::prelude::*;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();

            // lang attribute
            if config.html_basics.lang_attr_required {
                check_lang(page, &mut findings);
            }

            // title tag
            if config.html_basics.title_required {
                check_title(page, config, &mut findings);
            }

            // meta description: presence check + length check (independent)
            check_meta_description(page, config, &mut findings);

            // viewport
            if config.html_basics.viewport_required {
                check_viewport(page, &mut findings);
            }

            findings
        })
        .collect()
}

fn check_lang(page: &crate::discovery::PageInfo, findings: &mut Vec<Finding>) {
    let has_lang = page
        .html_lang
        .as_ref()
        .is_some_and(|v| !v.trim().is_empty());

    if !has_lang {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "html/lang-missing".into(),
            file: page.rel_path.clone(),
            selector: "html".into(),
            message: "Missing lang attribute on <html> element".into(),
            help: "Add lang attribute, e.g., <html lang=\"en\">".into(),
            suggestion: Some("<html lang=\"en\">".into()),
        });
    }
}

fn check_title(page: &crate::discovery::PageInfo, config: &Config, findings: &mut Vec<Finding>) {
    match &page.title_text {
        None => {
            findings.push(Finding {
                level: Level::Error,
                rule_id: "html/title-missing".into(),
                file: page.rel_path.clone(),
                selector: "head".into(),
                message: "Missing <title> tag".into(),
                help: "Add a <title> tag inside <head>".into(),
                suggestion: Some("<title>Page Title</title>".into()),
            });
        }
        Some(trimmed) => {
            if trimmed.is_empty() {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "html/title-empty".into(),
                    file: page.rel_path.clone(),
                    selector: "title".into(),
                    message: "Title tag is empty".into(),
                    help: "Add descriptive text to the <title> tag".into(),
                    suggestion: Some("<title>Page Title</title>".into()),
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
                        suggestion: None,
                    });
                }
            }
        }
    }
}

fn check_meta_description(
    page: &crate::discovery::PageInfo,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    match &page.meta_description {
        None => {
            // Only warn about missing description if required
            if config.html_basics.meta_description_required {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "html/meta-description-missing".into(),
                    file: page.rel_path.clone(),
                    selector: "head".into(),
                    message: "Missing or empty meta description".into(),
                    help: "Add <meta name=\"description\" content=\"...\"> to <head>".into(),
                    suggestion: Some("<meta name=\"description\" content=\"...\">".into()),
                });
            }
        }
        Some(trimmed) => {
            if trimmed.is_empty() {
                if config.html_basics.meta_description_required {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "html/meta-description-missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "Missing or empty meta description".into(),
                        help: "Add <meta name=\"description\" content=\"...\"> to <head>".into(),
                        suggestion: Some("<meta name=\"description\" content=\"...\">".into()),
                    });
                }
            } else if let Some(max) = config.html_basics.meta_description_max_length {
                // Length check runs independently, even if description is not required
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
                        suggestion: None,
                    });
                }
            }
        }
    }
}

fn check_viewport(page: &crate::discovery::PageInfo, findings: &mut Vec<Finding>) {
    if !page.has_viewport {
        findings.push(Finding {
            level: Level::Error,
            rule_id: "html/viewport-missing".into(),
            file: page.rel_path.clone(),
            selector: "head".into(),
            message: "Missing viewport meta tag".into(),
            help: "Add <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
                .into(),
            suggestion: Some(
                "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">".into(),
            ),
        });
    }
}
