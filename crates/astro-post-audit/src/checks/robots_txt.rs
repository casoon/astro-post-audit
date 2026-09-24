use url::Url;
use web_checks::robots::{self, BotClass, Group};

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};

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

    // Grammatik, Bot-Einordnung und Pfadauswertung kommen aus web-checks:
    // dieselbe Auswertung, die auditmysite an der laufenden Seite benutzt.
    let parsed = robots::parse(&content);
    let blocks = &parsed.groups;

    // Check for global Disallow: / (all crawlers blocked)
    if config.robots_txt.check_disallow_all {
        for block in blocks {
            let is_global = block.bot_class == BotClass::Wildcard;
            let has_disallow_all = block.disallows_all();
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
            for block in blocks {
                let is_bot = block.user_agent.eq_ignore_ascii_case(bot);
                let has_disallow_all = block.disallows_all();
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

    // AI bot policy. Die Einordnung kommt aus web-checks, nicht aus einer
    // eigenen Liste: zwei Listen fuer dieselbe Frage waren schon
    // widerspruechlich — GPTBot stand hier als Citation-Bot, waehrend
    // auditmysite ihn als Trainings-Bot fuehrte.
    if config.robots_txt.ai_bot_policy {
        for block in blocks {
            let agent = &block.user_agent;
            let has_disallow = block.disallows_all();
            let has_allow = block.allows.iter().any(|a| a == "/");

            if block.bot_class == BotClass::AiCitation && has_disallow && !has_allow {
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

            if block.bot_class == BotClass::AiTraining && !has_disallow {
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

    // Contradiction checks need the rules that apply to the wildcard (*) user-agent group.
    if config.robots_txt.check_noindex_contradiction || config.robots_txt.check_sitemap_blocked {
        let (disallows, allows) = wildcard_rules(blocks);

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
///
/// Es kann mehrere `*`-Gruppen geben; ihre Regeln gelten zusammen.
fn wildcard_rules(groups: &[Group]) -> (Vec<String>, Vec<String>) {
    let mut disallows = Vec::new();
    let mut allows = Vec::new();
    for group in groups {
        if group.bot_class == BotClass::Wildcard {
            disallows.extend(group.disallows.iter().cloned());
            allows.extend(group.allows.iter().cloned());
        }
    }
    (disallows, allows)
}

fn path_is_disallowed(disallows: &[String], allows: &[String], path: &str) -> bool {
    robots::path_is_disallowed(disallows, allows, path)
}
