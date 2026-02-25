use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub site: SiteConfig,
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
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SiteConfig {
    pub base_url: Option<String>,
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
    pub check_assets: bool,
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
            check_assets: false,
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
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
