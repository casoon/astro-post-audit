use std::sync::LazyLock;

use rayon::prelude::*;
use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Location, Severity};
use web_checks::structured_data::{self as sd, StructuralIssue};

static LD_SEL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("script[type='application/ld+json']").expect("valid selector")
});

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.structured_data.check_json_ld
        && !config.structured_data.require_json_ld
        && !config.structured_data.detect_duplicate_types
    {
        return Vec::new();
    }

    index
        .pages
        .par_iter()
        .flat_map(|page| {
            let mut findings = Vec::new();
            let html = page.parse_html();

            let scripts: Vec<_> = html.select(&LD_SEL).collect();

            if scripts.is_empty() {
                if config.structured_data.require_json_ld {
                    findings.push(
                        Finding::fail(
                            "structured-data/missing",
                            "No JSON-LD structured data found",
                        )
                        .with_severity(Severity::Medium)
                        .at(Location::file(page.rel_path.clone()).with_selector("head"))
                        .with_help(
                            "Add <script type=\"application/ld+json\"> with schema.org data",
                        ),
                    );
                }
                return findings;
            }

            let texts: Vec<String> = scripts.iter().map(|s| s.text().collect()).collect();
            let blocks = sd::parse_blocks(&texts);
            let selector = |i: usize| format!("script[type='application/ld+json']:nth({})", i + 1);

            if config.structured_data.check_json_ld {
                for (i, block) in blocks.iter().enumerate() {
                    let at = Location::file(page.rel_path.clone()).with_selector(selector(i));
                    for issue in &block.issues {
                        findings.push(structural_finding(*issue, block, at.clone()));
                    }
                    for (node_index, node) in block.nodes.iter().enumerate() {
                        for schema_type in &node.types {
                            for assessment in sd::assess_node(
                                node_index,
                                schema_type,
                                &node.content,
                                sd::ProductRuleContext::Indeterminate,
                            ) {
                                assessment_findings(&assessment, &at, &mut findings);
                            }
                        }
                    }
                }
            }

            // Duplicate @type across JSON-LD blocks on the same page
            if config.structured_data.detect_duplicate_types {
                for dup in sd::duplicate_types(&blocks) {
                    let selectors: Vec<String> = dup.blocks.iter().map(|&i| selector(i)).collect();
                    findings.push(
                        Finding::fail(
                            "structured-data/duplicate-type",
                            format!(
                                "Duplicate JSON-LD @type '{}' found {} times on this page",
                                dup.schema_type,
                                dup.blocks.len()
                            ),
                        )
                        .with_severity(Severity::Medium)
                        .at(Location::file(page.rel_path.clone())
                            .with_selector(selectors.join(", ")))
                        .with_help(format!(
                            "Consolidate {} blocks into a single JSON-LD script or use @graph",
                            dup.schema_type
                        )),
                    );
                }
            }

            findings
        })
        .collect()
}

/// A structural problem from `web_checks::structured_data` as a finding.
fn structural_finding(issue: StructuralIssue, block: &sd::Block, at: Location) -> Finding {
    let (rule, severity, message, help) = match issue {
        StructuralIssue::EmptyScript => (
            "structured-data/empty",
            Severity::High,
            "JSON-LD script is empty".to_string(),
            "Add valid JSON-LD content or remove the empty script tag",
        ),
        StructuralIssue::InvalidJson => (
            "structured-data/invalid-json",
            Severity::High,
            format!(
                "Invalid JSON in JSON-LD: {}",
                block.json_error.as_deref().unwrap_or_default()
            ),
            "Fix the JSON syntax in the structured data block",
        ),
        StructuralIssue::InvalidRoot | StructuralIssue::GraphNotArray => (
            "structured-data/invalid-structure",
            Severity::High,
            "JSON-LD root, array entry or @graph has the wrong shape".to_string(),
            "The root must be an object or an array of objects, and @graph an array",
        ),
        StructuralIssue::EmptyDocument => (
            "structured-data/invalid-structure",
            Severity::Medium,
            "JSON-LD array or @graph contains no nodes".to_string(),
            "Add nodes or remove the empty structured data",
        ),
        StructuralIssue::MissingContext | StructuralIssue::GraphWithoutContext => (
            "structured-data/missing-context",
            Severity::Medium,
            "JSON-LD missing @context property".to_string(),
            "Add \"@context\": \"https://schema.org\" to the JSON-LD object",
        ),
        StructuralIssue::NonSchemaOrgContext => (
            "structured-data/unusual-context",
            Severity::Medium,
            "JSON-LD @context is not schema.org".to_string(),
            "Use \"https://schema.org\" as the @context",
        ),
        StructuralIssue::MissingType => (
            "structured-data/missing-type",
            Severity::Medium,
            "JSON-LD entity missing @type property".to_string(),
            "Add an @type property (e.g. \"Article\", \"WebPage\")",
        ),
    };
    Finding::fail(rule, message)
        .with_severity(severity)
        .at(at)
        .with_help(help)
}

/// Findings for one rule assessment. Missing **required** properties are
/// always reported; missing **recommended** ones only where this tool has a
/// dedicated rule id — the rest lowers no score here, as in auditmysite.
fn assessment_findings(
    assessment: &sd::SchemaRuleAssessment,
    at: &Location,
    findings: &mut Vec<Finding>,
) {
    let schema_type = assessment.schema_type.as_str();
    for property in &assessment.missing_required {
        let (rule, severity) = required_rule(schema_type, property);
        findings.push(
            Finding::fail(
                rule,
                format!("JSON-LD {schema_type} is missing required property '{property}'"),
            )
            .with_severity(severity)
            .at(at.clone())
            .with_help(format!(
                "Add '{property}' to the {schema_type} schema (see {})",
                assessment.source_url
            )),
        );
    }
    for property in &assessment.missing_recommended {
        let Some((rule, severity)) = recommended_rule(schema_type, property) else {
            continue;
        };
        findings.push(
            Finding::fail(
                rule,
                format!("JSON-LD {schema_type} is missing recommended property '{property}'"),
            )
            .with_severity(severity)
            .at(at.clone())
            .with_help(format!(
                "Add '{property}' to the {schema_type} schema (see {})",
                assessment.source_url
            )),
        );
    }
}

/// Rule id and severity for a missing required property. The dedicated ids
/// from before web-checks 0.4 stay where the question is the same.
fn required_rule(schema_type: &str, property: &str) -> (&'static str, Severity) {
    if schema_type == "BreadcrumbList" && property.ends_with(".position") {
        (
            "structured-data/breadcrumb-missing-position",
            Severity::High,
        )
    } else if schema_type == "BreadcrumbList" && property.ends_with(".name") {
        ("structured-data/breadcrumb-missing-name", Severity::High)
    } else if schema_type == "FAQPage" && property.ends_with(".acceptedAnswer") {
        ("structured-data/faq-missing-answer", Severity::High)
    } else if schema_type == "LocalBusiness" && property == "address" {
        (
            "structured-data/local-business-missing-address",
            Severity::Medium,
        )
    } else {
        ("structured-data/missing-property", Severity::Medium)
    }
}

/// Rule id and severity for a missing recommended property that this tool
/// reports; `None` for the others.
fn recommended_rule(schema_type: &str, property: &str) -> Option<(&'static str, Severity)> {
    let article = matches!(schema_type, "Article" | "BlogPosting" | "NewsArticle");
    Some(match property {
        "author" if article => ("structured-data/article-missing-author", Severity::Medium),
        "datePublished" if article => (
            "structured-data/article-missing-date-published",
            Severity::Medium,
        ),
        "image" if article => ("structured-data/article-missing-image", Severity::Medium),
        "dateModified" if article => (
            "structured-data/article-missing-date-modified",
            Severity::Low,
        ),
        "publisher" if schema_type == "NewsArticle" => (
            "structured-data/news-article-missing-publisher",
            Severity::Medium,
        ),
        "url" if matches!(schema_type, "Organization" | "LocalBusiness") => {
            ("structured-data/organization-missing-url", Severity::Medium)
        }
        "telephone" if schema_type == "LocalBusiness" => (
            "structured-data/local-business-missing-telephone",
            Severity::Low,
        ),
        "potentialAction" if schema_type == "WebSite" => (
            "structured-data/website-missing-search-action",
            Severity::Low,
        ),
        _ => return None,
    })
}
