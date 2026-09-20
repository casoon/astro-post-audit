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
            let h1_count = page.h1_count;

            // Require H1
            if config.headings.require_h1 && h1_count == 0 {
                findings.push(
                    Finding::fail("headings/no-h1", "Page has no <h1> heading")
                        .with_severity(Severity::High)
                        .at(Location::file(page.rel_path.clone()).with_selector("body"))
                        .with_help("Add exactly one <h1> as the main heading"),
                );
            }

            // Single H1
            if config.headings.single_h1 && h1_count > 1 {
                findings.push(
                    Finding::fail(
                        "headings/multiple-h1",
                        format!("Page has {} <h1> headings (expected 1)", h1_count),
                    )
                    .with_severity(Severity::Medium)
                    .at(Location::file(page.rel_path.clone()).with_selector("h1"))
                    .with_help("Use only one <h1> per page for clear document structure"),
                );
            }

            // No heading level skip
            if config.headings.no_skip {
                for window in page.heading_levels.windows(2) {
                    if let [prev, curr] = window {
                        if *curr > *prev + 1 {
                            findings.push(
                                Finding::fail(
                                    "headings/skip-level",
                                    format!(
                                        "Heading level skip: <h{}> follows <h{}> (missing <h{}>)",
                                        curr,
                                        prev,
                                        prev + 1
                                    ),
                                )
                                .with_severity(Severity::Medium)
                                .at(Location::file(page.rel_path.clone())
                                    .with_selector(format!("h{}", curr)))
                                .with_help(
                                    "Don't skip heading levels; use sequential heading hierarchy",
                                ),
                            );
                        }
                    }
                }
            }

            findings
        })
        .collect()
}
