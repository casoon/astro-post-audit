use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::report::Finding;

/// Load baseline entries from a file.
/// Each line has format: `rule_id\tfile_path`
/// Lines starting with `#` are comments. Empty lines are skipped.
pub fn load(path: &Path) -> HashSet<(String, String)> {
    let mut entries = HashSet::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((rule_id, file)) = trimmed.split_once('\t') {
                entries.insert((rule_id.to_string(), file.to_string()));
            }
        }
    }
    entries
}

/// Generate baseline content from current findings.
pub fn generate(findings: &[Finding]) -> String {
    let mut lines = Vec::new();
    lines.push("# astro-post-audit baseline".to_string());
    lines.push("# Generated automatically. Each line: rule_id<TAB>file".to_string());
    lines.push("# Remove entries as you fix them.".to_string());
    lines.push(String::new());

    // Deduplicate and sort for deterministic output
    let mut entries: Vec<(&str, &str)> = findings
        .iter()
        .map(|f| (f.rule_id.as_str(), f.file.as_str()))
        .collect();
    entries.sort();
    entries.dedup();

    for (rule_id, file) in entries {
        lines.push(format!("{}\t{}", rule_id, file));
    }
    lines.push(String::new());
    lines.join("\n")
}

/// Filter out findings that match baseline entries.
/// Returns (kept_findings, ignored_count).
pub fn apply(
    findings: Vec<Finding>,
    baseline: &HashSet<(String, String)>,
) -> (Vec<Finding>, usize) {
    let mut ignored = 0usize;
    let kept: Vec<Finding> = findings
        .into_iter()
        .filter(|f| {
            let key = (f.rule_id.clone(), f.file.clone());
            if baseline.contains(&key) {
                ignored += 1;
                false
            } else {
                true
            }
        })
        .collect();
    (kept, ignored)
}
