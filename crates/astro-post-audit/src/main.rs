use anyhow::Result;
use clap::Parser;
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process;
use std::time::Instant;

mod baseline;
mod checks;
mod config;
mod discovery;
mod hints;
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

/// Width of the textual progress bar (in cells).
const PROGRESS_BAR_WIDTH: usize = 22;

/// Build the progress line, clamped to `max_cols` so it never wraps the
/// terminal (a wrapped line can't be fully erased by a single `\x1b[2K`).
fn render_progress_line(done: usize, total: usize, label: &str, max_cols: usize) -> String {
    let filled = (done * PROGRESS_BAR_WIDTH)
        .checked_div(total)
        .unwrap_or(PROGRESS_BAR_WIDTH)
        .min(PROGRESS_BAR_WIDTH);
    let bar: String = "█".repeat(filled) + &"░".repeat(PROGRESS_BAR_WIDTH - filled);
    let line = format!("  [{bar}] {done}/{total}  {label}");
    // Leave one spare column so the cursor never lands past the last cell.
    let limit = max_cols.saturating_sub(1);
    line.chars().take(limit).collect()
}

/// Terminal width of stderr (where the bar is drawn), falling back to 80.
fn stderr_width() -> usize {
    terminal_size::terminal_size_of(std::io::stderr())
        .map(|(w, _)| w.0 as usize)
        .filter(|&w| w > 0)
        .unwrap_or(80)
}

/// Redraw the single-line progress bar on stderr. `done` checks of `total` are
/// complete; `label` names the check about to run.
fn draw_progress(done: usize, total: usize, label: &str) {
    let line = render_progress_line(done, total, label, stderr_width());
    let mut err = std::io::stderr();
    // \r returns to column 0, \x1b[2K clears the line before redrawing.
    let _ = write!(err, "\r\x1b[2K{line}");
    let _ = err.flush();
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

    let debug = config.debug;
    if debug {
        eprintln!("[debug] effective config:\n{config:#?}");
    }

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

    if debug {
        let filtered = site_index
            .html_total
            .saturating_sub(site_index.html_matched);
        eprintln!(
            "[debug] discovery: {} HTML file(s) found, {} excluded by filters, {} parsed into pages ({} ms)",
            site_index.html_total,
            filtered,
            site_index.pages.len(),
            discovery_ms
        );
        match &site_index.sitemap_parse_error {
            Some(err) => eprintln!("[debug] sitemap.xml: parse error: {err}"),
            None if site_index.sitemap_urls.is_empty() => {
                eprintln!("[debug] sitemap.xml: not found or empty")
            }
            None => eprintln!(
                "[debug] sitemap.xml: {} URL(s)",
                site_index.sitemap_urls.len()
            ),
        }
    }

    // Parse format from config
    let format = match config.format.as_deref() {
        Some("json") => report::Format::Json,
        Some("markdown") => report::Format::Markdown,
        Some("sarif") => report::Format::Sarif,
        _ => report::Format::Text,
    };

    // Live progress is drawn on stderr so it never corrupts stdout reports
    // (works with any output format). Auto-enabled only in an interactive
    // terminal (silent in CI / when piped).
    let show_verbose = !config.page_overview && !debug && config.progress_verbose;
    let show_bar = !config.page_overview
        && !debug
        && !config.progress_verbose
        && config
            .progress
            .unwrap_or_else(|| std::io::stderr().is_terminal());
    let show_progress = show_bar || show_verbose;

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

    type CheckFn = fn(&SiteIndex, &Config) -> Vec<Finding>;
    let registry: &[(&str, CheckFn)] = &[
        ("seo", checks::seo::check_all),
        ("links", checks::links::check_all),
        ("a11y", checks::a11y::check_all),
        ("html_basics", checks::html_basics::check_all),
        ("headings", checks::headings::check_all),
        ("sitemap", checks::sitemap::check_all),
        ("robots_txt", checks::robots_txt::check_all),
        ("assets", checks::assets::check_all),
        ("opengraph", checks::opengraph::check_all),
        ("structured_data", checks::structured_data::check_all),
        ("hreflang", checks::hreflang::check_all),
        ("security", checks::security::check_all),
        ("content_quality", checks::content_quality::check_all),
        ("i18n_audit", checks::i18n_audit::check_all),
        ("crawl_budget", checks::crawl_budget::check_all),
        ("render_blocking", checks::render_blocking::check_all),
        ("privacy_security", checks::privacy_security::check_all),
        (
            "structured_data_graph",
            checks::structured_data_graph::check_all,
        ),
        ("golive", checks::golive::check_all),
        ("external_links", checks::external_links::check_all),
        ("images", checks::images::check_all),
        ("ai_visibility", checks::ai_visibility::check_all),
        ("ux_heuristics", checks::ux_heuristics::check_all),
        ("redirects", checks::redirects::check_all),
        ("js_bloat", checks::js_bloat::check_all),
        ("content_sync", checks::content_sync::check_all),
        ("html_validation", checks::html_validation::check_all),
    ];

    let total_checks = registry.len();
    if show_progress {
        eprintln!("  Auditing {} pages…", site_index.pages.len());
        if show_verbose {
            eprintln!();
        }
    }

    for (idx, &(name, check_fn)) in registry.iter().enumerate() {
        if max_errors.is_some_and(|m| error_count >= m) {
            break;
        }
        if show_bar {
            // 1-based: the bar fills to total/total on the last check.
            draw_progress(idx + 1, total_checks, name);
        }
        let t = Instant::now();
        let mut new_findings = check_fn(&site_index, &config);
        let elapsed_ms = t.elapsed().as_millis();
        if !config.severity.overrides.is_empty() {
            use config::SeverityLevel;
            new_findings.retain_mut(|f| {
                if let Some(override_level) = config.severity.overrides.get(&f.rule_id) {
                    match override_level {
                        SeverityLevel::Off => return false,
                        SeverityLevel::Error => f.level = report::Level::Error,
                        SeverityLevel::Warning => f.level = report::Level::Warning,
                        SeverityLevel::Info => f.level = report::Level::Info,
                    }
                }
                true
            });
        }
        if bench {
            check_timings.push(report::CheckTiming {
                name: name.to_string(),
                duration_ms: elapsed_ms,
            });
        }
        if show_verbose {
            let n = new_findings.len();
            eprintln!("    {name:<24}  {n:>5} finding(s)   {elapsed_ms:>5}ms");
        }
        if debug {
            eprintln!(
                "[debug] {:>2}/{} {:<24} {:>4} finding(s)  {} ms",
                idx + 1,
                total_checks,
                name,
                new_findings.len(),
                elapsed_ms
            );
        }
        error_count += new_findings
            .iter()
            .filter(|f| f.level == report::Level::Error)
            .count();
        findings.extend(new_findings);
    }
    let _ = error_count;

    if show_bar {
        // Keep the completed bar visible instead of erasing it.
        let _ = writeln!(std::io::stderr());
    }

    // Populate source-file hints when enabled
    if config.hints.source_files {
        if let Some(ref root) = config.project_root {
            // Build hint map per unique file to avoid redundant filesystem lookups
            let mut hint_cache: std::collections::HashMap<String, Option<String>> =
                std::collections::HashMap::new();
            for f in &mut findings {
                let hint = hint_cache
                    .entry(f.file.clone())
                    .or_insert_with(|| hints::find_source(&f.file, root));
                f.source_hint = hint.clone();
            }
        }
    }

    let baseline_written = if let Some(ref baseline_path) = config.baseline {
        if config.write_baseline {
            baseline::write(&findings, baseline_path)?;
            true
        } else {
            findings = baseline::filter(findings, baseline_path)?.0;
            false
        }
    } else {
        false
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

    // Write extra report files (all formats from a single audit run)
    for extra in &config.extra_reports {
        let fmt = extra
            .format
            .parse::<report::Format>()
            .map_err(|e| anyhow::anyhow!("extra_reports: {e}"))?;
        let extra_reporter = Reporter::new(fmt);
        let content =
            extra_reporter.render_to_string(&findings, &summary, benchmark_data.as_ref())?;
        std::fs::write(&extra.path, content)?;
    }

    // Determine exit code
    if baseline_written {
        Ok(0)
    } else if summary.errors > 0
        || (config.strict && summary.warnings > 0)
        || config
            .max_warnings
            .is_some_and(|max| summary.warnings > max)
    {
        Ok(1)
    } else {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::render_progress_line;

    #[test]
    fn progress_line_clamped_to_terminal_width() {
        // Long check name in a narrow terminal must not exceed the width.
        let line = render_progress_line(17, 27, "structured_data_graph", 51);
        assert!(line.chars().count() <= 50, "line too wide: {:?}", line);
    }

    #[test]
    fn progress_line_full_on_wide_terminal() {
        let line = render_progress_line(17, 27, "structured_data_graph", 120);
        assert!(line.contains("17/27"));
        assert!(line.ends_with("structured_data_graph"));
    }
}
