use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

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
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "robots-txt/missing".into(),
                file: "robots.txt".into(),
                selector: String::new(),
                message: "robots.txt not found in dist directory".into(),
                help: "Add a robots.txt file to your public/ directory".into(),
                suggestion: None,
                source_hint: None,
                confidence: None,
            });
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
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "robots-txt/no-sitemap".into(),
                file: "robots.txt".into(),
                selector: String::new(),
                message: "robots.txt does not contain a Sitemap directive".into(),
                help: "Add 'Sitemap: https://example.com/sitemap.xml' to robots.txt".into(),
                suggestion: None,
                source_hint: None,
                confidence: None,
            });
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
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "robots-txt/disallow-all".into(),
                    file: "robots.txt".into(),
                    selector: String::new(),
                    message: "robots.txt blocks all crawlers with 'Disallow: /'".into(),
                    help: "Remove 'Disallow: /' for User-agent: * to allow search engine indexing"
                        .into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
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
                    findings.push(Finding {
                        level: Level::Error,
                        rule_id: "robots-txt/disallow-search-bot".into(),
                        file: "robots.txt".into(),
                        selector: String::new(),
                        message: format!("robots.txt blocks {} with 'Disallow: /'", bot),
                        help: format!(
                            "Remove 'Disallow: /' for {} to allow search engine indexing",
                            bot
                        ),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
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
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "robots-txt/crawl-delay-high".into(),
                            file: "robots.txt".into(),
                            selector: String::new(),
                            message: format!(
                                "Crawl-delay of {} seconds is very high (max recommended: {})",
                                delay, max
                            ),
                            help: "High crawl delays reduce how often search engines index your content. Use a value ≤ 10.".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
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
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "robots-txt/ai-citation-bot-blocked".into(),
                        file: "robots.txt".into(),
                        selector: String::new(),
                        message: format!(
                            "AI citation bot '{}' is blocked — reduces AI search visibility",
                            agent
                        ),
                        help: format!(
                            "Remove 'Disallow: /' for {} to allow AI-powered search engines to cite your content",
                            agent
                        ),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }

                let is_training_bot = AI_TRAINING_BOTS
                    .iter()
                    .any(|b| agent.eq_ignore_ascii_case(b));
                if is_training_bot && !has_disallow {
                    findings.push(Finding {
                        level: Level::Info,
                        rule_id: "robots-txt/ai-training-bot-allowed".into(),
                        file: "robots.txt".into(),
                        selector: String::new(),
                        message: format!(
                            "AI training bot '{}' is allowed — consider blocking if you don't want your content used for training",
                            agent
                        ),
                        help: format!(
                            "Add 'User-agent: {}\nDisallow: /' to block AI training crawlers",
                            agent
                        ),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
            }
        }
    }

    findings
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
