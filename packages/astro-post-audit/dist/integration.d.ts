import type { AstroIntegration } from 'astro';
/**
 * Inline rules config that mirrors the Rust config structure.
 * All sections and fields are optional — only set what you want to override.
 */
export interface RulesConfig {
    /** Site-level settings. */
    site?: {
        /** Base URL for canonical/sitemap checks. Also settable via `site` option or Astro's `site` config. */
        base_url?: string;
    };
    /** File filters — glob patterns to include or exclude pages from all checks. */
    filters?: {
        /** Only check files matching these glob patterns. */
        include?: string[];
        /** Skip files matching these glob patterns (e.g. `["404.html", "drafts/**"]`). */
        exclude?: string[];
    };
    /** URL normalization rules for internal link and canonical consistency. */
    url_normalization?: {
        /** Trailing slash policy. `"always"`: require trailing slash, `"never"`: forbid, `"ignore"`: no check. @default "always" */
        trailing_slash?: 'always' | 'never' | 'ignore';
        /** Whether `index.html` in URLs is allowed. `"forbid"`: warn on `/page/index.html` links, `"allow"`: permit them. @default "forbid" */
        index_html?: 'forbid' | 'allow';
    };
    /** Canonical `<link rel="canonical">` tag checks. */
    canonical?: {
        /** Every page must have a canonical tag. @default true */
        require?: boolean;
        /** Canonical URL must be absolute (not relative). @default true */
        absolute?: boolean;
        /** Canonical must point to the same origin as `site`. @default true */
        same_origin?: boolean;
        /** Canonical must be a self-reference (point to the page itself). @default false */
        self_reference?: boolean;
        /** Warn when multiple pages share the same canonical URL (cluster detection). @default true */
        detect_clusters?: boolean;
    };
    /** Robots meta tag checks. */
    robots_meta?: {
        /** Don't warn on pages with `noindex`. @default true */
        allow_noindex?: boolean;
        /** Treat any `noindex` page as an error. @default false */
        fail_if_noindex?: boolean;
    };
    /** Internal link consistency checks. */
    links?: {
        /** Check that internal links resolve to existing pages. @default true */
        check_internal?: boolean;
        /** Broken internal links are errors (not just warnings). @default true */
        fail_on_broken?: boolean;
        /** Warn on query parameters (`?foo=bar`) in internal links. @default true */
        forbid_query_params_internal?: boolean;
        /** Validate that `#fragment` targets exist in the linked page. @default false */
        check_fragments?: boolean;
        /** Warn about pages with no incoming internal links (orphan pages). @default false */
        detect_orphan_pages?: boolean;
        /** Warn on `http://` in internal links (mixed content). @default true */
        check_mixed_content?: boolean;
    };
    /** Sitemap cross-reference checks. */
    sitemap?: {
        /** `sitemap.xml` must exist in `dist/`. @default false */
        require?: boolean;
        /** Canonical URLs should appear in the sitemap. @default true */
        canonical_must_be_in_sitemap?: boolean;
        /** Sitemap must not contain non-canonical URLs. @default false */
        forbid_noncanonical_in_sitemap?: boolean;
        /** Every sitemap URL must correspond to a page in `dist/`. @default true */
        entries_must_exist_in_dist?: boolean;
    };
    /** `robots.txt` file checks. */
    robots_txt?: {
        /** `robots.txt` must exist. @default false */
        require?: boolean;
        /** `robots.txt` must contain a link to the sitemap. @default false */
        require_sitemap_link?: boolean;
    };
    /** Basic HTML structure checks. */
    html_basics?: {
        /** `<html lang="...">` attribute is required. @default true */
        lang_attr_required?: boolean;
        /** `<title>` tag is required and must be non-empty. @default true */
        title_required?: boolean;
        /** `<meta name="description">` is required. @default false */
        meta_description_required?: boolean;
        /** `<meta name="viewport">` is required. @default true */
        viewport_required?: boolean;
        /** Warn if `<title>` exceeds this character length. @default 60 */
        title_max_length?: number;
        /** Warn if meta description exceeds this character length. @default 160 */
        meta_description_max_length?: number;
    };
    /** Heading hierarchy checks. */
    headings?: {
        /** Page must have at least one `<h1>`. @default true */
        require_h1?: boolean;
        /** Only one `<h1>` per page. @default true */
        single_h1?: boolean;
        /** No heading level gaps (e.g. `<h2>` followed by `<h4>`). @default false */
        no_skip?: boolean;
    };
    /** Accessibility (a11y) heuristics — static checks, no layout computation. */
    a11y?: {
        /** `<img>` elements must have an `alt` attribute. @default true */
        img_alt_required?: boolean;
        /** Allow images with `role="presentation"` or `aria-hidden="true"` to skip `alt`. @default true */
        allow_decorative_images?: boolean;
        /** `<a>` elements must have an accessible name (text, `aria-label`, or `aria-labelledby`). @default true */
        a_accessible_name_required?: boolean;
        /** `<button>` elements must have an accessible name. @default true */
        button_name_required?: boolean;
        /** Form controls must have an associated `<label>`. @default true */
        label_for_required?: boolean;
        /** Warn on generic link text like "click here", "mehr", "weiter". @default true */
        warn_generic_link_text?: boolean;
        /** Warn if `aria-hidden="true"` is set on a focusable element. @default true */
        aria_hidden_focusable_check?: boolean;
        /** Require a skip navigation link (e.g. `<a href="#main-content">`). @default false */
        require_skip_link?: boolean;
    };
    /** Asset reference and size checks. */
    assets?: {
        /** Check that `<img>`, `<script>`, `<link>` references resolve to files in `dist/`. @default false */
        check_broken_assets?: boolean;
        /** Warn if `<img>` is missing `width`/`height` attributes (CLS prevention). @default false */
        check_image_dimensions?: boolean;
        /** Warn if any image file exceeds this size in KB. Off by default. */
        max_image_size_kb?: number;
        /** Warn if any JS file exceeds this size in KB. Off by default. */
        max_js_size_kb?: number;
        /** Warn if any CSS file exceeds this size in KB. Off by default. */
        max_css_size_kb?: number;
        /** Warn if asset filenames lack a cache-busting hash. @default false */
        require_hashed_filenames?: boolean;
    };
    /** Open Graph and Twitter Card meta tag checks. */
    opengraph?: {
        /** Require `og:title` meta tag. @default false */
        require_og_title?: boolean;
        /** Require `og:description` meta tag. @default false */
        require_og_description?: boolean;
        /** Require `og:image` meta tag. @default false */
        require_og_image?: boolean;
        /** Require `twitter:card` meta tag. @default false */
        require_twitter_card?: boolean;
    };
    /** Structured data (JSON-LD) validation. */
    structured_data?: {
        /** Validate JSON-LD syntax and semantics (`@context`, `@type`, required properties). @default false */
        check_json_ld?: boolean;
        /** Every page must contain at least one JSON-LD block. @default false */
        require_json_ld?: boolean;
        /** Warn if a page has multiple JSON-LD blocks with the same `@type`. @default false */
        detect_duplicate_types?: boolean;
    };
    /** Hreflang checks for multilingual sites. */
    hreflang?: {
        /** Enable hreflang link checks. @default false */
        check_hreflang?: boolean;
        /** Require an `x-default` hreflang entry. @default false */
        require_x_default?: boolean;
        /** Hreflang must include a self-referencing entry. @default false */
        require_self_reference?: boolean;
        /** Hreflang links must be reciprocal (A→B and B→A). @default false */
        require_reciprocal?: boolean;
    };
    /** Security heuristic checks. */
    security?: {
        /** Warn on `target="_blank"` without `rel="noopener"`. @default true */
        check_target_blank?: boolean;
        /** Warn on `http://` resource URLs (mixed content). @default true */
        check_mixed_content?: boolean;
        /** Warn on inline `<script>` tags. @default false */
        warn_inline_scripts?: boolean;
    };
    /** Duplicate content detection. */
    content_quality?: {
        /** Warn if multiple pages share the same `<title>`. @default false */
        detect_duplicate_titles?: boolean;
        /** Warn if multiple pages share the same meta description. @default false */
        detect_duplicate_descriptions?: boolean;
        /** Warn if multiple pages share the same `<h1>`. @default false */
        detect_duplicate_h1?: boolean;
        /** Warn if pages have identical content (by hash). @default false */
        detect_duplicate_pages?: boolean;
    };
    /**
     * Override severity per rule ID.
     * @example `{ "html/title-too-long": "off", "a11y/img-alt-missing": "error" }`
     */
    severity?: Record<string, 'error' | 'warning' | 'info' | 'off'>;
    /** External link checking (HEAD requests to verify URLs return 2xx). */
    external_links?: {
        /** Enable external link checking. @default false */
        enabled?: boolean;
        /** Timeout per request in milliseconds. @default 3000 */
        timeout_ms?: number;
        /** Maximum concurrent requests. @default 10 */
        max_concurrent?: number;
        /** Broken external links are errors (not just warnings). @default false */
        fail_on_broken?: boolean;
        /** Only check links to these domains (empty = all). */
        allow_domains?: string[];
        /** Skip links to these domains. */
        block_domains?: string[];
    };
}
export interface PostAuditOptions {
    /** Inline rules config — all check settings go here. */
    rules?: RulesConfig;
    /** Base URL (auto-detected from Astro's `site` config if not set). */
    site?: string;
    /** Treat warnings as errors. */
    strict?: boolean;
    /** Maximum number of errors before aborting. */
    maxErrors?: number;
    /** Show page properties overview instead of running checks. */
    pageOverview?: boolean;
    /** Write the JSON report to this file path (relative to project root). */
    output?: string;
    /** Disable the integration (useful for dev mode). */
    disable?: boolean;
    /** Throw an error when the audit finds issues (fails the build). Default: false */
    throwOnError?: boolean;
}
export default function postAudit(options?: PostAuditOptions): AstroIntegration;
//# sourceMappingURL=integration.d.ts.map