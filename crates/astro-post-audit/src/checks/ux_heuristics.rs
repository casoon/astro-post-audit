use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};

const CTA_KEYWORDS_DE: &[&str] = &[
    "kaufen",
    "buchen",
    "starten",
    "kontakt",
    "anfrage",
    "bestellen",
    "anmelden",
    "registrieren",
    "herunterladen",
    "jetzt",
    "kostenlos",
];

const CTA_KEYWORDS_EN: &[&str] = &[
    "buy",
    "book",
    "start",
    "contact",
    "order",
    "sign up",
    "register",
    "download",
    "get started",
    "try",
    "free",
    "subscribe",
    "request",
];

const TRUST_KEYWORDS: &[&str] = &[
    "impressum",
    "datenschutz",
    "kontakt",
    "about",
    "über uns",
    "privacy",
    "legal",
    "imprint",
    "contact",
];

use std::sync::LazyLock;

static LINK_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a[href]").expect("valid selector"));
static BUTTON_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("button, [role='button']").expect("valid selector"));
static INTERACTIVE_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("button, input, select, textarea").expect("valid selector"));
static ADDRESS_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("address").expect("valid selector"));

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.ux_heuristics.enabled {
        return Vec::new();
    }

    let ux = &config.ux_heuristics;

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            let links: Vec<_> = html.select(&LINK_SEL).collect();
            let buttons: Vec<_> = html.select(&BUTTON_SEL).collect();

            // === Dimension 1: CTA Clarity ===

            // Count CTA-like elements; an element matching several keywords counts once
            let cta_count = links
                .iter()
                .chain(buttons.iter())
                .filter(|el| {
                    let text = el.text().collect::<String>().to_lowercase();
                    let href = el.value().attr("href").unwrap_or("").to_lowercase();
                    CTA_KEYWORDS_DE.iter().any(|&kw| text.contains(kw) || href.contains(kw))
                        || CTA_KEYWORDS_EN.iter().any(|&kw| text.contains(kw) || href.contains(kw))
                })
                .count();

            if ux.min_cta_per_page > 0 && cta_count < ux.min_cta_per_page {
                let message = if cta_count == 0 {
                    "No link or button with a call-to-action keyword found on this page".to_string()
                } else {
                    format!(
                        "{} link(s) or button(s) with a call-to-action keyword found on this page, expected at least {}",
                        cta_count, ux.min_cta_per_page
                    )
                };
                findings.push(Finding::review("ux/no-cta", message)
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("The check matches action keywords in link and button text. Pages without a conversion goal, such as articles or legal pages, need no CTA; lower or disable `minCtaPerPage` for them."));
            }

            // Generic link text is reported by the links/generic-name rule of a11y_core.

            // === Dimension 3: Trust Signals ===

            let has_trust_link = links.iter().any(|el| {
                let text = el.text().collect::<String>().to_lowercase();
                let href = el.value().attr("href").unwrap_or("").to_lowercase();
                TRUST_KEYWORDS.iter().any(|&kw| text.contains(kw) || href.contains(kw))
            });

            let has_address = html.select(&ADDRESS_SEL).next().is_some();

            if !has_trust_link && !has_address {
                findings.push(Finding::review("ux/no-trust-signals","No link to an Impressum, privacy, contact or about page and no <address> element found")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("The check only looks for a short list of keywords in link text and URLs. If your legal or contact pages are linked under other names, ignore this; some jurisdictions require such links."));
            }

            // === Dimension 4: Cognitive Load ===

            let link_count = links.len();
            if link_count > ux.max_links_per_page {
                findings.push(Finding::review("ux/high-link-density", format!(
                        "{} links on this page (threshold: {})",
                        link_count, ux.max_links_per_page
                    ))
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Many links can be fine on index and navigation pages. Raise `maxLinksPerPage` if this is intended."));
            }

            let interactive_count = html.select(&INTERACTIVE_SEL).count();
            if interactive_count > 20 {
                findings.push(Finding::review("ux/high-interactive-density", format!(
                        "{} buttons, inputs, selects and textareas on this page (threshold: 20)",
                        interactive_count
                    ))
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Long forms and control-heavy pages can be hard to complete. Ignore this where the page needs them."));
            }

            findings
        })
        .collect()
}
