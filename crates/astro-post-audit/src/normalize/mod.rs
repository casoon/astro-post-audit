use url::Url;

use crate::config::{IndexHtml, TrailingSlash, UrlNormalizationConfig};

/// Collapse `.` and `..` segments in a URL path (without filesystem access).
fn collapse_dots(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(seg),
        }
    }
    let result = segments.join("/");
    if path.starts_with('/') && !result.starts_with('/') {
        format!("/{}", result)
    } else {
        result
    }
}

/// Normalize a URL path according to the configured rules.
///
/// - Collapses `.` and `..` path segments
/// - Strips /index.html and /index.htm suffix if index_html == Forbid
/// - Ensures or removes trailing slash based on trailing_slash config
/// - Always preserves "/" as-is
pub fn normalize_path(path: &str, config: &UrlNormalizationConfig) -> String {
    let mut p = collapse_dots(path);

    // Strip index.html / index.htm suffix
    if config.index_html == IndexHtml::Forbid {
        if p.ends_with("/index.html") {
            p = p.trim_end_matches("index.html").to_string();
        } else if p.ends_with("/index.htm") {
            p = p.trim_end_matches("index.htm").to_string();
        } else if p == "index.html" || p == "/index.html" || p == "index.htm" || p == "/index.htm" {
            p = "/".to_string();
        }
    }

    // Handle trailing slash (skip for root "/")
    if p != "/" {
        match config.trailing_slash {
            TrailingSlash::Always => {
                if !p.ends_with('/') {
                    p.push('/');
                }
            }
            TrailingSlash::Never => {
                if p.ends_with('/') && p.len() > 1 {
                    p = p.trim_end_matches('/').to_string();
                }
            }
            TrailingSlash::Ignore => {}
        }
    }

    p
}

/// Convert a file path relative to dist/ into a normalized route URL path.
///
/// e.g., "about/index.html" -> "/about/"
///       "blog/post.html"   -> "/blog/post/"
pub fn file_path_to_route(rel_path: &str, config: &UrlNormalizationConfig) -> String {
    let mut route = format!("/{}", rel_path.replace('\\', "/"));

    // Strip .html/.htm extension for non-index files
    if route.ends_with("/index.html") {
        route = route.trim_end_matches("index.html").to_string();
    } else if route.ends_with("/index.htm") {
        route = route.trim_end_matches("index.htm").to_string();
    } else if route.ends_with(".html") {
        route = route.trim_end_matches(".html").to_string();
    } else if route.ends_with(".htm") {
        route = route.trim_end_matches(".htm").to_string();
    }

    // Ensure root is "/"
    if route.is_empty() {
        route = "/".to_string();
    }

    normalize_path(&route, config)
}

/// Build an absolute URL from a route path and base URL.
pub fn to_absolute(route: &str, base_url: &str) -> Option<String> {
    let base = Url::parse(base_url).ok()?;
    let joined = base.join(route).ok()?;
    Some(joined.to_string())
}

/// Strip fragment and query from a URL string, returning just the path portion.
pub fn strip_fragment_and_query(href: &str) -> &str {
    let s = href.split('#').next().unwrap_or(href);
    s.split('?').next().unwrap_or(s)
}

/// Check if a URL/href is internal (relative or same-origin).
pub fn is_internal(href: &str, base_url: Option<&str>) -> bool {
    // Relative URLs are always internal
    if !href.contains("://") && !href.starts_with("//") {
        return true;
    }

    // Protocol-relative URLs
    if href.starts_with("//") {
        if let Some(base) = base_url {
            if let Ok(base_parsed) = Url::parse(base) {
                if let Ok(href_parsed) = Url::parse(&format!("{}:{}", base_parsed.scheme(), href)) {
                    return href_parsed.host() == base_parsed.host();
                }
            }
        }
        return false;
    }

    // Absolute URLs - compare origin
    if let Some(base) = base_url {
        if let (Ok(base_parsed), Ok(href_parsed)) = (Url::parse(base), Url::parse(href)) {
            return href_parsed.origin() == base_parsed.origin();
        }
    }

    false
}

/// Check if href contains query parameters.
pub fn has_query_params(href: &str) -> bool {
    // Check before fragment
    let without_fragment = href.split('#').next().unwrap_or(href);
    without_fragment.contains('?')
}

/// Resolve a relative href against a page's route to get a normalized path.
pub fn resolve_href(href: &str, page_route: &str, base_url: Option<&str>) -> Option<String> {
    let clean = strip_fragment_and_query(href);

    if clean.is_empty() {
        return Some(page_route.to_string());
    }

    // Absolute URL
    if clean.contains("://") || clean.starts_with("//") {
        if let Ok(parsed) = Url::parse(clean) {
            return Some(parsed.path().to_string());
        }
        if let Some(base) = base_url {
            if let Ok(base_parsed) = Url::parse(base) {
                let full = format!("{}:{}", base_parsed.scheme(), clean);
                if let Ok(parsed) = Url::parse(&full) {
                    return Some(parsed.path().to_string());
                }
            }
        }
        return None;
    }

    // Relative URL - resolve against page route
    if clean.starts_with('/') {
        // Absolute path — collapse any `.`/`..` segments
        Some(collapse_dots(clean))
    } else {
        // Relative path - resolve against page directory
        let page_dir = if page_route.ends_with('/') {
            page_route.to_string()
        } else {
            match page_route.rfind('/') {
                Some(pos) => page_route[..=pos].to_string(),
                None => "/".to_string(),
            }
        };
        Some(collapse_dots(&format!("{}{}", page_dir, clean)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UrlNormalizationConfig;

    fn always_slash() -> UrlNormalizationConfig {
        UrlNormalizationConfig {
            trailing_slash: TrailingSlash::Always,
            index_html: IndexHtml::Forbid,
        }
    }

    fn never_slash() -> UrlNormalizationConfig {
        UrlNormalizationConfig {
            trailing_slash: TrailingSlash::Never,
            index_html: IndexHtml::Forbid,
        }
    }

    #[test]
    fn test_normalize_path_trailing_always() {
        let c = always_slash();
        assert_eq!(normalize_path("/about", &c), "/about/");
        assert_eq!(normalize_path("/about/", &c), "/about/");
        assert_eq!(normalize_path("/", &c), "/");
        assert_eq!(normalize_path("/about/index.html", &c), "/about/");
    }

    #[test]
    fn test_normalize_path_trailing_never() {
        let c = never_slash();
        assert_eq!(normalize_path("/about/", &c), "/about");
        assert_eq!(normalize_path("/about", &c), "/about");
        assert_eq!(normalize_path("/", &c), "/");
    }

    #[test]
    fn test_file_path_to_route() {
        let c = always_slash();
        assert_eq!(file_path_to_route("index.html", &c), "/");
        assert_eq!(file_path_to_route("about/index.html", &c), "/about/");
        assert_eq!(file_path_to_route("blog/post.html", &c), "/blog/post/");
        assert_eq!(
            file_path_to_route("blog/2024/my-post.html", &c),
            "/blog/2024/my-post/"
        );
    }

    #[test]
    fn test_is_internal() {
        assert!(is_internal("/about", None));
        assert!(is_internal("about/page", None));
        assert!(is_internal("../other", None));
        assert!(!is_internal(
            "https://other.com/page",
            Some("https://example.com")
        ));
        assert!(is_internal(
            "https://example.com/about",
            Some("https://example.com")
        ));
    }

    #[test]
    fn test_has_query_params() {
        assert!(has_query_params("/page?foo=bar"));
        assert!(has_query_params("/page?foo=bar#section"));
        assert!(!has_query_params("/page"));
        assert!(!has_query_params("/page#section"));
    }

    #[test]
    fn test_strip_fragment_and_query() {
        assert_eq!(strip_fragment_and_query("/page?foo=bar#section"), "/page");
        assert_eq!(strip_fragment_and_query("/page#section"), "/page");
        assert_eq!(strip_fragment_and_query("/page"), "/page");
    }

    #[test]
    fn test_resolve_href() {
        assert_eq!(
            resolve_href("/about", "/blog/", None),
            Some("/about".to_string())
        );
        assert_eq!(
            resolve_href("other", "/blog/", None),
            Some("/blog/other".to_string())
        );
        assert_eq!(
            resolve_href("sub/page", "/blog/", None),
            Some("/blog/sub/page".to_string())
        );
        assert_eq!(
            resolve_href("#section", "/blog/", None),
            Some("/blog/".to_string())
        );
    }

    #[test]
    fn test_to_absolute() {
        assert_eq!(
            to_absolute("/about/", "https://example.com"),
            Some("https://example.com/about/".to_string())
        );
        assert_eq!(
            to_absolute("/", "https://example.com"),
            Some("https://example.com/".to_string())
        );
    }

    #[test]
    fn test_collapse_dots() {
        assert_eq!(collapse_dots("/blog/posts/../contact"), "/blog/contact");
        assert_eq!(collapse_dots("/a/b/c/../../d"), "/a/d");
        assert_eq!(collapse_dots("/a/./b"), "/a/b");
        assert_eq!(collapse_dots("/a/b/./c/../d"), "/a/b/d");
        assert_eq!(collapse_dots("/"), "/");
        assert_eq!(collapse_dots("/../a"), "/a");
    }

    #[test]
    fn test_resolve_href_dotdot() {
        assert_eq!(
            resolve_href("../contact", "/blog/posts/", None),
            Some("/blog/contact".to_string())
        );
        assert_eq!(
            resolve_href("../../about", "/blog/posts/", None),
            Some("/about".to_string())
        );
        assert_eq!(
            resolve_href("./sibling", "/blog/posts/", None),
            Some("/blog/posts/sibling".to_string())
        );
    }

    #[test]
    fn test_normalize_path_htm() {
        let c = always_slash();
        assert_eq!(normalize_path("/about/index.htm", &c), "/about/");
        assert_eq!(normalize_path("/index.htm", &c), "/");
    }

    #[test]
    fn test_file_path_to_route_htm() {
        let c = always_slash();
        assert_eq!(file_path_to_route("index.htm", &c), "/");
        assert_eq!(file_path_to_route("about/index.htm", &c), "/about/");
        assert_eq!(file_path_to_route("blog/post.htm", &c), "/blog/post/");
    }
}
