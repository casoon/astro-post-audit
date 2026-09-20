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

const GENERIC_LINK_TEXTS: &[&str] = &[
    "mehr",
    "hier",
    "weiter",
    "lesen",
    "click here",
    "read more",
    "learn more",
    "more",
    "here",
    "details",
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

            // Check for at least one CTA-like element
            let cta_found = links.iter().chain(buttons.iter()).any(|el| {
                let text = el.text().collect::<String>().to_lowercase();
                let href = el.value().attr("href").unwrap_or("").to_lowercase();
                CTA_KEYWORDS_DE.iter().any(|&kw| text.contains(kw) || href.contains(kw))
                    || CTA_KEYWORDS_EN.iter().any(|&kw| text.contains(kw) || href.contains(kw))
            });

            if ux.min_cta_per_page > 0 && !cta_found {
                findings.push(Finding::fail("ux/no-cta","No call-to-action found on this page")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Add at least one clear CTA (button or link with action-oriented text like 'Get started', 'Contact', 'Buy')."));
            }

            // Generic link text (UX signal, separate from a11y check)
            for link in &links {
                let text = link.text().collect::<String>();
                let normalized = text.trim().to_lowercase();
                if GENERIC_LINK_TEXTS.iter().any(|&g| normalized == g) {
                    let href = link.value().attr("href").unwrap_or("(no href)");
                    findings.push(Finding::fail("ux/generic-link-text", format!(
                            "Generic link text '{}' is not descriptive — users can't predict the destination",
                            text.trim()
                        ))
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector(format!("a[href='{}']", href)))
.with_help("Replace with descriptive text that explains where the link leads."));
                }
            }

            // === Dimension 3: Trust Signals ===

            let has_trust_link = links.iter().any(|el| {
                let text = el.text().collect::<String>().to_lowercase();
                let href = el.value().attr("href").unwrap_or("").to_lowercase();
                TRUST_KEYWORDS.iter().any(|&kw| text.contains(kw) || href.contains(kw))
            });

            let has_address = html.select(&ADDRESS_SEL).next().is_some();

            if !has_trust_link && !has_address {
                findings.push(Finding::fail("ux/no-trust-signals","No trust signal links found (Impressum, Datenschutz, Contact, About)")
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Add links to legal/contact pages to build user trust and comply with legal requirements."));
            }

            // === Dimension 4: Cognitive Load ===

            let link_count = links.len();
            if link_count > ux.max_links_per_page {
                findings.push(Finding::fail("ux/high-link-density", format!(
                        "{} links on this page may overwhelm users (threshold: {})",
                        link_count, ux.max_links_per_page
                    ))
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Consider reducing the number of links or grouping them into fewer, clearer navigation areas."));
            }

            let interactive_count = html.select(&INTERACTIVE_SEL).count();
            if interactive_count > 20 {
                findings.push(Finding::fail("ux/high-interactive-density", format!(
                        "{} interactive elements on this page — high cognitive load",
                        interactive_count
                    ))
.with_severity(Severity::Low)
.at(Location::file(page.rel_path.clone()).with_selector("body"))
.with_help("Simplify forms and reduce the number of interactive elements per page."));
            }

            findings
        })
        .collect()
}
