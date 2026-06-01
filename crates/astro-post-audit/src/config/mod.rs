use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Strict,
    Relaxed,
    Seo,
    Accessibility,
    Performance,
    Production,
    Standard,
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
    /// Maximum number of warnings before returning exit code 1.
    pub max_warnings: Option<usize>,
    /// Show page properties overview instead of running checks.
    pub page_overview: bool,
    /// Output format: "text" (default) or "json".
    pub format: Option<String>,
    /// Print per-check timing benchmarks.
    pub benchmark: bool,
    /// Show a live progress bar on stderr. None = auto (on when stderr is a TTY and format is text).
    pub progress: Option<bool>,
    /// Show each check as a line on stderr as it completes, with findings count and timing.
    pub progress_verbose: bool,
    /// Emit verbose diagnostics on stderr: resolved config, discovery stats, per-check counts.
    pub debug: bool,
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
    pub redirects: RedirectsConfig,
    pub js_bloat: JsBloatConfig,
    pub content_sync: ContentSyncConfig,
    pub html_validation: HtmlValidationConfig,
    pub images: ImagesConfig,
    pub ai_visibility: AiVisibilityConfig,
    pub ux_heuristics: UxHeuristicsConfig,
    pub severity: SeverityConfig,
    pub hints: HintsConfig,
    /// Project root directory, used for source-file hint resolution.
    pub project_root: Option<String>,
    /// Baseline file path. Existing findings in this file are suppressed.
    pub baseline: Option<String>,
    /// Write the current findings to the baseline file and exit successfully.
    pub write_baseline: bool,
    /// Additional report formats to write to disk in a single audit run.
    pub extra_reports: Vec<ExtraReport>,
    pub go_live: GoLiveConfig,
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
    /// Warn when a page's URL depth (path segments) exceeds this value. None = disabled.
    pub max_url_depth: Option<usize>,
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
    /// Check for semantic landmark elements (main, nav, header, footer). @default true
    pub check_landmarks: bool,
    /// Detect duplicate id attributes on a page. @default true
    pub check_duplicate_ids: bool,
    /// Validate ARIA role values and required role attributes. @default true
    pub check_aria_roles: bool,
    /// Flag low-quality alt text (filename, placeholder words, too short). @default true
    pub check_alt_quality: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OpenGraphConfig {
    pub require_og_title: bool,
    pub require_og_description: bool,
    pub require_og_image: bool,
    pub require_twitter_card: bool,
    /// Require og:type meta tag (e.g. "website", "article"). @default false
    pub require_og_type: bool,
    /// Require og:url canonical property. @default false
    pub require_og_url: bool,
    /// Error if og:image value is a relative URL. @default true
    pub og_image_absolute_url: bool,
    /// Require twitter:image meta tag. @default false
    pub require_twitter_image: bool,
    /// Validate twitter:card value against allowed set. @default true
    pub twitter_card_valid_values: bool,
    /// Warn when og:title and <title> differ by more than 50% in length. @default false
    pub og_title_consistency: bool,
    /// Verify that a local og:image (own domain / relative) exists in dist. @default false
    pub check_image_exists: bool,
    /// Warn if a local og:image does not match recommended dimensions (1200x630). @default false
    pub check_image_dimensions: bool,
    /// Warn if a local og:image file exceeds this size in KB. None = disabled.
    pub og_image_max_size_kb: Option<u64>,
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
    /// Warn when an internal hreflang target does not exist in the build. @default false
    pub require_target_exists: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RobotsTxtConfig {
    pub require: bool,
    pub require_sitemap_link: bool,
    /// Error if User-agent: * with Disallow: / blocks all crawlers. @default true
    pub check_disallow_all: bool,
    /// Warn if Crawl-delay exceeds this value in seconds. 0 = disabled. @default 10
    pub max_crawl_delay: u32,
    /// Warn if AI citation bots (GPTBot, ClaudeBot, PerplexityBot) are blocked. @default false
    pub ai_bot_policy: bool,
    /// Error when a page is Disallow'd in robots.txt yet also carries a noindex meta. @default false
    pub check_noindex_contradiction: bool,
    /// Warn when a sitemap URL is blocked by robots.txt. @default false
    pub check_sitemap_blocked: bool,
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
    /// Enable GDPR/DSGVO third-party transfer checks (Google Fonts, YouTube, Maps, CDNs, external images). @default false
    pub gdpr: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct StructuredDataGraphConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RedirectsConfig {
    /// Analyze static meta-refresh redirects (chains, loops, links to redirect pages). @default false
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct JsBloatConfig {
    /// Enable client-side JS bloat detection per route. @default false
    pub enabled: bool,
    /// Warn when a route's total local JS exceeds this size in KB. @default 100
    pub max_kb: u64,
}

impl Default for JsBloatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_kb: 100,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ContentSyncConfig {
    /// Warn about content collection items (src/content) with no generated page. @default false
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HtmlValidationConfig {
    /// Report HTML5 parse/syntax errors collected by the html5ever tokenizer. @default false
    pub enabled: bool,
    /// Maximum distinct syntax errors reported per page. @default 20
    pub max_per_page: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HintsConfig {
    /// Show heuristic source-file hints in output (e.g. "Likely source: src/content/blog/post.mdx").
    pub source_files: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ExtraReport {
    /// Output format: "json", "markdown", or "sarif"
    pub format: String,
    /// Absolute path to write the report to
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ImagesConfig {
    /// Error if <img> is missing both width and height (causes CLS). @default true
    pub check_missing_dimensions: bool,
    /// Warn if <img> beyond the first on a page lacks loading="lazy". @default true
    pub warn_missing_lazy: bool,
    /// Info if <img> has no srcset (no responsive image markup). @default true
    pub info_missing_srcset: bool,
    /// Info if <img> src uses legacy format (.jpg/.jpeg/.png/.gif). @default false
    pub format_hints: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AiVisibilityConfig {
    /// Enable AI visibility scoring module. @default false
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UxHeuristicsConfig {
    /// Enable UX heuristics module. @default false
    pub enabled: bool,
    /// Warn if a page has more than this many links (cognitive load). @default 80
    pub max_links_per_page: usize,
    /// Warn if a page has fewer than this many CTA-like elements. @default 1
    pub min_cta_per_page: usize,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct GoLiveConfig {
    pub enabled: bool,
    pub expected_site: Option<String>,
    pub forbidden_domains: Vec<String>,
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
            max_url_depth: None,
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
            check_landmarks: true,
            check_duplicate_ids: true,
            check_aria_roles: true,
            check_alt_quality: true,
        }
    }
}

impl Default for OpenGraphConfig {
    fn default() -> Self {
        Self {
            require_og_title: false,
            require_og_description: false,
            require_og_image: false,
            require_twitter_card: false,
            require_og_type: false,
            require_og_url: false,
            og_image_absolute_url: true,
            require_twitter_image: false,
            twitter_card_valid_values: true,
            og_title_consistency: false,
            check_image_exists: false,
            check_image_dimensions: false,
            og_image_max_size_kb: None,
        }
    }
}

impl Default for ImagesConfig {
    fn default() -> Self {
        Self {
            check_missing_dimensions: true,
            warn_missing_lazy: true,
            info_missing_srcset: true,
            format_hints: false,
        }
    }
}

impl Default for UxHeuristicsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_links_per_page: 80,
            min_cta_per_page: 1,
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

impl Default for RobotsTxtConfig {
    fn default() -> Self {
        Self {
            require: false,
            require_sitemap_link: false,
            check_disallow_all: true,
            max_crawl_delay: 10,
            ai_bot_policy: false,
            check_noindex_contradiction: false,
            check_sitemap_blocked: false,
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
                "seo" => Self::seo_preset_json(),
                "accessibility" => Self::accessibility_preset_json(),
                "performance" => Self::performance_preset_json(),
                "production" => Self::production_preset_json(),
                "standard" => Self::standard_preset_json(),
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
        if self.write_baseline && self.baseline.is_none() {
            anyhow::bail!("baseline must be set when write_baseline is true");
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
                "require_skip_link": true,
                "check_landmarks": true,
                "check_duplicate_ids": true,
                "check_aria_roles": true
            },
            "assets": {
                "check_broken_assets": true,
                "check_image_dimensions": true
            },
            "opengraph": {
                "require_og_title": true,
                "require_og_description": true,
                "require_og_image": true,
                "require_twitter_card": true,
                "require_og_type": true,
                "require_og_url": true,
                "og_image_absolute_url": true,
                "twitter_card_valid_values": true
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

    /// Preset: SEO-focused checks.
    fn seo_preset_json() -> serde_json::Value {
        serde_json::json!({
            "canonical": {
                "require": true,
                "absolute": true,
                "same_origin": true,
                "self_reference": true,
                "detect_clusters": true
            },
            "html_basics": {
                "lang_attr_required": true,
                "title_required": true,
                "meta_description_required": true,
                "viewport_required": true,
                "title_max_length": 60,
                "meta_description_max_length": 160
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
            "sitemap": {
                "require": true,
                "canonical_must_be_in_sitemap": true,
                "entries_must_exist_in_dist": true
            }
        })
    }

    /// Preset: accessibility-focused checks.
    fn accessibility_preset_json() -> serde_json::Value {
        serde_json::json!({
            "html_basics": {
                "lang_attr_required": true,
                "title_required": true,
                "viewport_required": true
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
            "links": {
                "check_fragments": true
            }
        })
    }

    /// Preset: static performance checks.
    fn performance_preset_json() -> serde_json::Value {
        serde_json::json!({
            "assets": {
                "check_broken_assets": true,
                "check_image_dimensions": true,
                "require_hashed_filenames": true
            },
            "render_blocking": {
                "enabled": true
            }
        })
    }

    /// Preset: production gate. Equivalent to strict.
    fn production_preset_json() -> serde_json::Value {
        Self::strict_preset_json()
    }

    /// Preset: standard — comprehensive quality checks without aggressive extras.
    /// Covers SEO, a11y, hreflang, content quality, assets and structured data.
    /// Does NOT enable orphan-page detection, inline-script warnings, CSP readiness,
    /// or strict mode (warnings remain warnings).
    fn standard_preset_json() -> serde_json::Value {
        serde_json::json!({
            "canonical": {
                "self_reference": true
            },
            "headings": {
                "no_skip": true
            },
            "html_basics": {
                "meta_description_required": true
            },
            "opengraph": {
                "require_og_title": true,
                "require_og_description": true,
                "require_og_image": true,
                "og_image_absolute_url": true,
                "twitter_card_valid_values": true
            },
            "a11y": {
                "require_skip_link": true,
                "img_alt_required": true,
                "button_name_required": true,
                "label_for_required": true,
                "check_landmarks": true,
                "check_duplicate_ids": true,
                "check_aria_roles": true
            },
            "links": {
                "check_fragments": true
            },
            "sitemap": {
                "require": true,
                "canonical_must_be_in_sitemap": true,
                "entries_must_exist_in_dist": true
            },
            "security": {
                "check_target_blank": true
            },
            "hreflang": {
                "check_hreflang": true,
                "require_x_default": true,
                "require_self_reference": true,
                "require_reciprocal": true
            },
            "assets": {
                "check_broken_assets": true
            },
            "structured_data": {
                "check_json_ld": true
            },
            "content_quality": {
                "detect_duplicate_titles": true,
                "detect_duplicate_descriptions": true,
                "detect_duplicate_h1": true
            }
        })
    }
}
