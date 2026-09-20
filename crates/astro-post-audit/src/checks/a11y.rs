use rayon::prelude::*;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};

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

            // Skip navigation link
            if config.a11y.require_skip_link {
                check_skip_link(page, &html, &mut findings);
            }

            // Landmark structure
            if config.a11y.check_landmarks {
                check_landmarks(page, &html, &mut findings);
            }

            // Duplicate IDs
            if config.a11y.check_duplicate_ids {
                check_duplicate_ids(page, &html, &mut findings);
            }

            // ARIA role validation
            if config.a11y.check_aria_roles {
                check_aria_roles(page, &html, &mut findings);
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

        match attrs.attr("alt") {
            None => {
                let src = attrs.attr("src").unwrap_or("(unknown)");
                findings.push(Finding::fail("a11y/img-alt", format!("Image missing alt attribute: src='{}'", src))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector(format!("img[src='{}']", src)))
.with_help("Add an `alt` prop to <Image>/<Picture> or the <img> tag. Use alt=\"\" only for decorative images.")
.with_suggestion("alt=\"...\""));
            }
            Some(alt) if config.a11y.check_alt_quality => {
                let src = attrs.attr("src").unwrap_or("(unknown)");
                if let Some(reason) = low_quality_alt_reason(alt, src) {
                    findings.push(Finding::review("a11y/invalid-img-alt", format!("Image alt text looks low-quality ({}): alt='{}'", reason, alt))
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector(format!("img[src='{}']", src)))
.with_help("Describe the image's meaning for screen-reader users. Use alt=\"\" only for purely decorative images."));
                }
            }
            Some(_) => {}
        }
    }
}

/// Words that signal a non-descriptive, placeholder alt value (matched as the
/// whole trimmed alt, case-insensitive).
const SUSPICIOUS_ALT_WORDS: &[&str] = &[
    "image",
    "img",
    "photo",
    "picture",
    "pic",
    "placeholder",
    "logo",
    "icon",
    "alt-text",
    "alt text",
    "untitled",
    "thumbnail",
    "banner",
];

const ALT_IMAGE_EXTENSIONS: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".webp", ".avif", ".svg", ".bmp",
];

/// Return a human-readable reason if the alt text is heuristically low quality,
/// or None if it looks acceptable. A genuinely empty alt ("") is decorative and skipped.
fn low_quality_alt_reason(alt: &str, src: &str) -> Option<&'static str> {
    let trimmed = alt.trim();
    if trimmed.is_empty() {
        return None; // decorative, handled elsewhere
    }
    let lower = trimmed.to_lowercase();

    // Looks like a filename (has an image extension)
    if ALT_IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext)) {
        return Some("looks like a filename");
    }

    // Equals the image's own filename (with or without extension)
    let basename = src
        .split('?')
        .next()
        .unwrap_or(src)
        .rsplit('/')
        .next()
        .unwrap_or(src)
        .to_lowercase();
    let stem = basename
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(&basename);
    if !basename.is_empty() && (lower == basename || lower == stem) {
        return Some("matches the file name");
    }

    // Placeholder / generic words
    if SUSPICIOUS_ALT_WORDS.contains(&lower.as_str()) {
        return Some("generic placeholder word");
    }

    // Too short to be meaningful (1–2 characters)
    if trimmed.chars().count() < 3 {
        return Some("too short");
    }

    None
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
            findings.push(
                Finding::fail(
                    "a11y/link-name",
                    format!("Link has no accessible name: href='{}'", href),
                )
                .with_severity(Severity::High)
                .at(Location::file(page.rel_path.clone())
                    .with_selector(format!("a[href='{}']", href)))
                .with_help("Add text content, aria-label, or aria-labelledby to the link"),
            );
            continue;
        }

        // Check for generic link text
        if config.a11y.warn_generic_link_text && has_text && !has_aria_label {
            let normalized = text_content.trim().to_lowercase();
            let is_generic = GENERIC_LINK_TEXTS_DE.iter().any(|&t| normalized == t)
                || GENERIC_LINK_TEXTS_EN.iter().any(|&t| normalized == t);

            if is_generic {
                let href = attrs.attr("href").unwrap_or("(no href)");
                findings.push(
                    Finding::fail(
                        "a11y/generic-link-text",
                        format!(
                            "Link has generic text '{}' - not descriptive for screen readers",
                            text_content.trim()
                        ),
                    )
                    .with_severity(Severity::Medium)
                    .at(Location::file(page.rel_path.clone())
                        .with_selector(format!("a[href='{}']", href)))
                    .with_help("Use descriptive link text or add an aria-label"),
                );
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
            findings.push(
                Finding::fail("a11y/button-name", "Button has no accessible name")
                    .with_severity(Severity::High)
                    .at(Location::file(page.rel_path.clone()).with_selector("button"))
                    .with_help("Add text content, aria-label, or aria-labelledby to the button"),
            );
        }
    }
}

fn check_form_labels(page: &crate::discovery::PageInfo, html: &Html, findings: &mut Vec<Finding>) {
    let sel = match Selector::parse("input:not([type='hidden']):not([type='submit']):not([type='button']):not([type='reset']):not([type='image']), select, textarea") {
        Ok(s) => s,
        Err(_) => return,
    };

    let label_sel = Selector::parse("label").unwrap();
    let wrapped_label_sel = Selector::parse("label input, label select, label textarea").unwrap();

    // Collect all label[for] targets
    let label_fors: HashSet<String> = html
        .select(&label_sel)
        .filter_map(|l| l.value().attr("for").map(|s| s.to_string()))
        .collect();
    let wrapped_controls: HashSet<_> = html.select(&wrapped_label_sel).map(|c| c.id()).collect();

    for el in html.select(&sel) {
        let attrs = el.value();

        let has_aria_label = attrs
            .attr("aria-label")
            .is_some_and(|v| !v.trim().is_empty());
        let has_aria_labelledby = attrs
            .attr("aria-labelledby")
            .is_some_and(|v| !v.trim().is_empty());
        let has_id_with_label = attrs.attr("id").is_some_and(|id| label_fors.contains(id));
        let has_wrapping_label = wrapped_controls.contains(&el.id());

        if !has_aria_label && !has_aria_labelledby && !has_id_with_label && !has_wrapping_label {
            let input_type = attrs.attr("type").unwrap_or("text");
            let name = attrs.attr("name").unwrap_or("(unnamed)");
            findings.push(
                Finding::fail(
                    "a11y/form-label",
                    format!(
                        "Form control '{}' (type='{}') has no associated label",
                        name, input_type
                    ),
                )
                .with_severity(Severity::High)
                .at(Location::file(page.rel_path.clone())
                    .with_selector(format!("input[type='{}'][name='{}']", input_type, name)))
                .with_help("Add a <label for='id'>, aria-label, or aria-labelledby"),
            );
        }
    }
}

fn check_skip_link(page: &crate::discovery::PageInfo, html: &Html, findings: &mut Vec<Finding>) {
    // A skip link is an <a> early in the DOM linking to #main, #main-content, #content or similar
    let sel = match Selector::parse("a[href^='#']") {
        Ok(s) => s,
        Err(_) => return,
    };

    let has_skip = html.select(&sel).take(8).any(|el| {
        let href = el.value().attr("href").unwrap_or("");
        let target = href.trim_start_matches('#').to_lowercase();
        let text = el.text().collect::<String>().to_lowercase();
        let class_or_id = format!(
            "{} {}",
            el.value().attr("class").unwrap_or(""),
            el.value().attr("id").unwrap_or("")
        )
        .to_lowercase();

        let target_matches = matches!(
            target.as_str(),
            "main" | "main-content" | "maincontent" | "content" | "skip" | "inhalt"
        );
        let text_or_marker_matches = text.contains("skip")
            || text.contains("zum inhalt")
            || text.contains("skip to content")
            || class_or_id.contains("skip");

        target_matches && text_or_marker_matches
    });

    if !has_skip {
        findings.push(Finding::fail("a11y/skip-link","No skip navigation link found")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Add a skip link like <a href=\"#main-content\" class=\"sr-only focus:not-sr-only\">Skip to content</a> as the first element in <body>")
.with_suggestion("<a href=\"#main-content\" class=\"sr-only\">Skip to content</a>"));
    }
}

fn check_aria_hidden_focusable(
    page: &crate::discovery::PageInfo,
    html: &Html,
    findings: &mut Vec<Finding>,
) {
    let sel = Selector::parse("[aria-hidden='true']").unwrap();

    for el in html.select(&sel) {
        let tag = el.value().name();
        let is_focusable = matches!(tag, "a" | "button" | "input" | "select" | "textarea")
            || el.value().attr("tabindex").is_some_and(|v| v != "-1");

        if is_focusable {
            findings.push(
                Finding::fail(
                    "a11y/aria-hidden-focusable",
                    format!("Focusable element <{}> has aria-hidden=\"true\"", tag),
                )
                .with_severity(Severity::Medium)
                .at(Location::file(page.rel_path.clone())
                    .with_selector(format!("{}[aria-hidden='true']", tag)))
                .with_help("Remove aria-hidden from focusable elements, or add tabindex=\"-1\""),
            );
        }
    }
}

fn check_landmarks(page: &crate::discovery::PageInfo, html: &Html, findings: &mut Vec<Finding>) {
    let main_sel = Selector::parse("main, [role='main']").unwrap();
    let nav_sel = Selector::parse("nav, [role='navigation']").unwrap();
    // <header>/<footer> are banner/contentinfo landmarks unless nested inside
    // sectioning content — a <div> wrapper must not disqualify them (#31, #32)
    let header_sel = Selector::parse(
        "header:not(article header):not(aside header):not(main header):not(nav header):not(section header), [role='banner']",
    )
    .unwrap();
    let footer_sel = Selector::parse(
        "footer:not(article footer):not(aside footer):not(main footer):not(nav footer):not(section footer), [role='contentinfo']",
    )
    .unwrap();

    let main_count = html.select(&main_sel).count();
    if main_count == 0 {
        findings.push(
            Finding::fail(
                "a11y/landmark-main-missing",
                "Page has no <main> element or role=\"main\"",
            )
            .with_severity(Severity::High)
            .at(Location::file(page.rel_path.clone()).with_selector("body"))
            .with_help("Add a <main> element to wrap the primary page content (WCAG 1.3.1, 2.4.1)")
            .with_suggestion("<main id=\"main-content\">...</main>"),
        );
    } else if main_count > 1 {
        findings.push(
            Finding::fail(
                "a11y/landmark-main-duplicate",
                format!(
                    "Page has {} <main> elements — only one is allowed",
                    main_count
                ),
            )
            .with_severity(Severity::High)
            .at(Location::file(page.rel_path.clone()).with_selector("main"))
            .with_help("There must be exactly one <main> landmark per page"),
        );
    }

    if html.select(&nav_sel).next().is_none() {
        findings.push(
            Finding::fail(
                "a11y/landmark-nav-missing",
                "Page has no <nav> element or role=\"navigation\"",
            )
            .with_severity(Severity::Medium)
            .at(Location::file(page.rel_path.clone()).with_selector("body"))
            .with_help("Add a <nav> landmark for navigation regions (WCAG 2.4.1)"),
        );
    }

    if html.select(&header_sel).next().is_none() {
        findings.push(
            Finding::fail(
                "a11y/landmark-header-missing",
                "Page has no top-level <header> or role=\"banner\"",
            )
            .with_severity(Severity::Low)
            .at(Location::file(page.rel_path.clone()).with_selector("body"))
            .with_help("Add a <header> landmark at the top of the page"),
        );
    }

    if html.select(&footer_sel).next().is_none() {
        findings.push(
            Finding::fail(
                "a11y/landmark-footer-missing",
                "Page has no top-level <footer> or role=\"contentinfo\"",
            )
            .with_severity(Severity::Low)
            .at(Location::file(page.rel_path.clone()).with_selector("body"))
            .with_help("Add a <footer> landmark at the bottom of the page"),
        );
    }
}

fn check_duplicate_ids(
    page: &crate::discovery::PageInfo,
    html: &Html,
    findings: &mut Vec<Finding>,
) {
    let sel = Selector::parse("[id]").unwrap();
    let aria_sel = Selector::parse("[aria-labelledby], [aria-describedby]").unwrap();

    // Collect all id values and their occurrence count
    let mut id_counts: HashMap<&str, usize> = HashMap::new();
    for el in html.select(&sel) {
        if let Some(id) = el.value().attr("id") {
            *id_counts.entry(id).or_insert(0) += 1;
        }
    }

    // Collect IDs referenced in ARIA attributes
    let mut aria_referenced: HashSet<String> = HashSet::new();
    for el in html.select(&aria_sel) {
        for attr in ["aria-labelledby", "aria-describedby"] {
            if let Some(val) = el.value().attr(attr) {
                for id_ref in val.split_whitespace() {
                    aria_referenced.insert(id_ref.to_string());
                }
            }
        }
    }

    for (id, count) in &id_counts {
        if *count > 1 {
            let is_aria_ref = aria_referenced.contains(*id);
            let rule_id = if is_aria_ref {
                "a11y/duplicate-id-aria"
            } else {
                "a11y/duplicate-id"
            };
            findings.push(Finding::fail(rule_id, format!(
                    "Duplicate id=\"{}\" found {} times on this page{}",
                    id,
                    count,
                    if is_aria_ref { " (referenced by ARIA attribute)" } else { "" }
                ))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector(format!("[id='{}']", id)))
.with_help("Each id must be unique per page — duplicate ids break ARIA references and form associations (WCAG 4.1.1)"));
        }
    }
}

/// Valid WAI-ARIA roles (subset — abstract roles are listed separately).
const VALID_ARIA_ROLES: &[&str] = &[
    "alert",
    "alertdialog",
    "application",
    "article",
    "banner",
    "button",
    "cell",
    "checkbox",
    "columnheader",
    "combobox",
    "complementary",
    "contentinfo",
    "definition",
    "dialog",
    "directory",
    "document",
    "feed",
    "figure",
    "form",
    "generic",
    "grid",
    "gridcell",
    "group",
    "heading",
    "img",
    "link",
    "list",
    "listbox",
    "listitem",
    "log",
    "main",
    "marquee",
    "math",
    "menu",
    "menubar",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "navigation",
    "none",
    "note",
    "option",
    "presentation",
    "progressbar",
    "radio",
    "radiogroup",
    "region",
    "row",
    "rowgroup",
    "rowheader",
    "scrollbar",
    "search",
    "searchbox",
    "separator",
    "slider",
    "spinbutton",
    "status",
    "switch",
    "tab",
    "table",
    "tablist",
    "tabpanel",
    "term",
    "textbox",
    "timer",
    "toolbar",
    "tooltip",
    "tree",
    "treegrid",
    "treeitem",
];

/// Abstract ARIA roles that must not be used directly in HTML.
const ABSTRACT_ARIA_ROLES: &[&str] = &[
    "command",
    "composite",
    "input",
    "landmark",
    "range",
    "roletype",
    "section",
    "sectionhead",
    "select",
    "structure",
    "widget",
    "window",
];

fn check_aria_roles(page: &crate::discovery::PageInfo, html: &Html, findings: &mut Vec<Finding>) {
    let role_sel = Selector::parse("[role]").unwrap();

    for el in html.select(&role_sel) {
        let role_val = match el.value().attr("role") {
            Some(r) => r,
            None => continue,
        };

        // A role attribute can contain multiple space-separated tokens
        for role in role_val.split_whitespace() {
            if ABSTRACT_ARIA_ROLES.contains(&role) {
                findings.push(Finding::fail("a11y/aria-role-abstract", format!("Abstract ARIA role \"{}\" must not be used in HTML", role))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector(format!("[role='{}']", role_val)))
.with_help("Use a concrete role instead — abstract roles are base concepts, not usable in content"));
                continue;
            }

            if !VALID_ARIA_ROLES.contains(&role) {
                findings.push(Finding::fail("a11y/aria-role-invalid", format!("Unknown ARIA role \"{}\" — likely a typo", role))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector(format!("[role='{}']", role_val)))
.with_help(format!("Check the WAI-ARIA specification for valid role values. Did you mean one of: {:?}?",
                        VALID_ARIA_ROLES.iter().filter(|&&r| {
                            let d = strsim_distance(r, role);
                            d <= 2 && d > 0
                        }).take(3).collect::<Vec<_>>())));
                continue;
            }

            // Check required ARIA attributes for specific roles
            let tag = el.value().name();
            let attrs = el.value();
            match role {
                "checkbox" | "switch" if attrs.attr("aria-checked").is_none() => {
                    findings.push(
                        Finding::fail(
                            "a11y/aria-required-attr",
                            format!("role=\"{}\" requires aria-checked attribute", role),
                        )
                        .with_severity(Severity::High)
                        .at(Location::file(page.rel_path.clone())
                            .with_selector(format!("{}[role='{}']", tag, role)))
                        .with_help("Add aria-checked=\"true\", \"false\", or \"mixed\"")
                        .with_suggestion("aria-checked=\"false\""),
                    );
                }
                "checkbox" | "switch" => {}
                "combobox" if attrs.attr("aria-expanded").is_none() => {
                    findings.push(
                        Finding::fail(
                            "a11y/aria-required-attr",
                            "role=\"combobox\" requires aria-expanded attribute",
                        )
                        .with_severity(Severity::High)
                        .at(Location::file(page.rel_path.clone())
                            .with_selector(format!("{}[role='combobox']", tag)))
                        .with_help("Add aria-expanded=\"true\" or \"false\"")
                        .with_suggestion("aria-expanded=\"false\""),
                    );
                }
                "combobox" => {}
                "slider" => {
                    let missing: Vec<&str> = ["aria-valuenow", "aria-valuemin", "aria-valuemax"]
                        .iter()
                        .copied()
                        .filter(|&a| attrs.attr(a).is_none())
                        .collect();
                    if !missing.is_empty() {
                        findings.push(
                            Finding::fail(
                                "a11y/aria-required-attr",
                                format!(
                                    "role=\"slider\" requires missing attribute(s): {}",
                                    missing.join(", ")
                                ),
                            )
                            .with_severity(Severity::High)
                            .at(Location::file(page.rel_path.clone())
                                .with_selector(format!("{}[role='slider']", tag)))
                            .with_help("Add aria-valuenow, aria-valuemin, and aria-valuemax"),
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// Simple edit-distance approximation for role typo suggestions (max 2 substitutions).
fn strsim_distance(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    if a.len().abs_diff(b.len()) > 3 {
        return 99;
    }
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}
