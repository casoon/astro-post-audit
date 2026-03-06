use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Strict,
    Relaxed,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Preset to apply before user overrides.
    pub preset: Option<Preset>,
    /// Treat warnings as errors (exit code 1).
    pub strict: bool,
    /// Maximum number of errors before truncating output.
    pub max_errors: Option<usize>,
    /// Show page properties overview instead of running checks.
    pub page_overview: bool,
    /// Output format: "text" (default) or "json".
    pub format: Option<String>,
    /// Print per-check timing benchmarks.
    pub benchmark: bool,
    pub site: SiteConfig,
    pub filters: FilterConfig,
    pub url_normalization: UrlNormalizationConfig,
    pub canonical: CanonicalConfig,
    pub robots_meta: RobotsMetaConfig,
    pub links: LinksConfig,
    pub sitemap: SitemapConfig,
    pub html_basics: HtmlBasicsConfig,
    pub headings: HeadingsConfig,
    pub a11y: A11yConfig,
    pub assets: AssetsConfig,
    pub opengraph: OpenGraphConfig,
    pub structured_data: StructuredDataConfig,
    pub hreflang: HreflangConfig,
    pub security: SecurityConfig,
    pub content_quality: ContentQualityConfig,
    pub external_links: ExternalLinksConfig,
    pub robots_txt: RobotsTxtConfig,
    pub i18n_audit: I18nAuditConfig,
    pub crawl_budget: CrawlBudgetConfig,
    pub render_blocking: RenderBlockingConfig,
    pub privacy_security: PrivacySecurityConfig,
    pub structured_data_graph: StructuredDataGraphConfig,
    pub severity: SeverityConfig,
}

/// Custom severity overrides per rule ID.
/// Maps rule IDs (e.g. "links/orphan-page") to severity levels.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SeverityConfig {
    #[serde(flatten)]
    pub overrides: HashMap<String, SeverityLevel>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SeverityLevel {
    Error,
    Warning,
    Info,
    Off,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SiteConfig {
    pub base_url: Option<String>,
}

/// File include/exclude patterns (merged with CLI --include/--exclude).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FilterConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UrlNormalizationConfig {
    pub trailing_slash: TrailingSlash,
    pub index_html: IndexHtml,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrailingSlash {
    Always,
    Never,
    Ignore,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IndexHtml {
    Forbid,
    Allow,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CanonicalConfig {
    pub require: bool,
    pub absolute: bool,
    pub same_origin: bool,
    pub self_reference: bool,
    pub detect_clusters: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RobotsMetaConfig {
    pub allow_noindex: bool,
    pub fail_if_noindex: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LinksConfig {
    pub check_internal: bool,
    pub fail_on_broken: bool,
    pub forbid_query_params_internal: bool,
    pub check_fragments: bool,
    pub detect_orphan_pages: bool,
    pub check_mixed_content: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SitemapConfig {
    pub require: bool,
    pub canonical_must_be_in_sitemap: bool,
    pub forbid_noncanonical_in_sitemap: bool,
    pub entries_must_exist_in_dist: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HtmlBasicsConfig {
    pub lang_attr_required: bool,
    pub title_required: bool,
    pub meta_description_required: bool,
    pub viewport_required: bool,
    pub title_max_length: Option<usize>,
    pub meta_description_max_length: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HeadingsConfig {
    pub require_h1: bool,
    pub single_h1: bool,
    pub no_skip: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct A11yConfig {
    pub img_alt_required: bool,
    pub allow_decorative_images: bool,
    pub a_accessible_name_required: bool,
    pub button_name_required: bool,
    pub label_for_required: bool,
    pub warn_generic_link_text: bool,
    pub aria_hidden_focusable_check: bool,
    pub require_skip_link: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AssetsConfig {
    pub check_broken_assets: bool,
    pub check_image_dimensions: bool,
    pub max_image_size_kb: Option<u64>,
    pub max_js_size_kb: Option<u64>,
    pub max_css_size_kb: Option<u64>,
    pub require_hashed_filenames: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct OpenGraphConfig {
    pub require_og_title: bool,
    pub require_og_description: bool,
    pub require_og_image: bool,
    pub require_twitter_card: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct StructuredDataConfig {
    pub check_json_ld: bool,
    pub require_json_ld: bool,
    pub detect_duplicate_types: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HreflangConfig {
    pub check_hreflang: bool,
    pub require_x_default: bool,
    pub require_self_reference: bool,
    pub require_reciprocal: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub check_target_blank: bool,
    pub check_mixed_content: bool,
    pub warn_inline_scripts: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ContentQualityConfig {
    pub detect_duplicate_titles: bool,
    pub detect_duplicate_descriptions: bool,
    pub detect_duplicate_h1: bool,
    pub detect_duplicate_pages: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExternalLinksConfig {
    pub enabled: bool,
    pub timeout_ms: u64,
    pub max_concurrent: usize,
    pub fail_on_broken: bool,
    pub allow_domains: Vec<String>,
    pub block_domains: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RobotsTxtConfig {
    pub require: bool,
    pub require_sitemap_link: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct I18nAuditConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CrawlBudgetConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RenderBlockingConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PrivacySecurityConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct StructuredDataGraphConfig {
    pub enabled: bool,
}

// --- Defaults (only for structs with non-zero defaults) ---

impl Default for UrlNormalizationConfig {
    fn default() -> Self {
        Self {
            trailing_slash: TrailingSlash::Always,
            index_html: IndexHtml::Forbid,
        }
    }
}

impl Default for CanonicalConfig {
    fn default() -> Self {
        Self {
            require: true,
            absolute: true,
            same_origin: true,
            self_reference: false,
            detect_clusters: true,
        }
    }
}

impl Default for RobotsMetaConfig {
    fn default() -> Self {
        Self {
            allow_noindex: true,
            fail_if_noindex: false,
        }
    }
}

impl Default for LinksConfig {
    fn default() -> Self {
        Self {
            check_internal: true,
            fail_on_broken: true,
            forbid_query_params_internal: true,
            check_fragments: false,
            detect_orphan_pages: false,
            check_mixed_content: true,
        }
    }
}

impl Default for SitemapConfig {
    fn default() -> Self {
        Self {
            require: false,
            canonical_must_be_in_sitemap: true,
            forbid_noncanonical_in_sitemap: false,
            entries_must_exist_in_dist: true,
        }
    }
}

impl Default for HtmlBasicsConfig {
    fn default() -> Self {
        Self {
            lang_attr_required: true,
            title_required: true,
            meta_description_required: false,
            viewport_required: true,
            title_max_length: Some(60),
            meta_description_max_length: Some(160),
        }
    }
}

impl Default for HeadingsConfig {
    fn default() -> Self {
        Self {
            require_h1: true,
            single_h1: true,
            no_skip: false,
        }
    }
}

impl Default for A11yConfig {
    fn default() -> Self {
        Self {
            img_alt_required: true,
            allow_decorative_images: true,
            a_accessible_name_required: true,
            button_name_required: true,
            label_for_required: true,
            warn_generic_link_text: true,
            aria_hidden_focusable_check: true,
            require_skip_link: false,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            check_target_blank: true,
            check_mixed_content: true,
            warn_inline_scripts: false,
        }
    }
}

impl Default for ExternalLinksConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout_ms: 3000,
            max_concurrent: 10,
            fail_on_broken: false,
            allow_domains: Vec::new(),
            block_domains: Vec::new(),
        }
    }
}

impl Config {
    pub fn from_json(json_str: &str) -> Result<Self> {
        // Two-pass deserialization: check which fields the user set,
        // then inject preset defaults for missing fields.
        let mut raw: serde_json::Value = serde_json::from_str(json_str)?;

        if let Some(preset_val) = raw.get("preset").and_then(|v| v.as_str()) {
            let preset_defaults = match preset_val {
                "strict" => Self::strict_preset_json(),
                "relaxed" => Self::relaxed_preset_json(),
                _ => serde_json::Value::Object(serde_json::Map::new()),
            };

            // Merge: preset defaults first, user values override
            if let (Some(defaults), Some(user)) = (preset_defaults.as_object(), raw.as_object_mut())
            {
                for (key, default_val) in defaults {
                    if !user.contains_key(key) {
                        user.insert(key.clone(), default_val.clone());
                    } else if let (Some(default_obj), Some(user_obj)) = (
                        default_val.as_object(),
                        user.get_mut(key).and_then(|v| v.as_object_mut()),
                    ) {
                        // Merge nested objects (e.g. canonical, links)
                        for (k, v) in default_obj {
                            if !user_obj.contains_key(k) {
                                user_obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
            }
        }

        let config: Config = serde_json::from_value(raw)?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if matches!(self.max_errors, Some(0)) {
            anyhow::bail!("max_errors must be greater than 0 when set");
        }
        if matches!(self.html_basics.title_max_length, Some(0)) {
            anyhow::bail!("html_basics.title_max_length must be greater than 0 when set");
        }
        if matches!(self.html_basics.meta_description_max_length, Some(0)) {
            anyhow::bail!(
                "html_basics.meta_description_max_length must be greater than 0 when set"
            );
        }
        if self.external_links.enabled {
            if self.external_links.timeout_ms == 0 {
                anyhow::bail!("external_links.timeout_ms must be greater than 0 when enabled");
            }
            if self.external_links.max_concurrent == 0 {
                anyhow::bail!("external_links.max_concurrent must be greater than 0 when enabled");
            }
        }
        Ok(())
    }

    /// Preset: strict — all checks enabled, strict mode on.
    fn strict_preset_json() -> serde_json::Value {
        serde_json::json!({
            "strict": true,
            "canonical": {
                "require": true,
                "absolute": true,
                "same_origin": true,
                "self_reference": true,
                "detect_clusters": true
            },
            "links": {
                "check_internal": true,
                "fail_on_broken": true,
                "forbid_query_params_internal": true,
                "check_fragments": true,
                "detect_orphan_pages": true,
                "check_mixed_content": true
            },
            "html_basics": {
                "lang_attr_required": true,
                "title_required": true,
                "meta_description_required": true,
                "viewport_required": true,
                "title_max_length": 60,
                "meta_description_max_length": 160
            },
            "headings": {
                "require_h1": true,
                "single_h1": true,
                "no_skip": true
            },
            "a11y": {
                "img_alt_required": true,
                "allow_decorative_images": true,
                "a_accessible_name_required": true,
                "button_name_required": true,
                "label_for_required": true,
                "warn_generic_link_text": true,
                "aria_hidden_focusable_check": true,
                "require_skip_link": true
            },
            "assets": {
                "check_broken_assets": true,
                "check_image_dimensions": true
            },
            "opengraph": {
                "require_og_title": true,
                "require_og_description": true,
                "require_og_image": true,
                "require_twitter_card": true
            },
            "structured_data": {
                "check_json_ld": true,
                "detect_duplicate_types": true
            },
            "hreflang": {
                "check_hreflang": true,
                "require_x_default": true,
                "require_self_reference": true,
                "require_reciprocal": true
            },
            "security": {
                "check_target_blank": true,
                "check_mixed_content": true,
                "warn_inline_scripts": true
            },
            "content_quality": {
                "detect_duplicate_titles": true,
                "detect_duplicate_descriptions": true,
                "detect_duplicate_h1": true,
                "detect_duplicate_pages": true
            },
            "sitemap": {
                "require": true,
                "canonical_must_be_in_sitemap": true,
                "forbid_noncanonical_in_sitemap": true,
                "entries_must_exist_in_dist": true
            },
            "robots_txt": {
                "require": true,
                "require_sitemap_link": true
            },
            "i18n_audit": {
                "enabled": true
            },
            "crawl_budget": {
                "enabled": true
            },
            "render_blocking": {
                "enabled": true
            },
            "privacy_security": {
                "enabled": true
            },
            "structured_data_graph": {
                "enabled": true
            }
        })
    }

    /// Preset: relaxed — core SEO only, lenient settings.
    fn relaxed_preset_json() -> serde_json::Value {
        serde_json::json!({
            "strict": false,
            "canonical": {
                "require": true,
                "absolute": true,
                "same_origin": true,
                "self_reference": false,
                "detect_clusters": false
            },
            "links": {
                "check_internal": true,
                "fail_on_broken": false,
                "forbid_query_params_internal": false,
                "check_fragments": false,
                "detect_orphan_pages": false,
                "check_mixed_content": true
            },
            "html_basics": {
                "lang_attr_required": true,
                "title_required": true,
                "meta_description_required": false,
                "viewport_required": true
            },
            "headings": {
                "require_h1": true,
                "single_h1": false,
                "no_skip": false
            },
            "a11y": {
                "img_alt_required": true,
                "allow_decorative_images": true,
                "a_accessible_name_required": true,
                "button_name_required": false,
                "label_for_required": false,
                "warn_generic_link_text": false,
                "aria_hidden_focusable_check": false,
                "require_skip_link": false
            },
            "security": {
                "check_target_blank": true,
                "check_mixed_content": true,
                "warn_inline_scripts": false
            }
        })
    }
}
