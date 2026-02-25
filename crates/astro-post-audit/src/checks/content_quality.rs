use std::collections::HashMap;

use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

/// Truncate a string to approximately `max_chars` characters, safe for multi-byte UTF-8.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let cq = &config.content_quality;
    if !cq.detect_duplicate_titles
        && !cq.detect_duplicate_descriptions
        && !cq.detect_duplicate_h1
        && !cq.detect_duplicate_pages
    {
        return Vec::new();
    }

    let mut findings = Vec::new();

    // Collect values across all pages
    let mut titles: HashMap<String, Vec<String>> = HashMap::new();
    let mut descriptions: HashMap<String, Vec<String>> = HashMap::new();
    let mut h1s: HashMap<String, Vec<String>> = HashMap::new();
    let mut content_hashes: HashMap<u64, Vec<String>> = HashMap::new();

    for page in &index.pages {
        let html = page.parse_html();

        // Title
        if cq.detect_duplicate_titles {
            let sel = Selector::parse("title").unwrap();
            if let Some(el) = html.select(&sel).next() {
                let text: String = el.text().collect();
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    titles
                        .entry(trimmed)
                        .or_default()
                        .push(page.rel_path.clone());
                }
            }
        }

        // Meta description
        if cq.detect_duplicate_descriptions {
            let sel = Selector::parse("meta[name='description']").unwrap();
            if let Some(el) = html.select(&sel).next() {
                if let Some(content) = el.value().attr("content") {
                    let trimmed = content.trim().to_string();
                    if !trimmed.is_empty() {
                        descriptions
                            .entry(trimmed)
                            .or_default()
                            .push(page.rel_path.clone());
                    }
                }
            }
        }

        // H1
        if cq.detect_duplicate_h1 {
            let sel = Selector::parse("h1").unwrap();
            if let Some(el) = html.select(&sel).next() {
                let text: String = el.text().collect();
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    h1s.entry(trimmed).or_default().push(page.rel_path.clone());
                }
            }
        }

        // Duplicate pages (simple hash of HTML content)
        if cq.detect_duplicate_pages {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            page.html_content.hash(&mut hasher);
            let hash = hasher.finish();
            content_hashes
                .entry(hash)
                .or_default()
                .push(page.rel_path.clone());
        }
    }

    // Report duplicates — emit one Finding per affected file for clean JSON output
    if cq.detect_duplicate_titles {
        for (title, pages) in &titles {
            if pages.len() > 1 {
                let truncated = truncate_str(title, 50);
                for page in pages {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "content/duplicate-title".into(),
                        file: page.clone(),
                        selector: "title".into(),
                        message: format!(
                            "Duplicate title '{}' shared by {} pages",
                            truncated,
                            pages.len()
                        ),
                        help: "Each page should have a unique title tag".into(),
                    });
                }
            }
        }
    }

    if cq.detect_duplicate_descriptions {
        for (desc, pages) in &descriptions {
            if pages.len() > 1 {
                let truncated = truncate_str(desc, 50);
                for page in pages {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "content/duplicate-description".into(),
                        file: page.clone(),
                        selector: "meta[name='description']".into(),
                        message: format!(
                            "Duplicate meta description '{}' shared by {} pages",
                            truncated,
                            pages.len()
                        ),
                        help: "Each page should have a unique meta description".into(),
                    });
                }
            }
        }
    }

    if cq.detect_duplicate_h1 {
        for (h1, pages) in &h1s {
            if pages.len() > 1 {
                let truncated = truncate_str(h1, 50);
                for page in pages {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "content/duplicate-h1".into(),
                        file: page.clone(),
                        selector: "h1".into(),
                        message: format!(
                            "Duplicate H1 '{}' shared by {} pages",
                            truncated,
                            pages.len()
                        ),
                        help: "Each page should have a unique H1 heading".into(),
                    });
                }
            }
        }
    }

    if cq.detect_duplicate_pages {
        for pages in content_hashes.values() {
            if pages.len() > 1 {
                for page in pages {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "content/duplicate-page".into(),
                        file: page.clone(),
                        selector: String::new(),
                        message: format!(
                            "Identical HTML content shared by {} pages",
                            pages.len()
                        ),
                        help: "These pages have identical content - consider using canonical tags or redirects".into(),
                    });
                }
            }
        }
    }

    findings
}
