use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use html_conform::Severity as ConformSeverity;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};
const ASTRO_ISLAND_RUNTIME_STYLE: &str =
    "<style>astro-island,astro-slot,astro-static-slot{display:contents}</style>";

fn html_for_validation(html: &str) -> Cow<'_, str> {
    if !html.contains(ASTRO_ISLAND_RUNTIME_STYLE) {
        return Cow::Borrowed(html);
    }

    // Astro injects this exact style next to its first hydrated island. It is
    // framework runtime output, not author markup. Blanking it at equal byte
    // length keeps all source locations from html-conform stable.
    Cow::Owned(html.replace(
        ASTRO_ISLAND_RUNTIME_STYLE,
        &" ".repeat(ASTRO_ISLAND_RUNTIME_STYLE.len()),
    ))
}

/// Result cache for `html_validation`.
///
/// A page's findings depend only on its HTML and the validator, not on other
/// pages or on the config, so a content hash is an exact key. Stored are the
/// raw html-conform findings, before `max_per_page` and deduplication, so
/// config changes need no invalidation. The binary version is part of the key:
/// a new html-conform only ships with a new release.
#[derive(Serialize, Deserialize, Default)]
struct Cache {
    version: String,
    entries: HashMap<String, Vec<html_conform::Finding>>,
}

fn cache_version() -> String {
    format!("{}/1", env!("CARGO_PKG_VERSION"))
}

/// An unreadable, corrupt or outdated cache is an empty one — never an error.
fn load_cache(path: &Path) -> HashMap<String, Vec<html_conform::Finding>> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Cache>(&bytes).ok())
        .filter(|cache| cache.version == cache_version())
        .map(|cache| cache.entries)
        .unwrap_or_default()
}

/// Writes atomically (temp file + rename), so an aborted build cannot leave a
/// half-written cache behind.
fn store_cache(
    path: &Path,
    entries: HashMap<String, Vec<html_conform::Finding>>,
) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let cache = Cache {
        version: cache_version(),
        entries,
    };
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec(&cache)?)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

fn content_hash(html: &str) -> String {
    format!("{:x}", Sha256::digest(html.as_bytes()))
}

/// Native HTML5 conformance validation via `html-conform` (vnu-comparable,
/// pure Rust, no JVM/subprocess/network). Covers tree-construction errors,
/// RELAX NG content-model schema, ARIA co-constraints, attribute
/// microsyntaxes (srcset, datetime, CSP, lang, ...), import-map/speculation
/// JSON, CSP `meta` enforcement, and table cell-grid integrity.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.html_validation.enabled {
        return Vec::new();
    }

    let max_per_page = config.html_validation.max_per_page.unwrap_or(20);
    let cache_path = config
        .html_validation
        .cache_path
        .as_deref()
        .filter(|_| config.html_validation.cache)
        .map(Path::new);
    let cached = cache_path.map(load_cache).unwrap_or_default();

    // Per page: content hash, raw result, and whether it came from the cache.
    let results: Vec<_> = index
        .pages
        .par_iter()
        .map(|page| {
            let validation_html = html_for_validation(&page.html_content);
            let hash = content_hash(&validation_html);
            if let Some(raw) = cached.get(&hash) {
                return (hash, Ok(raw.clone()), true);
            }
            let raw = html_conform::check(&validation_html)
                .map(|report| report.findings)
                .map_err(|error| error.to_string());
            (hash, raw, false)
        })
        .collect();

    if let Some(path) = cache_path {
        // Only the current pages' entries: the file never outgrows the site.
        let entries: HashMap<_, _> = results
            .iter()
            .filter_map(|(hash, raw, _)| Some((hash.clone(), raw.as_ref().ok()?.clone())))
            .collect();
        let write = store_cache(path, entries);
        if config.debug {
            let hits = results.iter().filter(|(_, _, hit)| *hit).count();
            eprintln!(
                "[debug] html_validation: {} validated, {hits} cached",
                results.len() - hits
            );
            if let Err(error) = write {
                eprintln!("[debug] html_validation: cache not written: {error}");
            }
        }
    }

    index
        .pages
        .iter()
        .zip(results)
        .flat_map(|(page, (_, raw, _))| {
            let raw = match raw {
                Ok(raw) => raw,
                Err(error) => {
                    return vec![Finding::review("html/validator-error", format!("HTML conformance validation failed: {error}"))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()))
.with_help("The HTML validator could not initialize. Reinstall or update astro-post-audit before trusting this audit result.")];
                }
            };
            if raw.is_empty() {
                return Vec::new();
            }

            // Deduplicate identical (rule, message) pairs while preserving
            // first-seen order.
            let mut order: Vec<(
                String,
                String,
                Severity,
                Option<html_conform::SourceLocation>,
            )> = Vec::new();
            let mut counts: HashMap<(String, String), usize> = HashMap::new();
            for finding in &raw {
                let key = (finding.rule_id.clone(), finding.message.clone());
                if !counts.contains_key(&key) {
                    order.push((
                        finding.rule_id.clone(),
                        finding.message.clone(),
                        match finding.severity {
                            ConformSeverity::Error => Severity::High,
                            ConformSeverity::Warning => Severity::Medium,
                            ConformSeverity::Info => Severity::Low,
                        },
                        finding.location,
                    ));
                }
                *counts.entry(key).or_insert(0) += 1;
            }

            order
                .into_iter()
                .take(max_per_page)
                .map(|(rule_id, message, schwere, location)| {
                    let count = counts
                        .get(&(rule_id.clone(), message.clone()))
                        .copied()
                        .unwrap_or(1);
                    let occurrences = if count > 1 {
                        format!(" ({count} occurrences)")
                    } else {
                        String::new()
                    };
                    let location = location
                        .map(|location| format!(" at line {location}"))
                        .unwrap_or_default();
                    Finding::review(
                        format!("html/{rule_id}"),
                        format!("HTML conformance{location}: {message}{occurrences}"),
                    )
                    .with_severity(schwere)
                    .at(Location::file(page.rel_path.clone()))
                    .with_help("Fix the markup issue reported by conformance validation (tree construction, content-model schema, ARIA constraints, or attribute microsyntax). Browsers often recover silently, but it can break hydration, accessibility, or interoperability.")
                })
                .collect()
        })
        .collect()
}
