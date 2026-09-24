//! Was an `<head>` nicht Barrierefreiheit ist.
//!
//! `html/lang-missing`, `html/title-missing`, `html/title-empty` und
//! `html/viewport-missing` sind hier abgelöst — sie kommen jetzt als
//! `document/lang-missing`, `document/title-missing`, `document/title-empty`
//! und `zoom/viewport-missing` aus `a11y-rules`. Was bleibt, ist SEO:
//! Meta-Description und Titellänge. Die gehören nicht nach `a11y-rules` und
//! behalten ihre Kennung.

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

            // Titellaenge -- eine Empfehlung fuer die Suchergebnisseite,
            // keine Barrierefreiheit. Dass der Titel ueberhaupt da und nicht
            // leer ist, prueft jetzt document/title-* aus a11y-rules.
            check_title_length(page, config, &mut findings);

            // meta description: presence check + length check (independent)
            check_meta_description(page, config, &mut findings);

            findings
        })
        .collect()
}

/// Nur noch die Laenge. Vorhandensein und Leere prueft `a11y-rules`.
fn check_title_length(
    page: &crate::discovery::PageInfo,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    let Some(trimmed) = page.title_text.as_ref().filter(|t| !t.is_empty()) else {
        return;
    };
    let Some(max) = config.html_basics.title_max_length else {
        return;
    };
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
