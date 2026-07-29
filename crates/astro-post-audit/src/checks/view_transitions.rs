use rayon::prelude::*;
use scraper::Selector;
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

static TRANSITION_NAME_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("[transition\\:name], [data-astro-transition-name]").expect("valid selector")
});
static EXTERNAL_LINK_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("a[href^='http://'], a[href^='https://']").expect("valid selector")
});
static CLIENT_ROUTER_SCRIPT_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("[data-astro-transition-persist]").expect("valid selector"));

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.view_transitions.enabled {
        return Vec::new();
    }

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            // 1. Check for duplicate transition:name attributes on the same page
            if config.view_transitions.check_duplicate_names {
                let mut seen_names = HashSet::new();
                for el in html.select(&TRANSITION_NAME_SEL) {
                    let name_val = el
                        .value()
                        .attr("transition:name")
                        .or_else(|| el.value().attr("data-astro-transition-name"))
                        .unwrap_or("");
                    if !name_val.is_empty() && !seen_names.insert(name_val.to_string()) {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "view-transitions/duplicate-name".into(),
                            file: page.rel_path.clone(),
                            selector: format!("[transition:name='{}']", name_val),
                            message: format!("Duplicate transition:name '{}' on page — View Transitions require unique names per page", name_val),
                            help: "Ensure transition:name values are unique across all elements on the same page.".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }
            }

            // 2. Check for external links missing data-astro-reload only when a persisted
            // transition proves that the page uses Astro's ClientRouter.
            if config.view_transitions.check_external_reload {
                let router_present = html.select(&CLIENT_ROUTER_SCRIPT_SEL).next().is_some();
                if router_present {
                    for link in html.select(&EXTERNAL_LINK_SEL) {
                        let has_reload = link.value().attr("data-astro-reload").is_some();
                        let href = link.value().attr("href").unwrap_or("");
                        let target = link.value().attr("target").unwrap_or("");
                        if !has_reload && target != "_blank" && !href.is_empty() {
                            findings.push(Finding {
                                level: Level::Info,
                                rule_id: "view-transitions/missing-reload-hint".into(),
                                file: page.rel_path.clone(),
                                selector: format!("a[href='{}']", href),
                                message: format!("External link '{}' under ClientRouter missing data-astro-reload", href),
                                help: "Add data-astro-reload to external links so the Client Router does not intercept external navigation.".into(),
                                suggestion: Some("data-astro-reload".into()),
                                source_hint: None,
                                confidence: None,
                            });
                        }
                    }
                }
            }

            findings
        })
        .collect()
}
