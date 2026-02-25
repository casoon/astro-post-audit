use rayon::prelude::*;
use scraper::Selector;

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

            let h1_sel = Selector::parse("h1").unwrap();
            let h1_count = html.select(&h1_sel).count();

            // Require H1
            if config.headings.require_h1 && h1_count == 0 {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "headings/no-h1".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: "Page has no <h1> heading".into(),
                    help: "Add exactly one <h1> as the main heading".into(),
                });
            }

            // Single H1
            if config.headings.single_h1 && h1_count > 1 {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "headings/multiple-h1".into(),
                    file: page.rel_path.clone(),
                    selector: "h1".into(),
                    message: format!("Page has {} <h1> headings (expected 1)", h1_count),
                    help: "Use only one <h1> per page for clear document structure".into(),
                });
            }

            // No heading level skip
            if config.headings.no_skip {
                let all_headings_sel = Selector::parse("h1, h2, h3, h4, h5, h6").unwrap();
                let mut ordered_levels: Vec<u8> = Vec::new();

                for el in html.select(&all_headings_sel) {
                    let tag = el.value().name();
                    if let Some(level) = tag.strip_prefix('h').and_then(|s| s.parse::<u8>().ok()) {
                        ordered_levels.push(level);
                    }
                }

                for window in ordered_levels.windows(2) {
                    let prev = window[0];
                    let curr = window[1];
                    if curr > prev + 1 {
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "headings/skip-level".into(),
                            file: page.rel_path.clone(),
                            selector: format!("h{}", curr),
                            message: format!(
                                "Heading level skip: <h{}> follows <h{}> (missing <h{}>)",
                                curr,
                                prev,
                                prev + 1
                            ),
                            help: "Don't skip heading levels; use sequential heading hierarchy"
                                .into(),
                        });
                    }
                }
            }

            findings
        })
        .collect()
}
