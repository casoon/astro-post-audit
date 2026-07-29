use rayon::prelude::*;
use regex::Regex;
use scraper::{ElementRef, Selector};

use crate::config::{self, Config, SeverityLevel, StyleRule, StyleRuleType};
use crate::discovery::{PageInfo, SiteIndex};
use crate::report::{Confidence, Finding, Level};

struct CompiledRule<'a> {
    rule: &'a StyleRule,
    regex: Option<Regex>,
}

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    let cs = &config.content_style;
    if !cs.enabled {
        return Vec::new();
    }

    let Ok(content_sel) = Selector::parse(&cs.content_selector) else {
        return Vec::new();
    };

    let mut all_rules: Vec<StyleRule> =
        cs.rules.clone().unwrap_or_else(config::default_style_rules);
    all_rules.extend(cs.extra_rules.iter().cloned());

    let compiled: Vec<CompiledRule> = all_rules
        .iter()
        .filter(|r| {
            r.level != SeverityLevel::Off && !cs.disabled_rules.iter().any(|id| id == &r.id)
        })
        .map(|r| {
            let regex = r.pattern.as_deref().and_then(|p| Regex::new(p).ok());
            CompiledRule { rule: r, regex }
        })
        .collect();

    if compiled.is_empty() {
        return Vec::new();
    }

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let text = content_text(page, &content_sel);
            let word_count = text.split_whitespace().count();
            if word_count == 0 {
                return Vec::new();
            }

            let mut findings: Vec<Finding> = compiled
                .iter()
                .filter(|cr| rule_applies_to_page_language(cr.rule, page.html_lang.as_deref()))
                .filter_map(|cr| evaluate_rule(cr, &text, word_count, page, &cs.content_selector))
                .collect();
            if let Some(finding) = language_mismatch_finding(page, &text, cs) {
                findings.push(finding);
            }
            findings
        })
        .collect()
}

fn content_text(page: &PageInfo, content_selector: &Selector) -> String {
    let html = page.parse_html();
    html.select(content_selector)
        // A selector list such as the default `article, main, .prose` may
        // match nested containers. Only extract outermost matches so their
        // descendant text is not counted repeatedly.
        .filter(|el| {
            !el.ancestors()
                .filter_map(ElementRef::wrap)
                .any(|ancestor| content_selector.matches(&ancestor))
        })
        .flat_map(|el| el.text())
        .collect::<Vec<_>>()
        .join(" ")
}

const GERMAN_SIGNAL_WORDS: &[&str] = &[
    "der", "die", "das", "und", "ist", "ein", "eine", "mit", "für", "nicht", "auf", "von", "zu",
    "den", "dem", "des", "im", "in",
];
const ENGLISH_SIGNAL_WORDS: &[&str] = &[
    "the", "and", "is", "a", "an", "this", "that", "with", "for", "not", "of", "to", "in", "on",
    "from", "are", "it", "as",
];

fn language_mismatch_finding(
    page: &PageInfo,
    text: &str,
    content_style: &config::ContentStyleConfig,
) -> Option<Finding> {
    let detection = &content_style.language_detection;
    if !detection.enabled
        || content_style
            .disabled_rules
            .iter()
            .any(|id| id == "language-mismatch")
    {
        return None;
    }
    let expected = primary_language(page.html_lang.as_deref())?;
    let (expected_name, expected_signals, other_name, other_signals) = match expected.as_str() {
        "de" => (
            "German",
            GERMAN_SIGNAL_WORDS,
            "English",
            ENGLISH_SIGNAL_WORDS,
        ),
        "en" => (
            "English",
            ENGLISH_SIGNAL_WORDS,
            "German",
            GERMAN_SIGNAL_WORDS,
        ),
        _ => return None,
    };
    let (expected_count, _) = count_signal_words(text, expected_signals);
    let (other_count, examples) = count_signal_words(text, other_signals);
    if other_count < detection.min_signal_words
        || (other_count as f64) < expected_count as f64 * detection.mismatch_ratio
    {
        return None;
    }

    Some(Finding::new(
        Level::Info,
        "content-style/language-mismatch",
        page.rel_path.clone(),
        "html[lang]",
        format!(
            "html lang declares {expected_name}, but {other_name} signal words dominate ({other_count} vs {expected_count}); examples: {}",
            examples.join(", ")
        ),
        "Confirm the page language manually, then align html lang or exclude mixed-language content from this heuristic.",
        Some(Confidence::Low),
    ))
}

fn count_signal_words(text: &str, signals: &[&str]) -> (usize, Vec<String>) {
    let mut count = 0;
    let mut examples = Vec::new();
    for word in text.split(|c: char| !c.is_alphabetic()) {
        let normalized = word.to_ascii_lowercase();
        if signals.contains(&normalized.as_str()) {
            count += 1;
            if examples.len() < 5 && !examples.contains(&normalized) {
                examples.push(normalized);
            }
        }
    }
    (count, examples)
}

fn rule_applies_to_page_language(rule: &StyleRule, page_language: Option<&str>) -> bool {
    if rule.languages.is_empty() {
        return true;
    }
    let Some(primary_language) = primary_language(page_language) else {
        // Do not silently lose coverage on pages that omit html[lang].
        return true;
    };
    rule.languages
        .iter()
        .any(|rule_language| rule_language.eq_ignore_ascii_case(&primary_language))
}

fn primary_language(page_language: Option<&str>) -> Option<String> {
    page_language.map(|language| {
        language
            .trim()
            .split(['-', '_'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
    })
}

fn level_from_severity(level: &SeverityLevel) -> Level {
    match level {
        SeverityLevel::Error => Level::Error,
        SeverityLevel::Warning => Level::Warning,
        // Off is filtered out before evaluation; default to Info if it ever reaches here.
        SeverityLevel::Info | SeverityLevel::Off => Level::Info,
    }
}

fn evaluate_rule(
    cr: &CompiledRule,
    text: &str,
    word_count: usize,
    page: &PageInfo,
    content_selector: &str,
) -> Option<Finding> {
    let rule = cr.rule;
    match rule.rule_type {
        StyleRuleType::Presence => {
            let regex = cr.regex.as_ref()?;
            let (count, excerpt) = match_count_and_excerpt(regex, text)?;
            let message = rule
                .message
                .clone()
                .unwrap_or_else(|| "Pattern matched {count} time(s)".into())
                .replace("{count}", &count.to_string());
            Some(build_finding(
                rule,
                page,
                content_selector,
                message,
                Some(excerpt),
            ))
        }
        StyleRuleType::DensityPer1000Words => {
            let regex = cr.regex.as_ref()?;
            let threshold = rule.threshold?;
            let (count, excerpt) = match_count_and_excerpt(regex, text)?;
            let density = count as f64 / word_count as f64 * 1000.0;
            if density <= threshold {
                return None;
            }
            let message = rule
                .message
                .clone()
                .unwrap_or_else(|| {
                    "Pattern density is {density} per 1000 words ({count} in {word_count} words) — above the {threshold} threshold".into()
                })
                .replace("{count}", &count.to_string())
                .replace("{word_count}", &word_count.to_string())
                .replace("{density}", &format!("{density:.1}"))
                .replace("{threshold}", &format!("{threshold:.1}"));
            Some(build_finding(
                rule,
                page,
                content_selector,
                message,
                Some(excerpt),
            ))
        }
        StyleRuleType::SentenceLengthUniformity => {
            let threshold = rule.threshold?;
            let sentence_lengths = sentence_word_counts(text);
            if sentence_lengths.len() < rule.min_sentences {
                return None;
            }
            let n = sentence_lengths.len() as f64;
            let mean = sentence_lengths.iter().sum::<f64>() / n;
            if mean <= 0.0 {
                return None;
            }
            let variance = sentence_lengths
                .iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>()
                / n;
            let cv = variance.sqrt() / mean;
            if cv >= threshold {
                return None;
            }
            let message = rule
                .message
                .clone()
                .unwrap_or_else(|| {
                    "Sentence lengths are unusually uniform: coefficient of variation {cv} across {sentences} sentences (below the {threshold} threshold)".into()
                })
                .replace("{cv}", &format!("{cv:.2}"))
                .replace("{sentences}", &sentence_lengths.len().to_string())
                .replace("{threshold}", &format!("{threshold:.2}"));
            Some(build_finding(rule, page, content_selector, message, None))
        }
    }
}

fn build_finding(
    rule: &StyleRule,
    page: &PageInfo,
    content_selector: &str,
    message: String,
    excerpt: Option<String>,
) -> Finding {
    Finding::new(
        level_from_severity(&rule.level),
        format!("content-style/{}", rule.id),
        page.rel_path.clone(),
        content_selector,
        match excerpt {
            Some(excerpt) => format!("{message} — Example: \"{excerpt}\""),
            None => message,
        },
        rule.help.clone().unwrap_or_else(|| {
            "Heuristic content-style finding — confirm manually, then adjust wording or override the rule's threshold/severity in config.".into()
        }),
        Some(Confidence::Low),
    )
}

fn match_count_and_excerpt(regex: &Regex, text: &str) -> Option<(usize, String)> {
    let mut matches = regex.find_iter(text);
    let first_match = matches.next()?;
    Some((
        1 + matches.count(),
        excerpt_around(text, first_match.start(), first_match.end()),
    ))
}

fn excerpt_around(text: &str, start: usize, end: usize) -> String {
    const CONTEXT_CHARS: usize = 72;

    let before: String = text[..start]
        .chars()
        .rev()
        .take(CONTEXT_CHARS)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let after: String = text[end..].chars().take(CONTEXT_CHARS).collect();
    let prefix = if text[..start].chars().count() > CONTEXT_CHARS {
        "…"
    } else {
        ""
    };
    let suffix = if text[end..].chars().count() > CONTEXT_CHARS {
        "…"
    } else {
        ""
    };
    format!("{prefix}{before}{}{after}{suffix}", &text[start..end])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Naive sentence splitter: splits on `.`/`!`/`?`, filters fragments under 3 words
/// (headings, list items). Doesn't special-case abbreviations or decimals — a
/// stylistic heuristic, not a linguistic parser.
fn sentence_word_counts(text: &str) -> Vec<f64> {
    text.split(['.', '!', '?'])
        .map(|s| s.split_whitespace().count())
        .filter(|&words| words >= 3)
        .map(|words| words as f64)
        .collect()
}
