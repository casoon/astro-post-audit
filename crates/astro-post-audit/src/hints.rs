use std::path::Path;

/// Try to find the Astro source file corresponding to a dist-relative HTML path.
/// Returns a path relative to the project root, e.g. `src/content/blog/post.mdx`.
/// Returns `None` if no matching source file is found.
pub fn find_source(rel_html_path: &str, project_root: &str) -> Option<String> {
    // Normalise: "blog/post/index.html" → "blog/post", "about.html" → "about"
    let slug = rel_html_path
        .trim_end_matches("/index.html")
        .trim_end_matches(".html")
        .trim_start_matches('/')
        .trim_end_matches('/');

    let root = Path::new(project_root);

    // Candidates to probe (relative to project root)
    let candidates: &[String] = &[
        format!("src/pages/{slug}.astro"),
        format!("src/pages/{slug}.md"),
        format!("src/pages/{slug}.mdx"),
        format!("src/pages/{slug}/index.astro"),
        format!("src/pages/{slug}/index.md"),
        format!("src/pages/{slug}/index.mdx"),
        format!("src/content/{slug}.md"),
        format!("src/content/{slug}.mdx"),
    ];

    for candidate in candidates {
        if root.join(candidate).exists() {
            return Some(candidate.clone());
        }
    }
    None
}
