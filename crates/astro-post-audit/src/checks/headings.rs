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
            let h1_count = page.h1_count;

            // Require H1
            if config.headings.require_h1 && h1_count == 0 {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "headings/no-h1".into(),
                    file: page.rel_path.clone(),
                    selector: "body".into(),
                    message: "Page has no <h1> heading".into(),
                    help: "Add exactly one <h1> as the main heading".into(),
                    suggestion: None,
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
                    suggestion: None,
                });
            }

            // No heading level skip
            if config.headings.no_skip {
                for window in page.heading_levels.windows(2) {
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
                            suggestion: None,
                        });
                    }
                }
            }

            findings
        })
        .collect()
}
