use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::fmt::Write as FmtWrite;
use std::str::FromStr;

use crate::overview::PageOverview;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub level: Level,
    pub rule_id: String,
    pub file: String,
    pub selector: String,
    pub message: String,
    pub help: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub files_checked: usize,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

impl Summary {
    pub fn from_findings(findings: &[Finding]) -> Self {
        Self {
            errors: findings.iter().filter(|f| f.level == Level::Error).count(),
            warnings: findings
                .iter()
                .filter(|f| f.level == Level::Warning)
                .count(),
            info: findings.iter().filter(|f| f.level == Level::Info).count(),
            files_checked: 0, // set externally
            truncated: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkData {
    pub discovery_ms: u128,
    pub check_timings: Vec<CheckTiming>,
    pub total_ms: u128,
    pub pages_checked: usize,
    pub pages_per_second: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckTiming {
    pub name: String,
    pub duration_ms: u128,
}

#[derive(Debug, Clone)]
pub enum Format {
    Text,
    Json,
    Markdown,
    Sarif,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Format::Text),
            "json" => Ok(Format::Json),
            "markdown" => Ok(Format::Markdown),
            "sarif" => Ok(Format::Sarif),
            _ => Err(format!(
                "Invalid format '{}'. Use 'text', 'json', 'markdown', or 'sarif'.",
                s
            )),
        }
    }
}

pub struct Reporter {
    format: Format,
}

impl Reporter {
    pub fn new(format: Format) -> Self {
        Self { format }
    }

    pub fn print(
        &self,
        findings: &[Finding],
        summary: &Summary,
        benchmark: Option<&BenchmarkData>,
    ) -> Result<()> {
        match self.format {
            Format::Text => {
                self.print_text(findings, summary)?;
                if let Some(b) = benchmark {
                    self.print_benchmark_text(b)?;
                }
                Ok(())
            }
            Format::Json => self.print_json(findings, summary, benchmark),
            Format::Markdown => {
                print!("{}", self.render_markdown(findings, summary));
                Ok(())
            }
            Format::Sarif => {
                println!("{}", self.render_sarif(findings)?);
                Ok(())
            }
        }
    }

    fn print_text(&self, findings: &[Finding], summary: &Summary) -> Result<()> {
        if findings.is_empty() {
            println!(
                "\n  {} {}",
                "✓".green().bold(),
                "All checks passed!".green().bold()
            );
            println!();
            return Ok(());
        }

        // Group findings by file
        let mut by_file: std::collections::BTreeMap<&str, Vec<&Finding>> =
            std::collections::BTreeMap::new();
        for f in findings {
            by_file.entry(&f.file).or_default().push(f);
        }

        for (file, file_findings) in &by_file {
            // File header with miette-style location marker
            // Show source hint if present (same hint for all findings in this file)
            let source_hint = file_findings
                .first()
                .and_then(|f| f.source_hint.as_deref());
            println!();
            println!("  {} {}", "──▶".dimmed(), file.bold().underline());
            if let Some(hint) = source_hint {
                println!(
                    "       {} {} {}",
                    "source:".dimmed(),
                    hint.dimmed(),
                    "(heuristic)".dimmed()
                );
            }

            for f in file_findings {
                // Severity marker with miette-style symbols
                let (marker, level_label) = match f.level {
                    Level::Error => ("×".red().bold(), "error".red().bold()),
                    Level::Warning => ("⚠".yellow().bold(), "warning".yellow().bold()),
                    Level::Info => ("ℹ".blue(), "info".blue()),
                };

                // Rule ID and message
                let confidence_tag = match &f.confidence {
                    Some(Confidence::Medium) => " (confidence: medium)".dimmed().to_string(),
                    Some(Confidence::Low) => " (confidence: low)".dimmed().to_string(),
                    None => String::new(),
                };
                println!(
                    "  {} {}{} {}{}",
                    marker,
                    level_label,
                    format!("[{}]", f.rule_id).dimmed(),
                    f.message,
                    confidence_tag
                );

                // Selector (location within the HTML)
                if !f.selector.is_empty() {
                    println!("    {} {}", "╰─▶".dimmed(), f.selector.dimmed());
                }

                // Help text with miette-style formatting
                if !f.help.is_empty() {
                    println!("    {} {}", "help:".cyan().bold(), f.help);
                }
            }
        }

        // Summary box
        println!();
        let mut summary_line = String::new();
        if summary.errors > 0 {
            write!(
                summary_line,
                "{} error{}",
                summary.errors,
                if summary.errors == 1 { "" } else { "s" }
            )
            .unwrap();
        }
        if summary.warnings > 0 {
            if !summary_line.is_empty() {
                summary_line.push_str(", ");
            }
            write!(
                summary_line,
                "{} warning{}",
                summary.warnings,
                if summary.warnings == 1 { "" } else { "s" }
            )
            .unwrap();
        }
        if summary.info > 0 {
            if !summary_line.is_empty() {
                summary_line.push_str(", ");
            }
            write!(summary_line, "{} info", summary.info).unwrap();
        }

        let status_icon = if summary.errors > 0 {
            "×".red().bold()
        } else {
            "⚠".yellow().bold()
        };

        println!(
            "  {} {} ({} file{} checked)",
            status_icon,
            summary_line.bold(),
            summary.files_checked,
            if summary.files_checked == 1 { "" } else { "s" }
        );

        if summary.truncated {
            println!(
                "    {} {}",
                "note:".cyan().bold(),
                "output truncated due to max-errors limit".dimmed()
            );
        }

        println!();
        Ok(())
    }

    fn print_json(
        &self,
        findings: &[Finding],
        summary: &Summary,
        benchmark: Option<&BenchmarkData>,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct Report<'a> {
            findings: &'a [Finding],
            summary: &'a Summary,
            #[serde(skip_serializing_if = "Option::is_none")]
            benchmark: Option<&'a BenchmarkData>,
        }

        let report = Report {
            findings,
            summary,
            benchmark,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        Ok(())
    }

    fn render_markdown(&self, findings: &[Finding], summary: &Summary) -> String {
        let mut out = String::new();

        out.push_str("# astro-post-audit\n\n");
        out.push_str(&format!(
            "{} pages checked · {} errors · {} warnings · {} info\n",
            summary.files_checked, summary.errors, summary.warnings, summary.info
        ));

        if findings.is_empty() {
            out.push_str("\nAll checks passed!\n");
            return out;
        }

        let escape = |s: &str| s.replace('|', "\\|");

        for (level, heading) in &[
            (Level::Error, "## Errors"),
            (Level::Warning, "## Warnings"),
            (Level::Info, "## Info"),
        ] {
            let level_findings: Vec<&Finding> =
                findings.iter().filter(|f| f.level == *level).collect();
            if level_findings.is_empty() {
                continue;
            }
            out.push('\n');
            out.push_str(heading);
            out.push_str("\n\n");
            out.push_str("| File | Rule | Message |\n");
            out.push_str("|------|------|----------|\n");
            for f in &level_findings {
                out.push_str(&format!(
                    "| {} | `{}` | {} |\n",
                    escape(&f.file),
                    escape(&f.rule_id),
                    escape(&f.message)
                ));
            }
        }

        if summary.truncated {
            out.push_str("\n> **Note:** Output truncated due to max-errors limit.\n");
        }

        out
    }

    fn render_sarif(&self, findings: &[Finding]) -> Result<String> {
        // Collect unique rules (stable order via BTreeMap)
        let mut rule_map: std::collections::BTreeMap<&str, &Finding> =
            std::collections::BTreeMap::new();
        for f in findings {
            rule_map.entry(&f.rule_id).or_insert(f);
        }
        let rule_ids: Vec<&str> = rule_map.keys().copied().collect();
        let rule_index: std::collections::HashMap<&str, usize> =
            rule_ids.iter().enumerate().map(|(i, r)| (*r, i)).collect();

        let sarif_rules: Vec<serde_json::Value> = rule_ids
            .iter()
            .map(|id| {
                let f = rule_map[id];
                serde_json::json!({
                    "id": id,
                    "shortDescription": { "text": f.help.as_str() }
                })
            })
            .collect();

        let sarif_results: Vec<serde_json::Value> = findings
            .iter()
            .map(|f| {
                let level = match f.level {
                    Level::Error => "error",
                    Level::Warning => "warning",
                    Level::Info => "note",
                };
                serde_json::json!({
                    "ruleId": f.rule_id,
                    "ruleIndex": rule_index[f.rule_id.as_str()],
                    "level": level,
                    "message": { "text": f.message },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": f.file,
                                "uriBaseId": "%SRCROOT%"
                            }
                        }
                    }]
                })
            })
            .collect();

        let sarif = serde_json::json!({
            "$schema": "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "astro-post-audit",
                        "informationUri": "https://github.com/casoon/astro-post-audit",
                        "rules": sarif_rules
                    }
                },
                "results": sarif_results
            }]
        });

        Ok(serde_json::to_string_pretty(&sarif)?)
    }

    fn print_benchmark_text(&self, b: &BenchmarkData) -> Result<()> {
        println!(
            "  {} {} ({} pages)",
            "Benchmark".bold().underline(),
            format!("{}ms total", b.total_ms).dimmed(),
            b.pages_checked
        );
        println!("    {} Discovery: {}ms", "•".dimmed(), b.discovery_ms);
        for t in &b.check_timings {
            println!("    {} {}: {}ms", "•".dimmed(), t.name, t.duration_ms);
        }
        println!("    {} {:.1} pages/sec", "•".dimmed(), b.pages_per_second);
        println!();
        Ok(())
    }

    pub fn print_overview(&self, overview: &PageOverview) -> Result<()> {
        match self.format {
            Format::Text | Format::Markdown | Format::Sarif => {
                self.print_overview_text(overview)
            }
            Format::Json => self.print_overview_json(overview),
        }
    }

    fn print_overview_text(&self, overview: &PageOverview) -> Result<()> {
        let stats = &overview.stats;

        println!(
            "\n{}",
            format!("Page Properties Overview ({} pages)", stats.total_pages)
                .bold()
                .underline()
        );
        println!();

        // Determine max file path width
        let max_file_len = overview
            .pages
            .iter()
            .map(|p| p.file.len())
            .max()
            .unwrap_or(20)
            .min(50);

        // Header
        let header = format!(
            "  {:<width$}  Title  Desc  Canon  OG  H1  Lang  LD  Skip  LD Types",
            "File",
            width = max_file_len
        );
        println!("{}", header.dimmed());
        println!("  {}", "─".repeat(header.len().saturating_sub(2)).dimmed());

        // Rows
        for p in &overview.pages {
            let file_display = if p.file.len() > max_file_len {
                format!("…{}", &p.file[p.file.len() - max_file_len + 1..])
            } else {
                p.file.clone()
            };

            let check = |b: bool| {
                if b {
                    "✓".green().to_string()
                } else {
                    "✗".red().to_string()
                }
            };
            let og_all = p.has_og_title && p.has_og_description && p.has_og_image;

            let h1_str = if p.h1_count == 0 {
                "✗".red().to_string()
            } else {
                p.h1_count.to_string()
            };

            let lang_str = match &p.lang_value {
                Some(v) => v.clone(),
                None => "✗".red().to_string(),
            };

            let ld_types_str = if p.json_ld_types.is_empty() {
                "—".dimmed().to_string()
            } else {
                p.json_ld_types.join(", ")
            };

            println!(
                "  {:<width$}  {:^5}  {:^4}  {:^5}  {:^2}  {:>2}  {:^4}  {:^2}  {:^4}   {}",
                file_display,
                check(p.title.is_some()),
                check(p.meta_description.is_some()),
                check(p.has_canonical),
                check(og_all),
                h1_str,
                lang_str,
                check(p.has_json_ld),
                check(p.has_skip_link),
                ld_types_str,
                width = max_file_len
            );
        }

        // Summary
        println!();
        let stat = |label: &str, count: usize, total: usize| {
            let ratio = format!("{}/{}", count, total);
            let colored = if count == total {
                ratio.green().to_string()
            } else if count == 0 {
                ratio.red().to_string()
            } else {
                ratio.yellow().to_string()
            };
            format!("{} {}", label, colored)
        };

        println!(
            "{}:  {}  ·  {}  ·  {}  ·  {}  ·  {}  ·  {}  ·  {}  ·  {}",
            "Summary".bold(),
            stat("Title", stats.pages_with_title, stats.total_pages),
            stat("Desc", stats.pages_with_description, stats.total_pages),
            stat("Canonical", stats.pages_with_canonical, stats.total_pages),
            stat("OG", stats.pages_with_og_title, stats.total_pages),
            stat("H1", stats.pages_with_h1, stats.total_pages),
            stat("Lang", stats.pages_with_lang, stats.total_pages),
            stat("JSON-LD", stats.pages_with_json_ld, stats.total_pages),
            stat("Skip", stats.pages_with_skip_link, stats.total_pages),
        );

        if stats.pages_with_noindex > 0 {
            println!(
                "  {} {}",
                "Noindex:".yellow(),
                format!("{} pages", stats.pages_with_noindex).yellow()
            );
        }

        // JSON-LD types
        if !stats.json_ld_type_counts.is_empty() {
            let types_str: Vec<String> = stats
                .json_ld_type_counts
                .iter()
                .map(|(t, c)| format!("{} ×{}", t, c))
                .collect();
            println!("\n{}:  {}", "JSON-LD Types".bold(), types_str.join("  ·  "));
        }

        println!();
        Ok(())
    }

    fn print_overview_json(&self, overview: &PageOverview) -> Result<()> {
        println!("{}", serde_json::to_string_pretty(overview)?);
        Ok(())
    }
}
