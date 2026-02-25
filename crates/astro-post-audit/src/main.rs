use anyhow::Result;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

mod checks;
mod config;
mod discovery;
mod normalize;
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

/// Auto-discover config file: check CWD and dist parent for rules.toml or .astro-post-audit.toml.
fn discover_config(dist_path: &Path) -> Option<Config> {
    let candidates = ["rules.toml", ".astro-post-audit.toml"];

    // Search locations: CWD, then dist parent directory
    let mut search_dirs = vec![std::env::current_dir().ok()];
    if let Some(parent) = dist_path.parent() {
        search_dirs.push(Some(parent.to_path_buf()));
    }

    for dir in search_dirs.into_iter().flatten() {
        for name in &candidates {
            let path = dir.join(name);
            if path.is_file() {
                match Config::from_file(&path) {
                    Ok(cfg) => return Some(cfg),
                    Err(e) => {
                        eprintln!(
                            "Warning: found config '{}' but failed to parse: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    None
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    // Load config: explicit path > auto-discovery > defaults
    let mut config = match &cli.config {
        Some(path) => Config::from_file(path)?,
        None => discover_config(&cli.dist_path).unwrap_or_default(),
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

    // Discover HTML files and build site index
    let site_index = SiteIndex::build(&cli.dist_path, &config, &cli.include, &cli.exclude)?;

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

    // Generate report
    let mut summary = Summary::from_findings(&findings);
    summary.files_checked = site_index.pages.len();
    let reporter = Reporter::new(cli.format);
    reporter.print(&findings, &summary)?;

    // Determine exit code
    if summary.errors > 0 || (cli.strict && summary.warnings > 0) {
        Ok(1)
    } else {
        Ok(0)
    }
}
