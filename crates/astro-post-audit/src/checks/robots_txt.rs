use url::Url;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};

/// AI citation bots — blocking them reduces AI search visibility.
const AI_CITATION_BOTS: &[&str] = &[
    "ChatGPT-User",
    "GPTBot",
    "ClaudeBot",
    "anthropic-ai",
    "PerplexityBot",
    "Bingbot",
];

/// AI training bots — many publishers deliberately block these.
const AI_TRAINING_BOTS: &[&str] = &["CCBot", "Common Crawl", "CommonCrawl"];

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();

    let robots_path = index.dist_path.join("robots.txt");

    if !robots_path.exists() {
        if config.robots_txt.require {
            findings.push(
                Finding::fail(
                    "robots-txt/missing",
                    "robots.txt not found in dist directory",
                )
                .with_severity(Severity::Medium)
                .at(Location::file("robots.txt"))
                .with_help("Add a robots.txt file to your public/ directory"),
            );
        }
        return findings;
    }

    let content = match std::fs::read_to_string(&robots_path) {
        Ok(c) => c,
        Err(_) => return findings,
    };

    // Sitemap directive
    if config.robots_txt.require_sitemap_link {
        let has_sitemap = content
            .lines()
            .any(|line| line.trim().to_lowercase().starts_with("sitemap:"));

        if !has_sitemap {
            findings.push(
                Finding::fail(
                    "robots-txt/no-sitemap",
                    "robots.txt does not contain a Sitemap directive",
                )
                .with_severity(Severity::Medium)
                .at(Location::file("robots.txt"))
                .with_help("Add 'Sitemap: https://example.com/sitemap.xml' to robots.txt"),
            );
        }
    }

    // Parse robots.txt into agent → directive pairs
    let blocks = parse_robots_blocks(&content);

    // Check for global Disallow: / (all crawlers blocked)
    if config.robots_txt.check_disallow_all {
        for block in &blocks {
            let is_global = block.agents.iter().any(|a| a == "*");
            let has_disallow_all = block.disallows.iter().any(|d| d == "/");
            // Only flag if there's no Allow: / or Allow entries that override
            let has_allow_all = block.allows.iter().any(|a| a == "/");

            if is_global && has_disallow_all && !has_allow_all {
                findings.push(
                    Finding::fail(
                        "robots-txt/disallow-all",
                        "robots.txt blocks all crawlers with 'Disallow: /'",
                    )
                    .with_severity(Severity::High)
                    .at(Location::file("robots.txt"))
                    .with_help(
                        "Remove 'Disallow: /' for User-agent: * to allow search engine indexing",
                    ),
                );
                break;
            }
        }

        // Also check Googlebot/Bingbot specific blocks
        for bot in &["Googlebot", "Bingbot"] {
            for block in &blocks {
                let is_bot = block.agents.iter().any(|a| a.eq_ignore_ascii_case(bot));
                let has_disallow_all = block.disallows.iter().any(|d| d == "/");
                let has_allow_all = block.allows.iter().any(|a| a == "/");

                if is_bot && has_disallow_all && !has_allow_all {
                    findings.push(
                        Finding::fail(
                            "robots-txt/disallow-search-bot",
                            format!("robots.txt blocks {} with 'Disallow: /'", bot),
                        )
                        .with_severity(Severity::High)
                        .at(Location::file("robots.txt"))
                        .with_help(format!(
                            "Remove 'Disallow: /' for {} to allow search engine indexing",
                            bot
                        )),
                    );
                }
            }
        }
    }

    // Crawl-delay check
    if config.robots_txt.max_crawl_delay > 0 {
        let max = config.robots_txt.max_crawl_delay;
        for line in content.lines() {
            let trimmed = line.trim().to_lowercase();
            if let Some(rest) = trimmed.strip_prefix("crawl-delay:") {
                let val_str = rest.trim();
                if let Ok(delay) = val_str.parse::<f64>() {
                    if delay > max as f64 {
                        findings.push(Finding::fail("robots-txt/crawl-delay-high", format!(
                                "Crawl-delay of {} seconds is very high (max recommended: {})",
                                delay, max
                            ))
.with_severity(Severity::Medium)
.at(Location::file("robots.txt"))
.with_help("High crawl delays reduce how often search engines index your content. Use a value ≤ 10."));
                    }
                }
            }
        }
    }

    // AI bot policy
    if config.robots_txt.ai_bot_policy {
        for block in &blocks {
            for agent in &block.agents {
                let is_citation_bot = AI_CITATION_BOTS
                    .iter()
                    .any(|b| agent.eq_ignore_ascii_case(b));
                let has_disallow = block.disallows.iter().any(|d| d == "/");
                let has_allow = block.allows.iter().any(|a| a == "/");

                if is_citation_bot && has_disallow && !has_allow {
                    findings.push(Finding::fail("robots-txt/ai-citation-bot-blocked", format!(
                            "AI citation bot '{}' is blocked — reduces AI search visibility",
                            agent
                        ))
.with_severity(Severity::Medium)
.at(Location::file("robots.txt"))
.with_help(format!(
                            "Remove 'Disallow: /' for {} to allow AI-powered search engines to cite your content",
                            agent
                        )));
                }

                let is_training_bot = AI_TRAINING_BOTS
                    .iter()
                    .any(|b| agent.eq_ignore_ascii_case(b));
                if is_training_bot && !has_disallow {
                    findings.push(Finding::fail("robots-txt/ai-training-bot-allowed", format!(
                            "AI training bot '{}' is allowed — consider blocking if you don't want your content used for training",
                            agent
                        ))
.with_severity(Severity::Low)
.at(Location::file("robots.txt"))
.with_help(format!(
                            "Add 'User-agent: {}\nDisallow: /' to block AI training crawlers",
                            agent
                        )));
                }
            }
        }
    }

    // Contradiction checks need the rules that apply to the wildcard (*) user-agent group.
    if config.robots_txt.check_noindex_contradiction || config.robots_txt.check_sitemap_blocked {
        let (disallows, allows) = wildcard_rules(&blocks);

        // noindex page that is also Disallow'd: crawlers can't see the noindex tag.
        if config.robots_txt.check_noindex_contradiction {
            for page in &index.pages {
                if page.noindex && path_is_disallowed(&disallows, &allows, &page.route) {
                    findings.push(Finding::fail("robots/blocked-noindex-contradiction", format!(
                            "Page '{}' is Disallow'd in robots.txt but also has noindex",
                            page.route
                        ))
.with_severity(Severity::High)
.at(Location::file(page.rel_path.clone()).with_selector("meta[name='robots']"))
.with_help("Crawlers blocked by robots.txt cannot read the noindex tag, so the page may stay indexed. Allow crawling, or drop the noindex and remove internal links instead."));
                }
            }
        }

        // Sitemap URLs that robots.txt blocks send mixed signals to search engines.
        if config.robots_txt.check_sitemap_blocked {
            for url in &index.sitemap_urls {
                let path = Url::parse(url)
                    .ok()
                    .map(|u| u.path().to_string())
                    .unwrap_or_else(|| url.clone());
                if path_is_disallowed(&disallows, &allows, &path) {
                    findings.push(Finding::fail("sitemap/entry-blocked-by-robots", format!("Sitemap URL '{}' is blocked by robots.txt", url))
.with_severity(Severity::Medium)
.at(Location::file("sitemap.xml"))
.with_help("A sitemap should only list crawlable URLs. Remove the entry or allow it in robots.txt."));
                }
            }
        }
    }

    findings
}

/// Collect Disallow/Allow rules that apply to the wildcard (`*`) user-agent group.
fn wildcard_rules(blocks: &[RobotsBlock]) -> (Vec<String>, Vec<String>) {
    let mut disallows = Vec::new();
    let mut allows = Vec::new();
    for block in blocks {
        if block.agents.iter().any(|a| a == "*") {
            disallows.extend(block.disallows.iter().cloned());
            allows.extend(block.allows.iter().cloned());
        }
    }
    (disallows, allows)
}

/// Determine whether `path` is disallowed, applying RFC 9309 longest-match
/// precedence (the more specific rule wins; on equal specificity, Allow wins).
fn path_is_disallowed(disallows: &[String], allows: &[String], path: &str) -> bool {
    let best_disallow = disallows
        .iter()
        .filter_map(|r| rule_match_len(r, path))
        .max();
    let best_allow = allows.iter().filter_map(|r| rule_match_len(r, path)).max();
    match (best_disallow, best_allow) {
        (Some(d), Some(a)) => d > a,
        (Some(_), None) => true,
        _ => false,
    }
}

/// If `pattern` matches `path`, return its specificity (count of literal,
/// non-wildcard characters); otherwise None. Supports `*` wildcards and the
/// `$` end-anchor. An empty pattern (`Disallow:`) never matches (allow-all).
fn rule_match_len(pattern: &str, path: &str) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }
    let anchored = pattern.ends_with('$');
    let pat = if anchored {
        &pattern[..pattern.len() - 1]
    } else {
        pattern
    };

    let parts: Vec<&str> = pat.split('*').collect();
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !path[pos..].starts_with(part) {
                return None;
            }
            pos += part.len();
        } else {
            let idx = path[pos..].find(part)?;
            pos += idx + part.len();
        }
    }

    // With `$`, the match must consume the whole path (unless the pattern ended
    // with a `*`, in which case any suffix is allowed).
    if anchored && !pat.ends_with('*') && pos != path.len() {
        return None;
    }

    Some(pat.chars().filter(|&c| c != '*').count())
}

struct RobotsBlock {
    agents: Vec<String>,
    disallows: Vec<String>,
    allows: Vec<String>,
}

fn parse_robots_blocks(content: &str) -> Vec<RobotsBlock> {
    let mut blocks: Vec<RobotsBlock> = Vec::new();
    let mut current_agents: Vec<String> = Vec::new();
    let mut current_disallows: Vec<String> = Vec::new();
    let mut current_allows: Vec<String> = Vec::new();
    let mut in_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if in_block {
                blocks.push(RobotsBlock {
                    agents: std::mem::take(&mut current_agents),
                    disallows: std::mem::take(&mut current_disallows),
                    allows: std::mem::take(&mut current_allows),
                });
                in_block = false;
            }
            continue;
        }

        let lower = trimmed.to_lowercase();
        if let Some(rest) = lower.strip_prefix("user-agent:") {
            let agent_val = trimmed[trimmed.to_lowercase().find(':').unwrap() + 1..]
                .trim()
                .to_string();
            if in_block && !current_disallows.is_empty() {
                blocks.push(RobotsBlock {
                    agents: std::mem::take(&mut current_agents),
                    disallows: std::mem::take(&mut current_disallows),
                    allows: std::mem::take(&mut current_allows),
                });
            }
            let _ = rest;
            current_agents.push(agent_val);
            in_block = true;
        } else if let Some(rest) = lower.strip_prefix("disallow:") {
            let _ = rest;
            let val = trimmed[trimmed.to_lowercase().find(':').unwrap() + 1..]
                .trim()
                .to_string();
            current_disallows.push(val);
        } else if let Some(rest) = lower.strip_prefix("allow:") {
            let _ = rest;
            let val = trimmed[trimmed.to_lowercase().find(':').unwrap() + 1..]
                .trim()
                .to_string();
            current_allows.push(val);
        }
    }

    if in_block {
        blocks.push(RobotsBlock {
            agents: current_agents,
            disallows: current_disallows,
            allows: current_allows,
        });
    }

    blocks
}
