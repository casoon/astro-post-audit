use anyhow::Result;
use runemark::{
    ColorMode, Confidence as RunemarkConfidence, Console, DetailLevel, Finding as RunemarkFinding,
    FindingGroup, Location, Metric, Report as RunemarkReport, ScopeNote, Tone, Verdict,
};
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

impl Finding {
    /// Creates a finding with its common optional fields initialized consistently.
    pub(crate) fn new(
        level: Level,
        rule_id: impl Into<String>,
        file: impl Into<String>,
        selector: impl Into<String>,
        message: impl Into<String>,
        help: impl Into<String>,
        confidence: Option<Confidence>,
    ) -> Self {
        Self {
            level,
            rule_id: rule_id.into(),
            file: file.into(),
            selector: selector.into(),
            message: message.into(),
            help: help.into(),
            suggestion: None,
            source_hint: None,
            confidence,
        }
    }
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
    Html,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Format::Text),
            "json" => Ok(Format::Json),
            "markdown" => Ok(Format::Markdown),
            "sarif" => Ok(Format::Sarif),
            "html" => Ok(Format::Html),
            _ => Err(format!(
                "Invalid format '{}'. Use 'text', 'json', 'markdown', 'sarif', or 'html'.",
                s
            )),
        }
    }
}

fn tone_for(level: &Level) -> Tone {
    match level {
        Level::Error => Tone::Error,
        Level::Warning => Tone::Warning,
        Level::Info => Tone::Info,
    }
}

fn verdict_for(summary: &Summary) -> Verdict {
    if summary.errors > 0 {
        Verdict::Failed
    } else if summary.warnings > 0 {
        Verdict::Warning
    } else {
        Verdict::Info
    }
}

fn runemark_confidence(confidence: &Option<Confidence>) -> Option<RunemarkConfidence> {
    match confidence {
        Some(Confidence::Medium) => Some(RunemarkConfidence::Medium),
        Some(Confidence::Low) => Some(RunemarkConfidence::Low),
        None => None,
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
            Format::Html => {
                print!("{}", self.render_html(findings, summary));
                Ok(())
            }
        }
    }

    pub fn render_to_string(
        &self,
        findings: &[Finding],
        summary: &Summary,
        benchmark: Option<&BenchmarkData>,
    ) -> Result<String> {
        match self.format {
            Format::Json => {
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
                Ok(serde_json::to_string_pretty(&report)?)
            }
            Format::Markdown => Ok(self.render_markdown(findings, summary)),
            Format::Sarif => self.render_sarif(findings),
            Format::Html => Ok(self.render_html(findings, summary)),
            Format::Text => Err(anyhow::anyhow!(
                "text format cannot be rendered to a string; use print() for stdout output"
            )),
        }
    }

    fn print_text(&self, findings: &[Finding], summary: &Summary) -> Result<()> {
        let console = Console::stdout(ColorMode::Auto);
        let mut report = if findings.is_empty() {
            RunemarkReport::new("All checks passed!", Verdict::Passed)
        } else {
            RunemarkReport::new("astro-post-audit", verdict_for(summary))
                .add_metric(
                    Metric::new("Errors", summary.errors.to_string()).with_tone(Tone::Error),
                )
                .add_metric(
                    Metric::new("Warnings", summary.warnings.to_string()).with_tone(Tone::Warning),
                )
                .add_metric(Metric::new("Info", summary.info.to_string()).with_tone(Tone::Info))
                .add_metric(
                    Metric::new("Files", summary.files_checked.to_string()).with_tone(Tone::Muted),
                )
                .with_detail_level(DetailLevel::Detailed)
        };

        if summary.truncated {
            report = report.add_scope_note(ScopeNote::new(
                "Output",
                vec!["Truncated due to the max-errors limit".into()],
            ));
        }

        let mut by_file: std::collections::BTreeMap<&str, Vec<&Finding>> =
            std::collections::BTreeMap::new();
        for f in findings {
            by_file.entry(&f.file).or_default().push(f);
        }

        for (file, file_findings) in by_file {
            let source_hint = file_findings.first().and_then(|f| f.source_hint.as_deref());
            let title = source_hint
                .map(|hint| format!("{file} (source: {hint}, heuristic)"))
                .unwrap_or_else(|| file.to_string());
            let mut group = FindingGroup::new(title).with_advisory(
                file_findings
                    .iter()
                    .all(|finding| finding.level == Level::Info),
            );
            for f in file_findings {
                let mut finding =
                    RunemarkFinding::new(tone_for(&f.level), &f.message).with_rule_id(&f.rule_id);
                if !f.selector.is_empty() {
                    finding = finding.with_location(Location::Selector(f.selector.clone()));
                }
                if !f.help.is_empty() {
                    finding = finding.with_remedy(&f.help);
                }
                if let Some(confidence) = runemark_confidence(&f.confidence) {
                    finding = finding.with_confidence(confidence);
                }
                group = group.add_finding(finding);
            }
            report = report.add_group(group);
        }

        print!("\n{}\n", report.render(console));
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

    fn render_html(&self, findings: &[Finding], summary: &Summary) -> String {
        let mut out = String::new();
        out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
        out.push_str("<title>astro-post-audit Audit Report</title>\n");
        out.push_str("<style>\n");
        out.push_str("body { font-family: system-ui, -apple-system, sans-serif; background: #0f172a; color: #f8fafc; margin: 0; padding: 2rem; }\n");
        out.push_str("h1 { color: #38bdf8; margin-top: 0; }\n");
        out.push_str(".summary { display: flex; gap: 1.5rem; margin-bottom: 2rem; background: #1e293b; padding: 1rem 1.5rem; border-radius: 8px; font-weight: bold; }\n");
        out.push_str(
            ".error { color: #f87171; }\n.warning { color: #fbbf24; }\n.info { color: #38bdf8; }\n",
        );
        out.push_str("table { width: 100%; border-collapse: collapse; background: #1e293b; border-radius: 8px; overflow: hidden; }\n");
        out.push_str("th, td { text-align: left; padding: 0.75rem 1rem; border-bottom: 1px solid #334155; font-size: 0.875rem; }\n");
        out.push_str("th { background: #334155; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; }\n");
        out.push_str("tr:hover { background: #334155; }\n");
        out.push_str(".badge { display: inline-block; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: bold; text-transform: uppercase; }\n");
        out.push_str(".badge-error { background: #7f1d1d; color: #fca5a5; }\n");
        out.push_str(".badge-warning { background: #78350f; color: #fde68a; }\n");
        out.push_str(".badge-info { background: #0c4a6e; color: #7dd3fc; }\n");
        out.push_str("code { background: #0f172a; padding: 0.2rem 0.4rem; border-radius: 4px; font-family: monospace; font-size: 0.85rem; color: #e2e8f0; }\n");
        out.push_str("</style>\n</head>\n<body>\n");
        out.push_str("<h1>🚀 astro-post-audit Audit Report</h1>\n");
        out.push_str("<div class=\"summary\">\n");
        let _ = writeln!(out, "<div class=\"error\">Errors: {}</div>", summary.errors);
        let _ = writeln!(
            out,
            "<div class=\"warning\">Warnings: {}</div>",
            summary.warnings
        );
        let _ = writeln!(out, "<div class=\"info\">Info: {}</div>", summary.info);
        let _ = writeln!(out, "<div>Files Checked: {}</div>", summary.files_checked);
        out.push_str("</div>\n");

        if findings.is_empty() {
            out.push_str(
                "<p style=\"color: #4ade80; font-weight: bold;\">✓ No issues found!</p>\n",
            );
        } else {
            out.push_str("<table>\n<thead>\n<tr><th>Level</th><th>Rule ID</th><th>File</th><th>Selector</th><th>Message</th></tr>\n</thead>\n<tbody>\n");
            for f in findings {
                let badge_cls = match f.level {
                    Level::Error => "badge-error",
                    Level::Warning => "badge-warning",
                    Level::Info => "badge-info",
                };
                let lvl_str = match f.level {
                    Level::Error => "error",
                    Level::Warning => "warning",
                    Level::Info => "info",
                };
                let _ = writeln!(
                    out,
                    "<tr><td><span class=\"badge {}\">{}</span></td><td><code>{}</code></td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td></tr>",
                    badge_cls, lvl_str, html_escape(&f.rule_id), html_escape(&f.file), html_escape(&f.selector), html_escape(&f.message)
                );
            }
            out.push_str("</tbody>\n</table>\n");
        }

        out.push_str("</body>\n</html>\n");
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
        let console = Console::stdout(ColorMode::Auto);
        println!(
            "  {} {} ({} pages)",
            console.paint(Tone::Title, "Benchmark"),
            console.paint(Tone::Muted, format!("{}ms total", b.total_ms)),
            b.pages_checked
        );
        println!(
            "    {} Discovery: {}ms",
            console.paint(Tone::Muted, "•"),
            b.discovery_ms
        );
        for t in &b.check_timings {
            println!(
                "    {} {}: {}ms",
                console.paint(Tone::Muted, "•"),
                t.name,
                t.duration_ms
            );
        }
        println!(
            "    {} {:.1} pages/sec",
            console.paint(Tone::Muted, "•"),
            b.pages_per_second
        );
        println!();
        Ok(())
    }

    pub fn print_overview(&self, overview: &PageOverview) -> Result<()> {
        match self.format {
            Format::Text | Format::Markdown | Format::Sarif | Format::Html => {
                self.print_overview_text(overview)
            }
            Format::Json => self.print_overview_json(overview),
        }
    }

    fn print_overview_text(&self, overview: &PageOverview) -> Result<()> {
        let stats = &overview.stats;
        let console = Console::stdout(ColorMode::Auto);

        println!(
            "\n{}",
            console.paint(
                Tone::Title,
                format!("Page Properties Overview ({} pages)", stats.total_pages)
            )
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
        println!("{}", console.paint(Tone::Muted, &header));
        println!(
            "  {}",
            console.paint(Tone::Muted, "─".repeat(header.len().saturating_sub(2)))
        );

        // Rows
        for p in &overview.pages {
            let file_display = if p.file.len() > max_file_len {
                format!("…{}", &p.file[p.file.len() - max_file_len + 1..])
            } else {
                p.file.clone()
            };

            let check = |b: bool| {
                if b {
                    console.paint(Tone::Success, "✓")
                } else {
                    console.paint(Tone::Error, "✗")
                }
            };
            let og_all = p.has_og_title && p.has_og_description && p.has_og_image;

            let h1_str = if p.h1_count == 0 {
                console.paint(Tone::Error, "✗")
            } else {
                p.h1_count.to_string()
            };

            let lang_str = match &p.lang_value {
                Some(v) => v.clone(),
                None => console.paint(Tone::Error, "✗"),
            };

            let ld_types_str = if p.json_ld_types.is_empty() {
                console.paint(Tone::Muted, "—")
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
            let rendered = if count == total {
                console.paint(Tone::Success, ratio)
            } else if count == 0 {
                console.paint(Tone::Error, ratio)
            } else {
                console.paint(Tone::Warning, ratio)
            };
            format!("{} {}", label, rendered)
        };

        println!(
            "{}:  {}  ·  {}  ·  {}  ·  {}  ·  {}  ·  {}  ·  {}  ·  {}",
            console.paint(Tone::Title, "Summary"),
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
                console.paint(Tone::Warning, "Noindex:"),
                console.paint(Tone::Warning, format!("{} pages", stats.pages_with_noindex))
            );
        }

        // JSON-LD types
        if !stats.json_ld_type_counts.is_empty() {
            let types_str: Vec<String> = stats
                .json_ld_type_counts
                .iter()
                .map(|(t, c)| format!("{} ×{}", t, c))
                .collect();
            println!(
                "\n{}:  {}",
                console.paint(Tone::Title, "JSON-LD Types"),
                types_str.join("  ·  ")
            );
        }

        println!();
        Ok(())
    }

    fn print_overview_json(&self, overview: &PageOverview) -> Result<()> {
        println!("{}", serde_json::to_string_pretty(overview)?);
        Ok(())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
