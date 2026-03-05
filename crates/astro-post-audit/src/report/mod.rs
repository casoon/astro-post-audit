use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::str::FromStr;

use crate::overview::PageOverview;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub level: Level,
    pub rule_id: String,
    pub file: String,
    pub selector: String,
    pub message: String,
    pub help: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub files_checked: usize,
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
        }
    }
}

#[derive(Debug, Clone)]
pub enum Format {
    Text,
    Json,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Format::Text),
            "json" => Ok(Format::Json),
            _ => Err(format!("Invalid format '{}'. Use 'text' or 'json'.", s)),
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

    pub fn print(&self, findings: &[Finding], summary: &Summary) -> Result<()> {
        match self.format {
            Format::Text => self.print_text(findings, summary),
            Format::Json => self.print_json(findings, summary),
        }
    }

    fn print_text(&self, findings: &[Finding], summary: &Summary) -> Result<()> {
        if findings.is_empty() {
            println!("{}", "All checks passed!".green().bold());
            return Ok(());
        }

        // Group findings by file
        let mut by_file: std::collections::BTreeMap<&str, Vec<&Finding>> =
            std::collections::BTreeMap::new();
        for f in findings {
            by_file.entry(&f.file).or_default().push(f);
        }

        for (file, file_findings) in &by_file {
            println!("\n{}", file.bold().underline());
            for f in file_findings {
                let level_str = match f.level {
                    Level::Error => "ERROR".red().bold(),
                    Level::Warning => "WARN".yellow().bold(),
                    Level::Info => "INFO".blue(),
                };
                println!("  {} [{}] {}", level_str, f.rule_id.dimmed(), f.message);
                if !f.selector.is_empty() {
                    println!("    {} {}", "at".dimmed(), f.selector.dimmed());
                }
                if !f.help.is_empty() {
                    println!("    {} {}", "fix:".cyan(), f.help);
                }
            }
        }

        println!();
        println!(
            "{}: {} errors, {} warnings, {} info",
            "Summary".bold(),
            summary.errors.to_string().red(),
            summary.warnings.to_string().yellow(),
            summary.info.to_string().blue(),
        );

        Ok(())
    }

    fn print_json(&self, findings: &[Finding], summary: &Summary) -> Result<()> {
        #[derive(Serialize)]
        struct Report<'a> {
            findings: &'a [Finding],
            summary: &'a Summary,
        }

        let report = Report { findings, summary };
        println!("{}", serde_json::to_string_pretty(&report)?);
        Ok(())
    }

    pub fn print_overview(&self, overview: &PageOverview) -> Result<()> {
        match self.format {
            Format::Text => self.print_overview_text(overview),
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
        println!(
            "  {}",
            "─".repeat(header.len().saturating_sub(2)).dimmed()
        );

        // Rows
        for p in &overview.pages {
            let file_display = if p.file.len() > max_file_len {
                format!("…{}", &p.file[p.file.len() - max_file_len + 1..])
            } else {
                p.file.clone()
            };

            let check = |b: bool| if b { "✓".green().to_string() } else { "✗".red().to_string() };
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
            println!(
                "\n{}:  {}",
                "JSON-LD Types".bold(),
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
