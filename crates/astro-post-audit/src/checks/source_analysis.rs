//! Conservative, opt-in source analysis for Astro and Tailwind projects.
//!
//! This deliberately extracts only quoted, static class values. It neither
//! evaluates JavaScript nor attempts to replicate Tailwind's content scanner.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use globset::{Glob, GlobSetBuilder};
use regex::Regex;
use walkdir::WalkDir;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Confidence, Finding, Level};

#[derive(Debug, Clone)]
struct ClassUse {
    file: String,
    line: usize,
    tokens: Vec<String>,
}

pub fn check_all(_: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.source_analysis.enabled {
        return Vec::new();
    }
    let Some(root) = config.project_root.as_deref() else {
        return Vec::new(); // validated before checks run
    };
    let uses = discover(Path::new(root), config);
    let mut findings = Vec::new();
    if config.source_analysis.tailwind_inventory {
        findings.extend(inventory(&uses));
    }
    if config.source_analysis.duplicate_signatures {
        findings.extend(duplicates(
            &uses,
            config.source_analysis.min_duplicate_occurrences,
        ));
    }
    if config.source_analysis.utility_conflicts {
        findings.extend(conflicts(&uses));
    }
    if config.source_analysis.component_complexity {
        findings.extend(complexity(Path::new(root), config));
    }
    findings
}

fn discover(root: &Path, config: &Config) -> Vec<ClassUse> {
    let mut extensions: BTreeSet<String> = ["astro", "html", "mdx", "jsx", "tsx"]
        .into_iter()
        .map(str::to_string)
        .collect();
    extensions.extend(
        config
            .source_analysis
            .extensions
            .iter()
            .map(|s| s.trim_start_matches('.').to_string()),
    );
    let mut builder = GlobSetBuilder::new();
    for pattern in &config.source_analysis.exclude {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    let excluded = builder.build().ok();
    let class_re = Regex::new(r#"(?:class|className)\s*=\s*[\"']([^\"']+)[\"']"#).unwrap();
    let list_re = Regex::new(r#"[\"']([^\"']+)[\"']"#).unwrap();
    let class_list_re = Regex::new(r"class:list\s*=\s*\{\{([^}]*)\}\}").unwrap();
    let mut uses = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if !path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| extensions.contains(ext))
        {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        if excluded.as_ref().is_some_and(|set| set.is_match(relative)) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let file = relative.to_string_lossy().replace('\\', "/");
        for captures in class_re.captures_iter(&source) {
            let value = captures.get(1).unwrap();
            let tokens = value
                .as_str()
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>();
            if !tokens.is_empty() {
                uses.push(ClassUse {
                    file: file.clone(),
                    line: line_at(&source, value.start()),
                    tokens,
                });
            }
        }
        // Astro's class:list can contain quoted static entries. Dynamic entries
        // are intentionally ignored rather than guessed.
        for m in class_list_re.captures_iter(&source) {
            let value = m.get(1).unwrap();
            for quoted in list_re.captures_iter(value.as_str()) {
                let content = quoted.get(1).unwrap();
                let tokens = content
                    .as_str()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if !tokens.is_empty() {
                    uses.push(ClassUse {
                        file: file.clone(),
                        line: line_at(&source, value.start() + content.start()),
                        tokens,
                    });
                }
            }
        }
    }
    uses
}

fn line_at(source: &str, byte: usize) -> usize {
    source[..byte].bytes().filter(|b| *b == b'\n').count() + 1
}

fn location(item: &ClassUse) -> String {
    format!("{}:{}", item.file, item.line)
}

fn inventory(uses: &[ClassUse]) -> Vec<Finding> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for item in uses {
        for token in &item.tokens {
            *counts.entry(family(token)).or_default() += 1;
        }
    }
    if counts.is_empty() {
        return Vec::new();
    }
    let observed = counts
        .iter()
        .map(|(name, n)| format!("{name} ({n})"))
        .collect::<Vec<_>>()
        .join(", ");
    vec![Finding::new(Level::Info, "source-analysis/tailwind-inventory", "source", "static class attributes", format!("Tailwind inventory across {} static class list(s): {observed}", uses.len()), "Use the counts as evidence for design-system review; no utility is considered incorrect by itself.", Some(Confidence::Low))]
}

fn family(token: &str) -> String {
    let base = token.rsplit(':').next().unwrap_or(token);
    let prefix = base.split('-').next().unwrap_or(base);
    match prefix {
        "p" | "px" | "py" | "pt" | "pr" | "pb" | "pl" | "m" | "mx" | "my" | "mt" | "mr" | "mb"
        | "ml" | "gap" => "spacing".into(),
        "text" | "font" | "leading" | "tracking" => "typography".into(),
        "bg" | "border" | "fill" | "stroke" => "color".into(),
        "rounded" => "radius".into(),
        "shadow" => "shadow".into(),
        "animate" | "transition" | "duration" | "ease" => "motion".into(),
        _ => prefix.into(),
    }
}

fn duplicates(uses: &[ClassUse], minimum: usize) -> Vec<Finding> {
    let mut grouped: BTreeMap<String, Vec<&ClassUse>> = BTreeMap::new();
    for item in uses {
        let mut tokens = item.tokens.clone();
        tokens.sort();
        tokens.dedup();
        grouped.entry(tokens.join(" ")).or_default().push(item);
    }
    grouped.into_iter().filter_map(|(signature, items)| {
        (items.len() >= minimum).then(|| {
            let locations = items.iter().take(3).map(|i| location(i)).collect::<Vec<_>>().join(", ");
            let mut finding = Finding::new(Level::Info, "source-analysis/duplicate-signature", items[0].file.clone(), location(items[0]), format!("Static Tailwind signature occurs {} times: {signature}", items.len()), format!("Review the {} occurrences for intentional markup reuse; examples: {locations}.", items.len()), Some(Confidence::Low));
            finding.suggestion = Some("Consider a shared component only when the surrounding markup has the same purpose.".into());
            finding
        })
    }).collect()
}

fn conflicts(uses: &[ClassUse]) -> Vec<Finding> {
    let display = [
        "block",
        "inline",
        "inline-block",
        "flex",
        "inline-flex",
        "grid",
        "hidden",
    ];
    uses.iter().flat_map(|item| {
        let mut groups: HashMap<String, Vec<&str>> = HashMap::new();
        for token in &item.tokens {
            let (variant, base) = token.rsplit_once(':').unwrap_or(("", token));
            let key = if display.contains(&base) { format!("{variant}:display") } else if base.starts_with("justify-") { format!("{variant}:justify") } else { continue };
            groups.entry(key).or_default().push(token);
        }
        let mut seen = BTreeSet::new();
        item.tokens.iter().filter_map(move |token| (!seen.insert(token)).then_some(format!("duplicate `{token}`"))).chain(groups.into_values().filter(|tokens| tokens.iter().collect::<BTreeSet<_>>().len() > 1).map(|tokens| format!("mutually exclusive `{}`", tokens.join("`, `")))).map(move |detail| Finding::new(Level::Info, "source-analysis/utility-conflict", item.file.clone(), location(item), format!("Static class list contains {detail}"), "Review this same-variant utility list; partial-overlap utilities are intentionally not diagnosed.", Some(Confidence::Low)))
    }).collect()
}

fn complexity(root: &Path, config: &Config) -> Vec<Finding> {
    let astro_files = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "astro"))
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(root)
                .ok()
                .map(|path| path.to_string_lossy().replace('\\', "/"))
        })
        .collect::<BTreeSet<_>>();
    astro_files.into_iter().filter_map(|file| {
        let source = std::fs::read_to_string(root.join(&file)).ok()?;
        let lines = source.lines().count();
        let props = Regex::new(r"(?s)(?:interface|type)\s+Props\s*(?:=)?\s*\{(.*?)\}").unwrap().captures(&source).map_or(0, |c| c[1].lines().filter(|line| line.contains(':')).count());
        let slots = Regex::new(r#"<slot\s+name=[\"'][^\"']+"#).unwrap().find_iter(&source).count();
        if lines <= config.source_analysis.max_component_lines && props <= config.source_analysis.max_component_props && slots <= config.source_analysis.max_component_slots { return None; }
        Some(Finding::new(Level::Info, "source-analysis/component-complexity", file.clone(), file, format!("Astro component has {lines} lines, {props} declared Props member(s), and {slots} named slot(s)"), format!("Advisory thresholds: {} lines, {} props, {} slots. Review structure; no split is prescribed.", config.source_analysis.max_component_lines, config.source_analysis.max_component_props, config.source_analysis.max_component_slots), Some(Confidence::Low)))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_variants_separate_and_reports_safe_conflicts() {
        let uses = vec![ClassUse {
            file: "Card.astro".into(),
            line: 1,
            tokens: vec![
                "flex".into(),
                "grid".into(),
                "md:flex".into(),
                "md:flex".into(),
            ],
        }];
        let findings = conflicts(&uses);
        assert_eq!(findings.len(), 2);
        assert!(findings
            .iter()
            .any(|f| f.message.contains("`flex`, `grid`")));
        assert!(findings
            .iter()
            .any(|f| f.message.contains("duplicate `md:flex`")));
    }

    #[test]
    fn duplicate_signatures_are_deterministic() {
        let uses = (0..3)
            .map(|line| ClassUse {
                file: format!("{line}.astro"),
                line: 1,
                tokens: vec!["items-center".into(), "flex".into()],
            })
            .collect::<Vec<_>>();
        assert_eq!(duplicates(&uses, 3).len(), 1);
    }

    #[test]
    fn discovers_only_static_classes_and_respects_exclusions() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("Card.astro"),
            r#"<div class="flex items-center"></div><div class={dynamic}></div><div class:list={{ ["grid gap-4", dynamic] }}></div>"#,
        )
        .unwrap();
        std::fs::write(
            temp.path().join("Ignored.astro"),
            r#"<div class="hidden"></div>"#,
        )
        .unwrap();
        let mut config = Config::default();
        config.source_analysis.exclude = vec!["Ignored.astro".into()];
        let uses = discover(temp.path(), &config);
        assert_eq!(uses.len(), 2);
        assert!(uses.iter().all(|use_| use_.file == "Card.astro"));
        assert_eq!(uses[0].tokens, ["flex", "items-center"]);
        assert_eq!(uses[1].tokens, ["grid", "gap-4"]);
    }

    #[test]
    fn measures_components_even_without_static_classes() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("Large.astro"),
            "---\ninterface Props {\n  title: string;\n}\n---\n<slot name=\"header\" />\n",
        )
        .unwrap();
        let mut config = Config::default();
        config.source_analysis.max_component_lines = 1;
        assert_eq!(complexity(temp.path(), &config).len(), 1);
    }
}
