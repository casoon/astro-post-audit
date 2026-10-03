use std::collections::HashMap;
use std::sync::LazyLock;

use scraper::Selector;
use url::Url;
use web_checks::hreflang::{self as wc, Alternate};

use crate::config::{Config, UrlNormalizationConfig};
use crate::discovery::SiteIndex;
use crate::normalize;
use crate::report::{Finding, Location, Severity};

static HREFLANG_SEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("link[rel='alternate'][hreflang]").expect("valid selector"));

/// Codes, `x-default` and the self-reference come from `web-checks`, shared
/// with auditmysite. Reciprocal links and target existence need the whole
/// build and stay here.
pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.hreflang.check_hreflang {
        return Vec::new();
    }

    let mut findings = Vec::new();
    let norm_cfg = &config.url_normalization;
    let route_by_abs_url: HashMap<String, String> = index
        .pages
        .iter()
        .filter_map(|p| {
            let url = absolute(p.absolute_url.as_deref()?, "", norm_cfg)?;
            Some((url, p.route.clone()))
        })
        .collect();

    // page route -> its alternates, hrefs resolved against the page URL
    let mut all_hreflangs: HashMap<String, Vec<Alternate>> = HashMap::new();

    for page in &index.pages {
        let html = page.parse_html();

        let entries: Vec<(String, String)> = html
            .select(&HREFLANG_SEL)
            .filter_map(|el| {
                let lang = el.value().attr("hreflang")?.to_string();
                let href = el.value().attr("href")?.to_string();
                Some((lang, href))
            })
            .collect();

        if entries.is_empty() {
            continue;
        }

        let page_url = page
            .absolute_url
            .as_deref()
            .and_then(|u| absolute(u, "", norm_cfg));
        // Resolved like the browser does; without a site URL they stay as written.
        let alternates: Vec<Alternate> = entries
            .iter()
            .map(|(lang, href)| Alternate {
                hreflang: lang.clone(),
                href: page_url
                    .as_deref()
                    .and_then(|base| absolute(base, href, norm_cfg))
                    .unwrap_or_else(|| href.clone()),
            })
            .collect();

        if config.hreflang.require_valid_code {
            for alt in wc::invalid_codes(&alternates) {
                findings.push(
                    Finding::fail(
                        "hreflang/invalid-code",
                        format!("Invalid hreflang value '{}'", alt.hreflang),
                    )
                    .with_severity(Severity::Medium)
                    .at(Location::file(page.rel_path.clone())
                        .with_selector(format!("link[hreflang='{}']", alt.hreflang)))
                    .with_help("Use a language code (ISO 639), optionally with script and region, e.g. \"de\", \"de-AT\" or \"zh-Hant-TW\" — or \"x-default\". Search engines ignore other values."),
                );
            }
        }

        if config.hreflang.require_x_default && !wc::has_x_default(&alternates) {
            findings.push(
                Finding::fail(
                    "hreflang/no-x-default",
                    "Hreflang tags present but no x-default",
                )
                .with_severity(Severity::Medium)
                .at(Location::file(page.rel_path.clone())
                    .with_selector("link[rel='alternate'][hreflang]"))
                .with_help("Add <link rel=\"alternate\" hreflang=\"x-default\" href=\"...\">"),
            );
        }

        if config.hreflang.require_self_reference {
            if let Some(page_url) = &page_url {
                if !wc::has_self_reference(&alternates, page_url) {
                    findings.push(
                        Finding::fail(
                            "hreflang/no-self-reference",
                            "Hreflang tags don't include a self-reference",
                        )
                        .with_severity(Severity::Medium)
                        .at(Location::file(page.rel_path.clone())
                            .with_selector("link[rel='alternate'][hreflang]"))
                        .with_help("Include the current page URL under its own language code; x-default does not count"),
                    );
                }
            }
        }

        // Check that internal hreflang targets actually exist in the build
        if config.hreflang.require_target_exists {
            for (lang, href) in &entries {
                if wc::is_x_default(lang) {
                    continue;
                }
                if !normalize::is_internal(href, index.base_url.as_deref()) {
                    continue; // can't verify cross-origin targets statically
                }
                if let Some(resolved) =
                    normalize::resolve_href(href, &page.route, index.base_url.as_deref())
                {
                    let normalized =
                        normalize::normalize_path(&resolved, &config.url_normalization);
                    if !index.route_exists(&normalized) {
                        findings.push(Finding::fail("hreflang/target-missing", format!(
                                "Hreflang target '{}' (lang='{}') does not exist in the build",
                                href, lang
                            ))
.with_severity(Severity::Medium)
.at(Location::file(page.rel_path.clone()).with_selector(format!("link[hreflang='{}'][href='{}']", lang, href)))
.with_help("Ensure the translated page is generated, or fix the hreflang href."));
                    }
                }
            }
        }

        all_hreflangs.insert(page.route.clone(), alternates);
    }

    // Check reciprocal references
    if config.hreflang.require_reciprocal {
        for (source_route, alternates) in &all_hreflangs {
            let Some(source) = index.pages.iter().find(|p| p.route == *source_route) else {
                continue;
            };
            let Some(source_url) = source
                .absolute_url
                .as_deref()
                .and_then(|u| absolute(u, "", norm_cfg))
            else {
                continue;
            };
            for alt in alternates {
                if wc::is_x_default(&alt.hreflang) || wc::same_page(&alt.href, &source_url) {
                    continue;
                }
                let Some(target_entries) = route_by_abs_url
                    .get(&alt.href)
                    .and_then(|route| all_hreflangs.get(route))
                else {
                    continue;
                };
                let has_reciprocal = target_entries
                    .iter()
                    .any(|t| wc::same_page(&t.href, &source_url));
                if !has_reciprocal {
                    findings.push(
                        Finding::fail(
                            "hreflang/no-reciprocal",
                            format!(
                                "Hreflang target '{}' (lang='{}') doesn't link back",
                                alt.href, alt.hreflang
                            ),
                        )
                        .with_severity(Severity::Medium)
                        .at(Location::file(source.rel_path.clone())
                            .with_selector(format!("link[hreflang='{}']", alt.hreflang)))
                        .with_help("Add reciprocal hreflang link on the target page"),
                    );
                }
            }
        }
    }

    findings
}

/// `href` resolved against `base` (an absolute URL), with the path normalized
/// per `url_normalization`. The query stays — some sites tell their language
/// versions apart only by it (`?lang=de`); the fragment goes.
fn absolute(base: &str, href: &str, norm: &UrlNormalizationConfig) -> Option<String> {
    let mut url = Url::parse(base).ok()?.join(href).ok()?;
    url.set_path(&normalize::normalize_path(url.path(), norm));
    url.set_fragment(None);
    Some(url.to_string())
}
