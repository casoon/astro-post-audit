use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::str::FromStr;

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
}
