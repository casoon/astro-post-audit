use std::collections::HashMap;

use rayon::prelude::*;
use scraper::Selector;
use serde_json::Value;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.structured_data.check_json_ld
        && !config.structured_data.require_json_ld
        && !config.structured_data.detect_duplicate_types
    {
        return Vec::new();
    }

    let ld_sel = Selector::parse("script[type='application/ld+json']").unwrap();

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            let scripts: Vec<_> = html.select(&ld_sel).collect();

            if scripts.is_empty() {
                if config.structured_data.require_json_ld {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "structured-data/missing".into(),
                        file: page.rel_path.clone(),
                        selector: "head".into(),
                        message: "No JSON-LD structured data found".into(),
                        help: "Add <script type=\"application/ld+json\"> with schema.org data"
                            .into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
                return findings;
            }

            // Parse all JSON-LD blocks
            let mut parsed_blocks: Vec<(String, Value)> = Vec::new();
            for (i, script) in scripts.iter().enumerate() {
                let content: String = script.text().collect();
                let trimmed = content.trim();
                let selector_hint = format!("script[type='application/ld+json']:nth({})", i + 1);

                if trimmed.is_empty() {
                    if config.structured_data.check_json_ld {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "structured-data/empty".into(),
                            file: page.rel_path.clone(),
                            selector: selector_hint,
                            message: "JSON-LD script is empty".into(),
                            help: "Add valid JSON-LD content or remove the empty script tag".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                    }
                    continue;
                }

                match serde_json::from_str::<Value>(trimmed) {
                    Err(e) => {
                        if config.structured_data.check_json_ld {
                            findings.push(Finding {
                                level: Level::Error,
                                rule_id: "structured-data/invalid-json".into(),
                                file: page.rel_path.clone(),
                                selector: selector_hint.clone(),
                                message: format!("Invalid JSON in JSON-LD: {}", e),
                                help: "Fix the JSON syntax in the structured data block".into(),
                                suggestion: None,
                                source_hint: None,
                                confidence: None,
                            });
                        }
                    }
                    Ok(json) => {
                        if config.structured_data.check_json_ld {
                            check_semantics(&json, &page.rel_path, &selector_hint, &mut findings);
                        }
                        parsed_blocks.push((selector_hint, json));
                    }
                }
            }

            // Detect duplicate @type across JSON-LD blocks on the same page
            if config.structured_data.detect_duplicate_types && parsed_blocks.len() > 1 {
                let mut type_counts: HashMap<String, Vec<String>> = HashMap::new();
                for (selector, json) in &parsed_blocks {
                    for t in extract_types(json) {
                        type_counts.entry(t).or_default().push(selector.clone());
                    }
                }
                for (type_name, selectors) in &type_counts {
                    if selectors.len() > 1 {
                        findings.push(Finding {
                            level: Level::Warning,
                            rule_id: "structured-data/duplicate-type".into(),
                            file: page.rel_path.clone(),
                            selector: selectors.join(", "),
                            message: format!(
                                "Duplicate JSON-LD @type '{}' found {} times on this page",
                                type_name,
                                selectors.len()
                            ),
                            help: format!(
                                "Consolidate {} blocks into a single JSON-LD script or use @graph",
                                type_name
                            ),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }
            }

            findings
        })
        .collect()
}

/// Extract all @type values from a JSON-LD object (including @graph items).
fn extract_types(json: &Value) -> Vec<String> {
    let mut types = Vec::new();
    if let Some(graph) = json.get("@graph").and_then(|g| g.as_array()) {
        for item in graph {
            if let Some(t) = get_type_name(item) {
                types.push(t);
            }
        }
    } else if let Some(t) = get_type_name(json) {
        types.push(t);
    }
    types
}

fn get_type_name(entity: &Value) -> Option<String> {
    entity.get("@type").and_then(|t| {
        t.as_str().map(|s| s.to_string()).or_else(|| {
            t.as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    })
}

/// Check semantic validity of a JSON-LD object.
fn check_semantics(json: &Value, file: &str, selector: &str, findings: &mut Vec<Finding>) {
    // Handle @graph arrays
    if let Some(graph) = json.get("@graph").and_then(|g| g.as_array()) {
        for item in graph {
            check_single_entity(item, file, selector, findings);
        }
    } else {
        check_single_entity(json, file, selector, findings);
    }

    // Check @context is present and plausible at root level
    if json.get("@context").is_none() && json.get("@graph").is_none() {
        findings.push(Finding {
            level: Level::Warning,
            rule_id: "structured-data/missing-context".into(),
            file: file.to_string(),
            selector: selector.to_string(),
            message: "JSON-LD missing @context property".into(),
            help: "Add \"@context\": \"https://schema.org\" to the JSON-LD object".into(),
            suggestion: None,
            source_hint: None,
            confidence: None,
        });
    } else if let Some(ctx) = json.get("@context").and_then(|c| c.as_str()) {
        if !ctx.contains("schema.org") {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "structured-data/unusual-context".into(),
                file: file.to_string(),
                selector: selector.to_string(),
                message: format!("JSON-LD @context '{}' is not schema.org", ctx),
                help: "Use \"https://schema.org\" as the @context".into(),
                suggestion: None,
                source_hint: None,
                confidence: None,
            });
        }
    }
}

/// Check a single JSON-LD entity for required properties based on @type.
fn check_single_entity(entity: &Value, file: &str, selector: &str, findings: &mut Vec<Finding>) {
    // @type must be present
    let type_val = match entity.get("@type") {
        Some(t) => t,
        None => {
            if entity.is_object() && !entity.as_object().unwrap().is_empty() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "structured-data/missing-type".into(),
                    file: file.to_string(),
                    selector: selector.to_string(),
                    message: "JSON-LD entity missing @type property".into(),
                    help: "Add an @type property (e.g. \"Article\", \"WebPage\")".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
            return;
        }
    };

    let type_name = match type_val.as_str() {
        Some(s) => s.to_string(),
        None => {
            if let Some(arr) = type_val.as_array() {
                arr.first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                return;
            }
        }
    };

    // Type-specific required fields (conservative to minimize false positives)
    let required_fields: &[&str] = match type_name.as_str() {
        "Article" | "NewsArticle" | "BlogPosting" => &["headline"],
        "BreadcrumbList" => &["itemListElement"],
        "Organization" | "LocalBusiness" => &["name"],
        "Person" => &["name"],
        "WebSite" => &["name", "url"],
        "Product" => &["name"],
        "FAQPage" => &["mainEntity"],
        "WebPage" => &[],
        _ => &[],
    };

    for field in required_fields {
        if entity.get(field).is_none() {
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "structured-data/missing-property".into(),
                file: file.to_string(),
                selector: selector.to_string(),
                message: format!(
                    "JSON-LD {} is missing required property '{}'",
                    type_name, field
                ),
                help: format!("Add the '{}' property to the {} schema", field, type_name),
                suggestion: None,
                source_hint: None,
                confidence: None,
            });
        }
    }

    // Extended property completeness checks
    check_property_completeness(&type_name, entity, file, selector, findings);
}

/// Extended checks for recommended properties per schema type.
fn check_property_completeness(
    type_name: &str,
    entity: &Value,
    file: &str,
    selector: &str,
    findings: &mut Vec<Finding>,
) {
    match type_name {
        "Article" | "BlogPosting" | "NewsArticle" => {
            // Recommended: author
            if entity.get("author").is_none() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "structured-data/article-missing-author".into(),
                    file: file.to_string(),
                    selector: selector.to_string(),
                    message: format!("JSON-LD {} is missing recommended property 'author'", type_name),
                    help: "Add \"author\": {\"@type\": \"Person\", \"name\": \"...\"} for rich results".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
            // Recommended: datePublished
            if entity.get("datePublished").is_none() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "structured-data/article-missing-date-published".into(),
                    file: file.to_string(),
                    selector: selector.to_string(),
                    message: format!("JSON-LD {} is missing recommended property 'datePublished'", type_name),
                    help: "Add \"datePublished\": \"YYYY-MM-DD\" (ISO 8601) for search-engine rich results".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
            // Info: dateModified
            if entity.get("dateModified").is_none() {
                findings.push(Finding {
                    level: Level::Info,
                    rule_id: "structured-data/article-missing-date-modified".into(),
                    file: file.to_string(),
                    selector: selector.to_string(),
                    message: format!("JSON-LD {} has no 'dateModified' property", type_name),
                    help: "Add \"dateModified\" to help search engines understand freshness".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
            // Recommended: image
            if entity.get("image").is_none() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "structured-data/article-missing-image".into(),
                    file: file.to_string(),
                    selector: selector.to_string(),
                    message: format!(
                        "JSON-LD {} is missing recommended property 'image'",
                        type_name
                    ),
                    help: "Add an \"image\" property with an absolute URL for rich results".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
            // NewsArticle requires publisher with logo
            if type_name == "NewsArticle" && entity.get("publisher").is_none() {
                findings.push(Finding {
                    level: Level::Error,
                    rule_id: "structured-data/news-article-missing-publisher".into(),
                    file: file.to_string(),
                    selector: selector.to_string(),
                    message: "JSON-LD NewsArticle is missing required 'publisher' property".into(),
                    help: "Add \"publisher\": {\"@type\": \"Organization\", \"name\": \"...\", \"logo\": {\"@type\": \"ImageObject\", \"url\": \"...\"}}".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
        }
        "Organization" | "LocalBusiness" => {
            // Recommended: url
            if entity.get("url").is_none() {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "structured-data/organization-missing-url".into(),
                    file: file.to_string(),
                    selector: selector.to_string(),
                    message: format!(
                        "JSON-LD {} is missing recommended property 'url'",
                        type_name
                    ),
                    help: "Add a \"url\" property with the organization's website URL".into(),
                    suggestion: None,
                    source_hint: None,
                    confidence: None,
                });
            }
            // Info for LocalBusiness: telephone, address
            if type_name == "LocalBusiness" {
                if entity.get("telephone").is_none() {
                    findings.push(Finding {
                        level: Level::Info,
                        rule_id: "structured-data/local-business-missing-telephone".into(),
                        file: file.to_string(),
                        selector: selector.to_string(),
                        message: "JSON-LD LocalBusiness has no 'telephone' property".into(),
                        help: "Add \"telephone\" for local search visibility".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
                if entity.get("address").is_none() {
                    findings.push(Finding {
                        level: Level::Info,
                        rule_id: "structured-data/local-business-missing-address".into(),
                        file: file.to_string(),
                        selector: selector.to_string(),
                        message: "JSON-LD LocalBusiness has no 'address' property".into(),
                        help: "Add \"address\" with a PostalAddress for local search".into(),
                        suggestion: None,
                        source_hint: None,
                        confidence: None,
                    });
                }
            }
        }
        "FAQPage" => {
            // Verify mainEntity items have acceptedAnswer
            if let Some(main_entity) = entity.get("mainEntity") {
                let items = if let Some(arr) = main_entity.as_array() {
                    arr.iter().collect::<Vec<_>>()
                } else {
                    vec![main_entity]
                };
                for item in &items {
                    if item.get("acceptedAnswer").is_none() {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "structured-data/faq-missing-answer".into(),
                            file: file.to_string(),
                            selector: selector.to_string(),
                            message: "JSON-LD FAQPage mainEntity item is missing 'acceptedAnswer'".into(),
                            help: "Each Question in FAQPage must have an \"acceptedAnswer\": {\"@type\": \"Answer\", \"text\": \"...\"}".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                        break; // report once per block
                    }
                }
            }
        }
        "WebSite" if entity.get("potentialAction").is_none() => {
            // Info: potentialAction (SearchAction)
            findings.push(Finding {
                level: Level::Info,
                rule_id: "structured-data/website-missing-search-action".into(),
                file: file.to_string(),
                selector: selector.to_string(),
                message: "JSON-LD WebSite has no 'potentialAction' (SearchAction)".into(),
                help: "Add a SearchAction to enable Google Sitelinks search box".into(),
                suggestion: None,
                source_hint: None,
                confidence: None,
            });
        }
        "BreadcrumbList" => {
            // Each ListItem must have position and item.name
            if let Some(items) = entity.get("itemListElement").and_then(|v| v.as_array()) {
                for (i, item) in items.iter().enumerate() {
                    if item.get("position").is_none() {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "structured-data/breadcrumb-missing-position".into(),
                            file: file.to_string(),
                            selector: selector.to_string(),
                            message: format!(
                                "JSON-LD BreadcrumbList item [{}] is missing 'position'",
                                i
                            ),
                            help: "Each ListItem must have \"position\": N (1-based index)".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                    }
                    let has_name = item
                        .get("item")
                        .and_then(|it| it.get("name"))
                        .and_then(|n| n.as_str())
                        .is_some_and(|n| !n.is_empty());
                    let has_name_direct = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .is_some_and(|n| !n.is_empty());
                    if !has_name && !has_name_direct {
                        findings.push(Finding {
                            level: Level::Error,
                            rule_id: "structured-data/breadcrumb-missing-name".into(),
                            file: file.to_string(),
                            selector: selector.to_string(),
                            message: format!(
                                "JSON-LD BreadcrumbList item [{}] is missing 'name'",
                                i
                            ),
                            help: "Each ListItem must have a \"name\" or \"item\": {\"name\": \"...\"}".into(),
                            suggestion: None,
                            source_hint: None,
                            confidence: None,
                        });
                    }
                }
            }
        }
        _ => {}
    }
}
