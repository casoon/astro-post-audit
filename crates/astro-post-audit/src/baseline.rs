use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::report::{self, Finding};

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
            file: report::datei_von(f).to_string(),
            selector: f.location.selector.clone().unwrap_or_default(),
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
    // Eine committete Baseline kann noch die alten Kennungen tragen. Sie wird
    // beim Einlesen uebersetzt, nicht beim Schreiben -- geschrieben wird
    // ausschliesslich neu.
    let known: HashSet<(String, String, String)> = baseline
        .findings
        .into_iter()
        .flat_map(|e| {
            crate::rule_ids::beide_kennungen(&e.rule_id, "baseline file")
                .into_iter()
                .map(move |kennung| (kennung, e.file.clone(), e.selector.clone()))
        })
        .collect();
    let before = findings.len();
    let filtered: Vec<Finding> = findings
        .into_iter()
        .filter(|f| {
            !known.contains(&(
                f.rule_id.clone(),
                report::datei_von(f).to_string(),
                f.location.selector.clone().unwrap_or_default(),
            )) && !known.contains(&(
                f.rule_id.clone(),
                report::datei_von(f).to_string(),
                String::new(),
            ))
        })
        .collect();
    let suppressed = before - filtered.len();
    Ok((filtered, suppressed))
}
