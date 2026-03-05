use anyhow::Result;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

mod baseline;
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

    /// Base URL of the site (for URL normalization)
    #[arg(long)]
    site: Option<String>,

    /// Treat warnings as errors
    #[arg(long)]
    strict: bool,

    /// Output format
    #[arg(long, default_value = "text")]
    format: report::Format,

    /// Path to rules config file (TOML)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Maximum number of errors before aborting
    #[arg(long)]
    max_errors: Option<usize>,

    /// Include only files matching these glob patterns
    #[arg(long)]
    include: Vec<String>,

    /// Exclude files matching these glob patterns
    #[arg(long)]
    exclude: Vec<String>,

    /// Skip sitemap.xml checks
    #[arg(long)]
    no_sitemap_check: bool,

    /// Enable asset reference checking (img/src, script/src, link/href)
    #[arg(long)]
    check_assets: bool,

    /// Enable structured data (JSON-LD) validation
    #[arg(long)]
    check_structured_data: bool,

    /// Enable security heuristic checks
    #[arg(long)]
    check_security: bool,

    /// Enable content duplicate detection
    #[arg(long)]
    check_duplicates: bool,

    /// Show page properties overview (informational, no checks)
    #[arg(long)]
    page_overview: bool,

    /// Generate/update a baseline file from current findings
    #[arg(long)]
    update_baseline: bool,

    /// Path to baseline ignore file (default: .astro-post-audit-baseline)
    #[arg(long)]
    baseline: Option<PathBuf>,
}

fn main() {
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

    // Load config: explicit --config path or defaults
    let mut config = match &cli.config {
        Some(path) => Config::from_file(path)?,
        None => Config::default(),
    };

    // CLI overrides
    if let Some(ref site) = cli.site {
        config.site.base_url = Some(site.clone());
    }
    if cli.no_sitemap_check {
        config.sitemap.require = false;
        config.sitemap.canonical_must_be_in_sitemap = false;
        config.sitemap.forbid_noncanonical_in_sitemap = false;
        config.sitemap.entries_must_exist_in_dist = false;
    }
    if cli.check_assets {
        config.assets.check_broken_assets = true;
        config.assets.check_image_dimensions = true;
    }
    if cli.check_structured_data {
        config.structured_data.check_json_ld = true;
    }
    if cli.check_security {
        config.security.check_target_blank = true;
        config.security.check_mixed_content = true;
    }
    if cli.check_duplicates {
        config.content_quality.detect_duplicate_titles = true;
        config.content_quality.detect_duplicate_descriptions = true;
        config.content_quality.detect_duplicate_h1 = true;
        config.content_quality.detect_duplicate_pages = true;
    }

    // Validate dist path
    if !cli.dist_path.is_dir() {
        anyhow::bail!(
            "dist path '{}' does not exist or is not a directory",
            cli.dist_path.display()
        );
    }

    // Merge CLI and config include/exclude patterns
    let mut include = cli.include.clone();
    include.extend(config.filters.include.iter().cloned());
    let mut exclude = cli.exclude.clone();
    exclude.extend(config.filters.exclude.iter().cloned());

    // Discover HTML files and build site index
    let site_index = SiteIndex::build(&cli.dist_path, &config, &include, &exclude)?;

    // Page properties overview mode (informational, exits before checks)
    if cli.page_overview {
        let ov = overview::collect(&site_index);
        let reporter = Reporter::new(cli.format);
        reporter.print_overview(&ov)?;
        return Ok(0);
    }

    // Run all checks, with early stop if --max-errors is exceeded
    let mut findings: Vec<Finding> = Vec::new();
    let max_errors = cli.max_errors;
    let mut error_count: usize = 0;

    // Core checks (always on by default)
    macro_rules! run_check {
        ($check:expr) => {
            if !max_errors.is_some_and(|m| error_count >= m) {
                let new_findings = $check;
                error_count += new_findings
                    .iter()
                    .filter(|f| f.level == report::Level::Error)
                    .count();
                findings.extend(new_findings);
            }
        };
    }

    run_check!(checks::seo::check_all(&site_index, &config));
    run_check!(checks::links::check_all(&site_index, &config));
    run_check!(checks::a11y::check_all(&site_index, &config));
    run_check!(checks::html_basics::check_all(&site_index, &config));
    run_check!(checks::headings::check_all(&site_index, &config));

    // Sitemap checks
    if !cli.no_sitemap_check {
        run_check!(checks::sitemap::check_all(&site_index, &config));
    }

    // robots.txt checks
    run_check!(checks::robots_txt::check_all(&site_index, &config));

    // Optional checks (enabled via flags or config)
    run_check!(checks::assets::check_all(&site_index, &config));
    run_check!(checks::opengraph::check_all(&site_index, &config));
    run_check!(checks::structured_data::check_all(&site_index, &config));
    run_check!(checks::hreflang::check_all(&site_index, &config));
    run_check!(checks::security::check_all(&site_index, &config));
    run_check!(checks::content_quality::check_all(&site_index, &config));
    let _ = error_count; // used by run_check! macro for early-stop

    // --update-baseline: write baseline file and exit
    let baseline_path = cli.baseline.clone().unwrap_or_else(|| {
        cli.dist_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(".astro-post-audit-baseline")
    });
    if cli.update_baseline {
        let content = baseline::generate(&findings);
        std::fs::write(&baseline_path, &content)?;
        eprintln!(
            "Baseline written to '{}' ({} entries)",
            baseline_path.display(),
            findings.len()
        );
        return Ok(0);
    }

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

    // Apply baseline: suppress known findings
    let ignored_count = if baseline_path.exists() && !cli.update_baseline {
        let baseline_entries = baseline::load(&baseline_path);
        if !baseline_entries.is_empty() {
            let (kept, ignored) = baseline::apply(findings, &baseline_entries);
            findings = kept;
            ignored
        } else {
            0
        }
    } else {
        0
    };

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
    summary.ignored = ignored_count;
    let reporter = Reporter::new(cli.format);
    reporter.print(&findings, &summary)?;

    // Determine exit code
    if summary.errors > 0 || (cli.strict && summary.warnings > 0) {
        Ok(1)
    } else {
        Ok(0)
    }
}
