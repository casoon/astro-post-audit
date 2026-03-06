use scraper::Selector;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.i18n_audit.enabled {
        return Vec::new();
    }

    let hreflang_sel = Selector::parse("link[rel='alternate'][hreflang][href]").unwrap();
    let mut findings = Vec::new();

    for page in &index.pages {
        let html = page.parse_html();
        let inferred_locale = infer_locale_from_route(&page.route);
        let html_lang = page.html_lang.as_deref().map(normalize_lang);

        if let Some(route_locale) = inferred_locale {
            if let Some(lang) = &html_lang {
                if !same_language_family(route_locale, lang) {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "i18n/lang-locale-mismatch".into(),
                        file: page.rel_path.clone(),
                        selector: "html[lang]".into(),
                        message: format!(
                            "Route locale '{}' does not match html lang '{}'",
                            route_locale, lang
                        ),
                        help: "Align route locale and html lang for consistent i18n signals".into(),
                        suggestion: None,
                    });
                }
            } else {
                findings.push(Finding {
                    level: Level::Warning,
                    rule_id: "i18n/lang-missing-for-locale-route".into(),
                    file: page.rel_path.clone(),
                    selector: "html".into(),
                    message: format!(
                        "Route looks localized ('{}') but html lang is missing",
                        route_locale
                    ),
                    help:
                        "Set html lang to the page locale (for example lang=\"en\" or lang=\"de\")"
                            .into(),
                    suggestion: None,
                });
            }
        }

        let hreflangs: Vec<(String, String)> = html
            .select(&hreflang_sel)
            .filter_map(|el| {
                let lang = el.value().attr("hreflang")?.to_string();
                let href = el.value().attr("href")?.to_string();
                Some((lang, href))
            })
            .collect();

        if !hreflangs.is_empty() {
            if let (Some(canonical), Some(base)) = (&page.canonical, index.base_url.as_deref()) {
                let canonical_norm = normalize::normalize_path(
                    &normalize::resolve_href(canonical, &page.route, Some(base))
                        .unwrap_or_else(|| canonical.clone()),
                    &config.url_normalization,
                );
                let has_canonical_in_hreflang = hreflangs.iter().any(|(_, href)| {
                    normalize::resolve_href(href, &page.route, Some(base))
                        .map(|r| normalize::normalize_path(&r, &config.url_normalization))
                        .is_some_and(|r| r == canonical_norm)
                });
                if !has_canonical_in_hreflang {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "i18n/canonical-not-in-hreflang-set".into(),
                        file: page.rel_path.clone(),
                        selector: "link[rel='alternate'][hreflang]".into(),
                        message: "Canonical URL is not represented in hreflang alternates".into(),
                        help: "Add a self hreflang entry matching the canonical URL".into(),
                        suggestion: None,
                    });
                }
            }

            if let Some(route_locale) = inferred_locale {
                let has_matching_hreflang = hreflangs
                    .iter()
                    .any(|(lang, _)| same_language_family(route_locale, &normalize_lang(lang)));
                if !has_matching_hreflang {
                    findings.push(Finding {
                        level: Level::Warning,
                        rule_id: "i18n/no-matching-hreflang-for-route-locale".into(),
                        file: page.rel_path.clone(),
                        selector: "link[rel='alternate'][hreflang]".into(),
                        message: format!(
                            "No hreflang entry matches the route locale '{}'",
                            route_locale
                        ),
                        help: "Add hreflang entries that include the page locale".into(),
                        suggestion: None,
                    });
                }
            }
        }
    }

    findings
}

fn infer_locale_from_route(route: &str) -> Option<&str> {
    let first = route.trim_start_matches('/').split('/').next()?;
    if first.is_empty() {
        return None;
    }
    if is_locale_token(first) {
        Some(first)
    } else {
        None
    }
}

fn is_locale_token(token: &str) -> bool {
    let parts: Vec<&str> = token.split('-').collect();
    match parts.as_slice() {
        [lang] if lang.len() == 2 => lang.chars().all(|c| c.is_ascii_alphabetic()),
        [lang, region] if lang.len() == 2 && (region.len() == 2 || region.len() == 3) => {
            lang.chars().all(|c| c.is_ascii_alphabetic())
                && region.chars().all(|c| c.is_ascii_alphabetic())
        }
        _ => false,
    }
}

fn normalize_lang(lang: &str) -> String {
    lang.trim().to_lowercase().replace('_', "-")
}

fn same_language_family(a: &str, b: &str) -> bool {
    a.split('-').next().is_some_and(|la| {
        b.split('-')
            .next()
            .is_some_and(|lb| la.eq_ignore_ascii_case(lb))
    })
}
