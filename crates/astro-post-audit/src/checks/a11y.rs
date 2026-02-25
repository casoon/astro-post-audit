use rayon::prelude::*;
use scraper::{Html, Selector};

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

/// Generic link texts that should trigger a warning (lowercase, trimmed).
const GENERIC_LINK_TEXTS_DE: &[&str] = &[
    "hier", "mehr", "weiter", "klick", "link", "details", "ansehen",
];
const GENERIC_LINK_TEXTS_EN: &[&str] = &[
    "click here",
    "read more",
    "learn more",
    "more",
    "here",
    "details",
];

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            // Image alt checks
            if config.a11y.img_alt_required {
                check_img_alt(page, &html, config, &mut findings);
            }

            // Link accessible name checks
            if config.a11y.a_accessible_name_required {
                check_link_names(page, &html, config, &mut findings);
            }

            // Button name checks
            if config.a11y.button_name_required {
                check_button_names(page, &html, &mut findings);
            }

            // Form label checks
            if config.a11y.label_for_required {
                check_form_labels(page, &html, &mut findings);
            }

            // aria-hidden on focusable elements
            if config.a11y.aria_hidden_focusable_check {
                check_aria_hidden_focusable(page, &html, &mut findings);
            }

            findings
        })
        .collect()
}

fn check_img_alt(
    page: &crate::discovery::PageInfo,
    html: &Html,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    let sel = match Selector::parse("img") {
        Ok(s) => s,
        Err(_) => return,
    };

    for el in html.select(&sel) {
        let attrs = el.value();

        // Check decorative image exceptions
        if config.a11y.allow_decorative_images {
            if attrs
                .attr("role")
                .is_some_and(|r| r == "presentation" || r == "none")
            {
                continue;
            }
            if attrs.attr("aria-hidden").is_some_and(|v| v == "true") {
                continue;
            }
        }

        if attrs.attr("alt").is_none() {
            let src = attrs.attr("src").unwrap_or("(unknown)");
            findings.push(Finding {
                level: Level::Error,
                rule_id: "a11y/img-alt".into(),
                file: page.rel_path.clone(),
                selector: format!("img[src='{}']", src),
                message: format!("Image missing alt attribute: src='{}'", src),
                help: "Add an alt attribute describing the image, or use alt=\"\" for decorative images".into(),
            });
        }
    }
}

fn check_link_names(
    page: &crate::discovery::PageInfo,
    html: &Html,
    config: &Config,
    findings: &mut Vec<Finding>,
) {
    let sel = match Selector::parse("a") {
        Ok(s) => s,
        Err(_) => return,
    };

    for el in html.select(&sel) {
        let attrs = el.value();

        // Check if link has accessible name
        let has_aria_label = attrs
            .attr("aria-label")
            .is_some_and(|v| !v.trim().is_empty());
        let has_aria_labelledby = attrs
            .attr("aria-labelledby")
            .is_some_and(|v| !v.trim().is_empty());
        let text_content = el.text().collect::<String>();
        let has_text = !text_content.trim().is_empty();

        // Check for img with alt inside the link
        let img_alt_sel = Selector::parse("img[alt]").unwrap();
        let has_img_alt = el.select(&img_alt_sel).any(|img| {
            img.value()
                .attr("alt")
                .is_some_and(|a| !a.trim().is_empty())
        });

        if !has_aria_label && !has_aria_labelledby && !has_text && !has_img_alt {
            let href = attrs.attr("href").unwrap_or("(no href)");
            findings.push(Finding {
                level: Level::Error,
                rule_id: "a11y/link-name".into(),
                file: page.rel_path.clone(),
                selector: format!("a[href='{}']", href),
                message: format!("Link has no accessible name: href='{}'", href),
                help: "Add text content, aria-label, or aria-labelledby to the link".into(),
            });
            continue;
        }

        // Check for generic link text
        if config.a11y.warn_generic_link_text && has_text && !has_aria_label {
            let normalized = text_content.trim().to_lowercase();
            let is_generic = GENERIC_LINK_TEXTS_DE.iter().any(|&t| normalized == t)
                || GENERIC_LINK_TEXTS_EN.iter().any(|&t| normalized == t);

            if is_generic {
                let href = attrs.attr("href").unwrap_or("(no href)");
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "a11y/generic-link-text".into(),
                    file: page.rel_path.clone(),
                    selector: format!("a[href='{}']", href),
                    message: format!(
                        "Link has generic text '{}' - not descriptive for screen readers",
                        text_content.trim()
                    ),
                    help: "Use descriptive link text or add an aria-label".into(),
                });
            }
        }
    }
}

fn check_button_names(page: &crate::discovery::PageInfo, html: &Html, findings: &mut Vec<Finding>) {
    let sel = match Selector::parse("button") {
        Ok(s) => s,
        Err(_) => return,
    };

    for el in html.select(&sel) {
        let attrs = el.value();
        let has_aria_label = attrs
            .attr("aria-label")
            .is_some_and(|v| !v.trim().is_empty());
        let has_aria_labelledby = attrs
            .attr("aria-labelledby")
            .is_some_and(|v| !v.trim().is_empty());
        let text_content = el.text().collect::<String>();
        let has_text = !text_content.trim().is_empty();

        if !has_aria_label && !has_aria_labelledby && !has_text {
            findings.push(Finding {
                level: Level::Error,
                rule_id: "a11y/button-name".into(),
                file: page.rel_path.clone(),
                selector: "button".into(),
                message: "Button has no accessible name".into(),
                help: "Add text content, aria-label, or aria-labelledby to the button".into(),
            });
        }
    }
}

fn check_form_labels(page: &crate::discovery::PageInfo, html: &Html, findings: &mut Vec<Finding>) {
    let sel = match Selector::parse("input:not([type='hidden']):not([type='submit']):not([type='button']):not([type='reset']):not([type='image']), select, textarea") {
        Ok(s) => s,
        Err(_) => return,
    };

    let label_sel = Selector::parse("label").unwrap();

    // Collect all label[for] targets
    let label_fors: std::collections::HashSet<String> = html
        .select(&label_sel)
        .filter_map(|l| l.value().attr("for").map(|s| s.to_string()))
        .collect();

    for el in html.select(&sel) {
        let attrs = el.value();

        let has_aria_label = attrs
            .attr("aria-label")
            .is_some_and(|v| !v.trim().is_empty());
        let has_aria_labelledby = attrs
            .attr("aria-labelledby")
            .is_some_and(|v| !v.trim().is_empty());
        let has_id_with_label = attrs.attr("id").is_some_and(|id| label_fors.contains(id));

        if !has_aria_label && !has_aria_labelledby && !has_id_with_label {
            let input_type = attrs.attr("type").unwrap_or("text");
            let name = attrs.attr("name").unwrap_or("(unnamed)");
            findings.push(Finding {
                level: Level::Error,
                rule_id: "a11y/form-label".into(),
                file: page.rel_path.clone(),
                selector: format!("input[type='{}'][name='{}']", input_type, name),
                message: format!(
                    "Form control '{}' (type='{}') has no associated label",
                    name, input_type
                ),
                help: "Add a <label for='id'>, aria-label, or aria-labelledby".into(),
            });
        }
    }
}

fn check_aria_hidden_focusable(
    page: &crate::discovery::PageInfo,
    html: &Html,
    findings: &mut Vec<Finding>,
) {
    // Check for aria-hidden="true" on focusable elements
    let sel = Selector::parse("[aria-hidden='true']").unwrap();

    for el in html.select(&sel) {
        let tag = el.value().name();
        let is_focusable = matches!(tag, "a" | "button" | "input" | "select" | "textarea")
            || el.value().attr("tabindex").is_some_and(|v| v != "-1");

        if is_focusable {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "a11y/aria-hidden-focusable".into(),
                file: page.rel_path.clone(),
                selector: format!("{}[aria-hidden='true']", tag),
                message: format!("Focusable element <{}> has aria-hidden=\"true\"", tag),
                help: "Remove aria-hidden from focusable elements, or add tabindex=\"-1\"".into(),
            });
        }
    }
}
