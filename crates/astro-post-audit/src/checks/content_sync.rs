use std::collections::HashSet;
use std::path::Path;

use walkdir::WalkDir;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Confidence, Finding, Level};

const CONTENT_EXTENSIONS: &[&str] = &["md", "mdx", "markdown", "mdoc"];

/// Cross-check `src/content/` collection items against generated pages. Because
/// the audit runs post-build, content files that were never rendered (e.g. a
/// broken `getStaticPaths` filter or slug mapping) can be surfaced.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.content_sync.enabled {
        return Vec::new();
    }
    let Some(root) = config.project_root.as_deref() else {
        return Vec::new();
    };
    let content_dir = Path::new(root).join("src").join("content");
    if !content_dir.is_dir() {
        return Vec::new();
    }

    // All route path segments that exist in the build, for slug matching.
    let segments: HashSet<String> = index
        .pages
        .iter()
        .flat_map(|p| {
            p.route
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_lowercase())
        })
        .collect();

    let mut findings = Vec::new();

    for entry in WalkDir::new(&content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !CONTENT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        // Astro ignores underscore-prefixed files/dirs; skip config and collection indexes.
        let rel = path.strip_prefix(root).unwrap_or(path);
        if rel
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('_'))
        {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if stem.is_empty() || stem == "index" {
            continue;
        }

        // A frontmatter `slug:` overrides the filename-derived slug.
        let slug = frontmatter_slug(path).unwrap_or(stem);

        let matched = if slug.contains('/') {
            // Multi-segment slug: require all segments to be present.
            slug.split('/')
                .filter(|s| !s.is_empty())
                .all(|s| segments.contains(&s.to_lowercase()))
        } else {
            segments.contains(&slug.to_lowercase())
        };

        if !matched {
            let display = rel.to_string_lossy().replace('\\', "/");
            findings.push(Finding {
                level: Level::Warning,
                rule_id: "content/missing-page".into(),
                file: display.clone(),
                selector: String::new(),
                message: format!(
                    "Content item '{}' has no corresponding build page",
                    display
                ),
                help: "Check your slug mapping or filter criteria in getStaticPaths — this content was not rendered.".into(),
                suggestion: None,
                source_hint: None,
                confidence: Some(Confidence::Low),
            });
        }
    }

    findings
}

/// Read a `slug:` value from a file's leading YAML frontmatter, if present.
fn frontmatter_slug(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    // Frontmatter is between the first two `---` fences.
    let after = &trimmed[3..];
    let end = after.find("\n---")?;
    let frontmatter = &after[..end];
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("slug:") {
            let value = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !value.is_empty() {
                return Some(value.trim_matches('/').to_string());
            }
        }
    }
    None
}
