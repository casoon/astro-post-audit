import type { AstroIntegration } from 'astro';
/**
 * Inline rules config that mirrors the Rust rules.toml structure.
 * All sections and fields are optional — only set what you want to override.
 */
export interface RulesConfig {
    site?: {
        base_url?: string;
    };
    url_normalization?: {
        trailing_slash?: 'always' | 'never' | 'ignore';
        index_html?: 'forbid' | 'allow';
    };
    canonical?: {
        require?: boolean;
        absolute?: boolean;
        same_origin?: boolean;
        self_reference?: boolean;
    };
    robots_meta?: {
        allow_noindex?: boolean;
        fail_if_noindex?: boolean;
    };
    links?: {
        check_internal?: boolean;
        fail_on_broken?: boolean;
        forbid_query_params_internal?: boolean;
        check_fragments?: boolean;
        detect_orphan_pages?: boolean;
        check_mixed_content?: boolean;
    };
    sitemap?: {
        require?: boolean;
        canonical_must_be_in_sitemap?: boolean;
        forbid_noncanonical_in_sitemap?: boolean;
        entries_must_exist_in_dist?: boolean;
    };
    robots_txt?: {
        require?: boolean;
        require_sitemap_link?: boolean;
    };
    html_basics?: {
        lang_attr_required?: boolean;
        title_required?: boolean;
        meta_description_required?: boolean;
        viewport_required?: boolean;
        title_max_length?: number;
        meta_description_max_length?: number;
    };
    headings?: {
        require_h1?: boolean;
        single_h1?: boolean;
        no_skip?: boolean;
    };
    a11y?: {
        img_alt_required?: boolean;
        allow_decorative_images?: boolean;
        a_accessible_name_required?: boolean;
        button_name_required?: boolean;
        label_for_required?: boolean;
        warn_generic_link_text?: boolean;
        aria_hidden_focusable_check?: boolean;
        require_skip_link?: boolean;
    };
    assets?: {
        check_broken_assets?: boolean;
        check_image_dimensions?: boolean;
        max_image_size_kb?: number;
        max_js_size_kb?: number;
        max_css_size_kb?: number;
        require_hashed_filenames?: boolean;
    };
    opengraph?: {
        require_og_title?: boolean;
        require_og_description?: boolean;
        require_og_image?: boolean;
        require_twitter_card?: boolean;
    };
    structured_data?: {
        check_json_ld?: boolean;
        require_json_ld?: boolean;
    };
    hreflang?: {
        check_hreflang?: boolean;
        require_x_default?: boolean;
        require_self_reference?: boolean;
        require_reciprocal?: boolean;
    };
    security?: {
        check_target_blank?: boolean;
        check_mixed_content?: boolean;
        warn_inline_scripts?: boolean;
    };
    content_quality?: {
        detect_duplicate_titles?: boolean;
        detect_duplicate_descriptions?: boolean;
        detect_duplicate_h1?: boolean;
        detect_duplicate_pages?: boolean;
    };
    /** Custom severity overrides per rule ID. Maps rule IDs to 'error' | 'warning' | 'info' | 'off'. */
    severity?: Record<string, 'error' | 'warning' | 'info' | 'off'>;
    /** @deprecated Not yet implemented — will be ignored. */
    external_links?: {
        enabled?: boolean;
        timeout_ms?: number;
        max_concurrent?: number;
        fail_on_broken?: boolean;
        allow_domains?: string[];
        block_domains?: string[];
    };
}
export interface PostAuditOptions {
    /** Path to rules.toml config file. Mutually exclusive with `rules`. */
    config?: string;
    /** Inline rules config (generates a temporary rules.toml). Mutually exclusive with `config`. */
    rules?: RulesConfig;
    /** Base URL (auto-detected from Astro's `site` config if not set) */
    site?: string;
    /** Treat warnings as errors */
    strict?: boolean;
    /** Output format */
    format?: 'text' | 'json';
    /** Maximum number of errors before aborting */
    maxErrors?: number;
    /** Glob patterns to include */
    include?: string[];
    /** Glob patterns to exclude */
    exclude?: string[];
    /** Skip sitemap.xml checks */
    noSitemapCheck?: boolean;
    /** Enable asset reference checking */
    checkAssets?: boolean;
    /** Enable structured data (JSON-LD) validation */
    checkStructuredData?: boolean;
    /** Enable security heuristic checks */
    checkSecurity?: boolean;
    /** Enable duplicate content detection */
    checkDuplicates?: boolean;
    /** Show page properties overview instead of running checks */
    pageOverview?: boolean;
    /** Disable the integration (useful for dev mode) */
    disable?: boolean;
    /** Throw an AstroError when the audit finds errors (fails the build). Default: false */
    throwOnError?: boolean;
}
/**
 * Serialize a RulesConfig object to TOML format.
 * @internal Exported for testing only.
 */
export declare function rulesToToml(rules: RulesConfig): string;
export default function postAudit(options?: PostAuditOptions): AstroIntegration;
//# sourceMappingURL=integration.d.ts.map