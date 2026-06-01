use std::collections::{BTreeSet, HashMap, HashSet};

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Confidence, Finding, Level};

/// Analyze Astro's static meta-refresh redirects (generated from `redirects` in
/// `astro.config.mjs`): links pointing at redirect pages, redirect chains, and loops.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.redirects.enabled {
        return Vec::new();
    }

    let norm = &config.url_normalization;
    let base = index.base_url.as_deref();

    // route -> normalized internal redirect target (only internal targets tracked).
    let mut redirect_map: HashMap<String, String> = HashMap::new();
    // route -> rel_path of the redirect page (for reporting).
    let mut file_of: HashMap<String, String> = HashMap::new();

    for page in &index.pages {
        let Some(target) = &page.meta_refresh_target else {
            continue;
        };
        file_of.insert(page.route.clone(), page.rel_path.clone());
        if !normalize::is_internal(target, base) {
            continue;
        }
        if let Some(resolved) = normalize::resolve_href(target, &page.route, base) {
            let target_route = normalize::normalize_path(&resolved, norm);
            if target_route != page.route {
                redirect_map.insert(page.route.clone(), target_route);
            }
        }
    }

    let mut findings = Vec::new();
    let redirect_routes: HashSet<&String> = redirect_map.keys().collect();

    // 1. Internal links that point at a redirect page instead of the final target.
    let mut reported_links: HashSet<(String, String)> = HashSet::new();
    for page in &index.pages {
        for href in &page.anchor_hrefs {
            if !normalize::is_internal(href, base) {
                continue;
            }
            let Some(resolved) = normalize::resolve_href(href, &page.route, base) else {
                continue;
            };
            let route = normalize::normalize_path(&resolved, norm);
            if route == page.route || !redirect_routes.contains(&route) {
                continue;
            }
            if !reported_links.insert((page.rel_path.clone(), route.clone())) {
                continue;
            }
            let final_target = resolve_final(&route, &redirect_map);
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "links/redirect-target".into(),
                file: page.rel_path.clone(),
                selector: format!("a[href='{}']", href),
                message: format!(
                    "Internal link points to redirect page '{}' (final target: '{}')",
                    route, final_target
                ),
                help: "Link directly to the final URL to avoid an unnecessary redirect hop.".into(),
                suggestion: None,
                source_hint: None,
                confidence: Some(Confidence::Medium),
            });
        }
    }

    // 2 + 3. Redirect chains (length > 1) and loops.
    let mut reported_loops: HashSet<BTreeSet<String>> = HashSet::new();
    // Heads = redirect routes that are not themselves a target of another redirect.
    let targets: HashSet<&String> = redirect_map.values().collect();

    for start in redirect_map.keys() {
        let mut path: Vec<String> = vec![start.clone()];
        let mut current = start.clone();

        while let Some(next) = redirect_map.get(&current).cloned() {
            if let Some(cycle_start) = path.iter().position(|r| *r == next) {
                // Loop detected.
                let cycle: BTreeSet<String> = path[cycle_start..].iter().cloned().collect();
                if reported_loops.insert(cycle) {
                    let file = file_of.get(start).cloned().unwrap_or_else(|| start.clone());
                    let mut display = path[cycle_start..].to_vec();
                    display.push(next.clone());
                    findings.push(Finding {
                        level: Level::Error,
                        rule_id: "redirects/loop".into(),
                        file,
                        selector: "meta[http-equiv='refresh']".into(),
                        message: format!("Redirect loop detected: {}", display.join(" -> ")),
                        help: "Break the cycle — a redirect loop makes the page unreachable."
                            .into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
                break;
            }
            path.push(next.clone());
            current = next;
        }

        // Emit a chain finding only from the head of a chain to avoid duplicates.
        let is_head = !targets.contains(start);
        if is_head && path.len() > 2 {
            let file = file_of.get(start).cloned().unwrap_or_else(|| start.clone());
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "redirects/chain".into(),
                file,
                selector: "meta[http-equiv='refresh']".into(),
                message: format!(
                    "Redirect chain of length {}: {}",
                    path.len() - 1,
                    path.join(" -> ")
                ),
                help: "Point the first redirect straight at the final destination to remove intermediate hops.".into(),
                suggestion: None,
                source_hint: None,
                confidence: Some(Confidence::Medium),
            });
        }
    }

    findings
}

/// Follow the redirect chain from `route` to its final destination, stopping on
/// a non-redirect target or a cycle.
fn resolve_final(route: &str, redirect_map: &HashMap<String, String>) -> String {
    let mut current = route.to_string();
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(current.clone());
    while let Some(next) = redirect_map.get(&current) {
        if !visited.insert(next.clone()) {
            break;
        }
        current = next.clone();
    }
    current
}
