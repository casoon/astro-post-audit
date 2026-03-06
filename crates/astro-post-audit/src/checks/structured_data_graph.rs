use std::collections::{HashMap, HashSet};

use scraper::Selector;
use serde_json::Value;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

#[derive(Debug, Clone)]
struct EntitySnapshot {
    file: String,
    entity_id: String,
    type_name: Option<String>,
    name: Option<String>,
    url: Option<String>,
}

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.structured_data_graph.enabled {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let script_sel = Selector::parse("script[type='application/ld+json']").unwrap();
    let mut snapshots: Vec<EntitySnapshot> = Vec::new();

    for page in &index.pages {
        let html = page.parse_html();
        for script in html.select(&script_sel) {
            let content = script.text().collect::<String>();
            let trimmed = content.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(json) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            collect_entity_snapshots(&json, &page.rel_path, &mut snapshots);
        }
    }

    let mut by_id: HashMap<String, Vec<&EntitySnapshot>> = HashMap::new();
    for snap in &snapshots {
        by_id.entry(snap.entity_id.clone()).or_default().push(snap);
    }

    for (entity_id, refs) in by_id {
        if refs.len() < 2 {
            continue;
        }
        let types: HashSet<String> = refs.iter().filter_map(|r| r.type_name.clone()).collect();
        if types.len() > 1 {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "structured-data-graph/type-conflict".into(),
                file: refs[0].file.clone(),
                selector: "script[type='application/ld+json']".into(),
                message: format!(
                    "Entity '{}' appears with conflicting @type values across pages: {}",
                    entity_id,
                    {
                        let mut sorted: Vec<_> = types.into_iter().collect();
                        sorted.sort_unstable();
                        sorted.join(", ")
                    }
                ),
                help: "Use a consistent @type for the same @id entity across the site".into(),
                suggestion: None,
            });
        }

        let names: HashSet<String> = refs.iter().filter_map(|r| r.name.clone()).collect();
        if names.len() > 1 {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "structured-data-graph/name-conflict".into(),
                file: refs[0].file.clone(),
                selector: "script[type='application/ld+json']".into(),
                message: format!(
                    "Entity '{}' has inconsistent 'name' values across pages",
                    entity_id
                ),
                help: "Keep core entity fields (name/url/type) consistent across pages".into(),
                suggestion: None,
            });
        }

        let urls: HashSet<String> = refs.iter().filter_map(|r| r.url.clone()).collect();
        if urls.len() > 1 {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "structured-data-graph/url-conflict".into(),
                file: refs[0].file.clone(),
                selector: "script[type='application/ld+json']".into(),
                message: format!(
                    "Entity '{}' has conflicting 'url' values across pages",
                    entity_id
                ),
                help: "Use one canonical URL value for the same entity across pages".into(),
                suggestion: None,
            });
        }
    }

    for snap in snapshots {
        let Some(entity_url) = &snap.url else {
            continue;
        };
        if !normalize::is_internal(entity_url, index.base_url.as_deref()) {
            continue;
        }
        if let Some(route) = normalize::resolve_href(entity_url, "/", index.base_url.as_deref()) {
            let norm = normalize::normalize_path(&route, &config.url_normalization);
            if !index.route_exists(&norm) {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "structured-data-graph/internal-url-missing".into(),
                    file: snap.file.clone(),
                    selector: "script[type='application/ld+json']".into(),
                    message: format!(
                        "Structured data entity '{}' references internal URL '{}' that is missing in dist",
                        snap.entity_id, entity_url
                    ),
                    help: "Point structured-data URLs to existing canonical pages".into(),
                    suggestion: None,
                });
            }
        }
    }

    findings
}

fn collect_entity_snapshots(value: &Value, file: &str, out: &mut Vec<EntitySnapshot>) {
    if let Some(graph) = value.get("@graph").and_then(|g| g.as_array()) {
        for item in graph {
            collect_entity_snapshots(item, file, out);
        }
    }
    if let Some(arr) = value.as_array() {
        for item in arr {
            collect_entity_snapshots(item, file, out);
        }
    }
    let Some(obj) = value.as_object() else {
        return;
    };
    if obj.is_empty() {
        return;
    }
    let entity_id = obj
        .get("@id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            obj.get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });
    let Some(entity_id) = entity_id else {
        return;
    };

    let type_name = obj
        .get("@type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            obj.get("@type")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let url = obj
        .get("url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    out.push(EntitySnapshot {
        file: file.to_string(),
        entity_id,
        type_name,
        name,
        url,
    });
}
