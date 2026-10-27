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

/// The findings recorded in a baseline file, keyed by rule id, file and selector.
pub struct Baseline {
    known: HashSet<(String, String, String)>,
}

impl Baseline {
    /// Load the baseline at `path`; `None` if the file does not exist yet.
    pub fn load(path: &str) -> Result<Option<Self>> {
        if !Path::new(path).exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(path)?;
        let baseline: BaselineFile = serde_json::from_str(&raw)?;
        // Eine committete Baseline kann noch die alten Kennungen tragen. Sie wird
        // beim Einlesen uebersetzt, nicht beim Schreiben -- geschrieben wird
        // ausschliesslich neu.
        let known = baseline
            .findings
            .into_iter()
            .flat_map(|e| {
                crate::rule_ids::beide_kennungen(&e.rule_id, "baseline file")
                    .into_iter()
                    .map(move |kennung| (kennung, e.file.clone(), e.selector.clone()))
            })
            .collect();
        Ok(Some(Self { known }))
    }

    /// Whether `finding` is already recorded and therefore suppressed.
    pub fn contains(&self, finding: &Finding) -> bool {
        let file = report::datei_von(finding).to_string();
        let selector = finding.location.selector.clone().unwrap_or_default();
        self.known
            .contains(&(finding.rule_id.clone(), file.clone(), selector))
            || self
                .known
                .contains(&(finding.rule_id.clone(), file, String::new()))
    }
}
