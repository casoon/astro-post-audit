use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::report::Finding;

#[derive(Debug, Serialize, Deserialize)]
struct BaselineEntry {
    rule_id: String,
    file: String,
    #[serde(default)]
    selector: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BaselineFile {
    version: u32,
    findings: Vec<BaselineEntry>,
}

/// Write the current findings to `path` as a baseline file. Returns the count written.
pub fn write(findings: &[Finding], path: &str) -> Result<usize> {
    let entries: Vec<BaselineEntry> = findings
        .iter()
        .map(|f| BaselineEntry {
            rule_id: f.rule_id.clone(),
            file: f.file.clone(),
            selector: f.selector.clone(),
        })
        .collect();
    let count = entries.len();
    let file = BaselineFile {
        version: 1,
        findings: entries,
    };
    std::fs::write(path, serde_json::to_string_pretty(&file)?)?;
    Ok(count)
}

/// Filter `findings`, removing entries that already appear in the baseline at `path`.
/// Returns `(filtered_findings, suppressed_count)`.
/// If the baseline file does not exist, returns findings unchanged with suppressed = 0.
pub fn filter(findings: Vec<Finding>, path: &str) -> Result<(Vec<Finding>, usize)> {
    if !Path::new(path).exists() {
        return Ok((findings, 0));
    }
    let raw = std::fs::read_to_string(path)?;
    let baseline: BaselineFile = serde_json::from_str(&raw)?;
    let known: HashSet<(String, String, String)> = baseline
        .findings
        .into_iter()
        .map(|e| (e.rule_id, e.file, e.selector))
        .collect();
    let before = findings.len();
    let filtered: Vec<Finding> = findings
        .into_iter()
        .filter(|f| {
            !known.contains(&(f.rule_id.clone(), f.file.clone(), f.selector.clone()))
                && !known.contains(&(f.rule_id.clone(), f.file.clone(), String::new()))
        })
        .collect();
    let suppressed = before - filtered.len();
    Ok((filtered, suppressed))
}
