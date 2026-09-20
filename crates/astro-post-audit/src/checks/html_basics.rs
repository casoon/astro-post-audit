use rayon::prelude::*;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};

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
        findings.push(Finding::fail("html/lang-missing","Missing lang attribute on <html> element")
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector("html"))
.with_help("Set the lang attribute on the root <html> element in your main Layout (e.g. <html lang=\"en\">). For multilingual sites, derive it from Astro.currentLocale.")
.with_suggestion("<html lang=\"en\">"));
    }
}

fn check_title(page: &crate::discovery::PageInfo, config: &Config, findings: &mut Vec<Finding>) {
    match &page.title_text {
        None => {
            findings.push(
                Finding::fail("html/title-missing", "Missing <title> tag")
                    .with_severity(Severity::High)
                    .at(Location::file(page.rel_path.clone()).with_selector("head"))
                    .with_help("Add a <title> tag inside <head>")
                    .with_suggestion("<title>Page Title</title>"),
            );
        }
        Some(trimmed) => {
            if trimmed.is_empty() {
                findings.push(
                    Finding::fail("html/title-empty", "Title tag is empty")
                        .with_severity(Severity::High)
                        .at(Location::file(page.rel_path.clone()).with_selector("title"))
                        .with_help("Add descriptive text to the <title> tag")
                        .with_suggestion("<title>Page Title</title>"),
                );
            } else if let Some(max) = config.html_basics.title_max_length {
                if trimmed.len() > max {
                    findings.push(
                        Finding::fail(
                            "html/title-too-long",
                            format!(
                                "Title is {} chars (recommended max: {})",
                                trimmed.len(),
                                max
                            ),
                        )
                        .with_severity(Severity::Medium)
                        .at(Location::file(page.rel_path.clone()).with_selector("title"))
                        .with_help("Shorten the title for better display in search results"),
                    );
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
                findings.push(Finding::fail("html/meta-description-missing","Missing or empty meta description")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Render <meta name=\"description\" content={...}> in your BaseHead component, driven by a `description` prop from page frontmatter")
.with_suggestion("<meta name=\"description\" content=\"...\">"));
            }
        }
        Some(trimmed) => {
            if trimmed.is_empty() {
                if config.html_basics.meta_description_required {
                    findings.push(Finding::fail("html/meta-description-missing","Missing or empty meta description")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("head"))
.with_help("Render <meta name=\"description\" content={...}> in your BaseHead component, driven by a `description` prop from page frontmatter")
.with_suggestion("<meta name=\"description\" content=\"...\">"));
                }
            } else if let Some(max) = config.html_basics.meta_description_max_length {
                // Length check runs independently, even if description is not required
                if trimmed.len() > max {
                    findings.push(
                        Finding::fail(
                            "html/meta-description-too-long",
                            format!(
                                "Meta description is {} chars (recommended max: {})",
                                trimmed.len(),
                                max
                            ),
                        )
                        .with_severity(Severity::Medium)
                        .at(Location::file(page.rel_path.clone())
                            .with_selector("meta[name='description']"))
                        .with_help("Shorten the description for better display in search results"),
                    );
                }
            }
        }
    }
}

fn check_viewport(page: &crate::discovery::PageInfo, findings: &mut Vec<Finding>) {
    if !page.has_viewport {
        findings.push(
            Finding::fail("html/viewport-missing", "Missing viewport meta tag")
                .with_severity(Severity::High)
                .at(Location::file(page.rel_path.clone()).with_selector("head"))
                .with_help(
                    "Add <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
                )
                .with_suggestion(
                    "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
                ),
        );
    }
}
