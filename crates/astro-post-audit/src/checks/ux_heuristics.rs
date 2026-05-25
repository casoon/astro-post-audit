use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

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

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.ux_heuristics.enabled {
        return Vec::new();
    }

    let ux = &config.ux_heuristics;

    let link_sel = Selector::parse("a[href]").unwrap();
    let button_sel = Selector::parse("button, [role='button']").unwrap();
    let interactive_sel = Selector::parse("button, input, select, textarea").unwrap();
    let address_sel = Selector::parse("address").unwrap();

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            let links: Vec<_> = html.select(&link_sel).collect();
            let buttons: Vec<_> = html.select(&button_sel).collect();

            // === Dimension 1: CTA Clarity ===

            // Check for at least one CTA-like element
            let cta_found = links.iter().chain(buttons.iter()).any(|el| {
                let text = el.text().collect::<String>().to_lowercase();
                let href = el.value().attr("href").unwrap_or("").to_lowercase();
                CTA_KEYWORDS_DE.iter().any(|&kw| text.contains(kw) || href.contains(kw))
                    || CTA_KEYWORDS_EN.iter().any(|&kw| text.contains(kw) || href.contains(kw))
            });

            if ux.min_cta_per_page > 0 && !cta_found {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "ux/no-cta".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: "No call-to-action found on this page".into(),
                    help: "Add at least one clear CTA (button or link with action-oriented text like 'Get started', 'Contact', 'Buy').".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            // Generic link text (UX signal, separate from a11y check)
            for link in &links {
                let text = link.text().collect::<String>();
                let normalized = text.trim().to_lowercase();
                if GENERIC_LINK_TEXTS.iter().any(|&g| normalized == g) {
                    let href = link.value().attr("href").unwrap_or("(no href)");
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "ux/generic-link-text".into(),
                        file: page.rel_path.clone(),
                        selector: format!("a[href='{}']", href),
                        message: format!(
                            "Generic link text '{}' is not descriptive — users can't predict the destination",
                            text.trim()
                        ),
                        help: "Replace with descriptive text that explains where the link leads.".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
            }

            // === Dimension 3: Trust Signals ===

            let has_trust_link = links.iter().any(|el| {
                let text = el.text().collect::<String>().to_lowercase();
                let href = el.value().attr("href").unwrap_or("").to_lowercase();
                TRUST_KEYWORDS.iter().any(|&kw| text.contains(kw) || href.contains(kw))
            });

            let has_address = html.select(&address_sel).next().is_some();

            if !has_trust_link && !has_address {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "ux/no-trust-signals".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: "No trust signal links found (Impressum, Datenschutz, Contact, About)".into(),
                    help: "Add links to legal/contact pages to build user trust and comply with legal requirements.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            // === Dimension 4: Cognitive Load ===

            let link_count = links.len();
            if link_count > ux.max_links_per_page {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "ux/high-link-density".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: format!(
                        "{} links on this page may overwhelm users (threshold: {})",
                        link_count, ux.max_links_per_page
                    ),
                    help: "Consider reducing the number of links or grouping them into fewer, clearer navigation areas.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            let interactive_count = html.select(&interactive_sel).count();
            if interactive_count > 20 {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "ux/high-interactive-density".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: format!(
                        "{} interactive elements on this page — high cognitive load",
                        interactive_count
                    ),
                    help: "Simplify forms and reduce the number of interactive elements per page.".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }

            findings
        })
        .collect()
}
