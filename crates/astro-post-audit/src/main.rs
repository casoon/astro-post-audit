use anyhow::Result;
use clap::Parser;
use std::io::Read;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

mod checks;
mod config;
mod discovery;
mod normalize;
mod overview;
mod report;

use config::Config;
use discovery::SiteIndex;
use report::{Finding, Reporter, Summary};

#[derive(Parser, Debug)]
#[command(name = "astro-post-audit")]
#[command(
    about = "Fast post-build auditor for Astro sites: SEO, links, and lightweight WCAG checks"
)]
#[command(version)]
struct Cli {
    /// Path to the dist/ directory to audit
    #[arg(default_value = "dist")]
    dist_path: PathBuf,

    /// Read JSON config from stdin (all options are passed via JSON)
    #[arg(long)]
    config_stdin: bool,
}

fn main() {
    // Install miette's fancy graphical handler for any unhandled errors
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::GraphicalReportHandler::new().with_theme(miette::GraphicalTheme::unicode()),
        )
    }))
    .ok();

    match run() {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("Error: {e:#}");
            process::exit(2);
        }
    }
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    // Load config: --config-stdin (JSON) or defaults
    let config = if cli.config_stdin {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Config::from_json(&buf)?
    } else {
        Config::default()
    };
    config.validate()?;

    // Validate dist path
    if !cli.dist_path.is_dir() {
        anyhow::bail!(
            "dist path '{}' does not exist or is not a directory",
            cli.dist_path.display()
        );
    }

    // Discover HTML files and build site index
    let bench = config.benchmark;
    let t_start = Instant::now();
    let include = &config.filters.include;
    let exclude = &config.filters.exclude;
    let site_index = SiteIndex::build(&cli.dist_path, &config, include, exclude)?;
    let discovery_ms = t_start.elapsed().as_millis();

    // Parse format from config
    let format = match config.format.as_deref() {
        Some("json") => report::Format::Json,
        _ => report::Format::Text,
    };

    // Page properties overview mode (informational, exits before checks)
    if config.page_overview {
        let ov = overview::collect(&site_index);
        let reporter = Reporter::new(format);
        reporter.print_overview(&ov)?;
        return Ok(0);
    }

    // Run all checks, with early stop if --max-errors is exceeded
    let mut findings: Vec<Finding> = Vec::new();
    let max_errors = config.max_errors;
    let mut error_count: usize = 0;
    let mut check_timings: Vec<report::CheckTiming> = Vec::new();

    // Core checks (always on by default)
    macro_rules! run_check {
        ($name:expr, $check:expr) => {
            if !max_errors.is_some_and(|m| error_count >= m) {
                let t = Instant::now();
                let new_findings = $check;
                if bench {
                    check_timings.push(report::CheckTiming {
                        name: $name.to_string(),
                        duration_ms: t.elapsed().as_millis(),
                    });
                }
                error_count += new_findings
                    .iter()
                    .filter(|f| f.level == report::Level::Error)
                    .count();
                findings.extend(new_findings);
            }
        };
    }

    run_check!("seo", checks::seo::check_all(&site_index, &config));
    run_check!("links", checks::links::check_all(&site_index, &config));
    run_check!("a11y", checks::a11y::check_all(&site_index, &config));
    run_check!(
        "html_basics",
        checks::html_basics::check_all(&site_index, &config)
    );
    run_check!(
        "headings",
        checks::headings::check_all(&site_index, &config)
    );
    run_check!("sitemap", checks::sitemap::check_all(&site_index, &config));
    run_check!(
        "robots_txt",
        checks::robots_txt::check_all(&site_index, &config)
    );
    run_check!("assets", checks::assets::check_all(&site_index, &config));
    run_check!(
        "opengraph",
        checks::opengraph::check_all(&site_index, &config)
    );
    run_check!(
        "structured_data",
        checks::structured_data::check_all(&site_index, &config)
    );
    run_check!(
        "hreflang",
        checks::hreflang::check_all(&site_index, &config)
    );
    run_check!(
        "security",
        checks::security::check_all(&site_index, &config)
    );
    run_check!(
        "content_quality",
        checks::content_quality::check_all(&site_index, &config)
    );
    run_check!(
        "external_links",
        checks::external_links::check_all(&site_index, &config)
    );
    let _ = error_count; // used by run_check! macro for early-stop

    // Apply severity overrides from [severity] config section
    if !config.severity.overrides.is_empty() {
        use config::SeverityLevel;
        findings.retain_mut(|f| {
            if let Some(override_level) = config.severity.overrides.get(&f.rule_id) {
                match override_level {
                    SeverityLevel::Off => return false, // remove finding entirely
                    SeverityLevel::Error => f.level = report::Level::Error,
                    SeverityLevel::Warning => f.level = report::Level::Warning,
                    SeverityLevel::Info => f.level = report::Level::Info,
                }
            }
            true
        });
    }

    // Enforce exact --max-errors cap: keep only the first N errors (plus all non-errors before them)
    let truncated = if let Some(max) = max_errors {
        let total_errors = findings
            .iter()
            .filter(|f| f.level == report::Level::Error)
            .count();
        if total_errors > max {
            let mut error_seen = 0usize;
            findings.retain(|f| {
                if f.level == report::Level::Error {
                    error_seen += 1;
                    error_seen <= max
                } else {
                    true // keep all warnings/info
                }
            });
            true
        } else {
            false
        }
    } else {
        false
    };

    // Generate report
    let mut summary = Summary::from_findings(&findings);
    summary.files_checked = site_index.pages.len();
    summary.truncated = truncated;

    let benchmark_data = if bench {
        let total_ms = t_start.elapsed().as_millis();
        let pages = site_index.pages.len();
        Some(report::BenchmarkData {
            discovery_ms,
            check_timings,
            total_ms,
            pages_checked: pages,
            pages_per_second: if total_ms > 0 {
                pages as f64 / (total_ms as f64 / 1000.0)
            } else {
                0.0
            },
        })
    } else {
        None
    };

    let reporter = Reporter::new(format);
    reporter.print(&findings, &summary, benchmark_data.as_ref())?;

    // Determine exit code
    if summary.errors > 0 || (config.strict && summary.warnings > 0) {
        Ok(1)
    } else {
        Ok(0)
    }
}
