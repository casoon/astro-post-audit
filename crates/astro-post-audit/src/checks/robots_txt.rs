use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();

    let robots_path = index.dist_path.join("robots.txt");

    if !robots_path.exists() {
        if config.robots_txt.require {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "robots-txt/missing".into(),
                file: "robots.txt".into(),
                selector: String::new(),
                message: "robots.txt not found in dist directory".into(),
                help: "Add a robots.txt file to your public/ directory".into(),
            });
        }
        return findings;
    }

    if config.robots_txt.require_sitemap_link {
        let content = match std::fs::read_to_string(&robots_path) {
            Ok(c) => c,
            Err(_) => return findings,
        };

        let has_sitemap = content
            .lines()
            .any(|line| line.trim().to_lowercase().starts_with("sitemap:"));

        if !has_sitemap {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "robots-txt/no-sitemap".into(),
                file: "robots.txt".into(),
                selector: String::new(),
                message: "robots.txt does not contain a Sitemap directive".into(),
                help: "Add 'Sitemap: https://example.com/sitemap.xml' to robots.txt".into(),
            });
        }
    }

    findings
}
