import type { AstroIntegration } from "astro";
import { execFileSync } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
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
        trailing_slash?: "always" | "never" | "ignore";
        /** Whether `index.html` in URLs is allowed. `"forbid"`: warn on `/page/index.html` links, `"allow"`: permit them. @default "forbid" */
        index_html?: "forbid" | "allow";
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
        /** Warn when a page's URL nesting depth (path segments) exceeds this value. Disabled when unset. */
        max_url_depth?: number;
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
        /** Error if `User-agent: *` with `Disallow: /` blocks all crawlers. @default true */
        check_disallow_all?: boolean;
        /** Warn if `Crawl-delay` exceeds this value in seconds (0 = disabled). @default 10 */
        max_crawl_delay?: number;
        /** Warn if AI citation bots (GPTBot, ClaudeBot, PerplexityBot) are blocked. @default false */
        ai_bot_policy?: boolean;
        /** Error when a page is `Disallow`'d in robots.txt yet also has a `noindex` meta tag. @default false */
        check_noindex_contradiction?: boolean;
        /** Warn when a URL listed in the sitemap is blocked by robots.txt. @default false */
        check_sitemap_blocked?: boolean;
    };
    /** Image HTML attribute checks for CLS prevention and responsive image best practices. */
    images?: {
        /** Error if `<img>` is missing both `width` and `height` (causes Cumulative Layout Shift). @default true */
        check_missing_dimensions?: boolean;
        /** Warn if `<img>` beyond the first on a page has no `loading` attribute. @default true */
        warn_missing_lazy?: boolean;
        /** Info if `<img>` has no `srcset` (no responsive image markup). @default true */
        info_missing_srcset?: boolean;
        /** Info if `<img>` uses a legacy format (`.jpg`, `.png`, `.gif`) — suggests WebP/AVIF. @default false */
        format_hints?: boolean;
    };
    /** AI visibility scoring — checks static signals that influence AI search citation probability. @default false */
    ai_visibility?: {
        /** Enable AI visibility checks. @default false */
        enabled?: boolean;
    };
    /** UX heuristic checks — CTA clarity, trust signals, cognitive load. @default false */
    ux_heuristics?: {
        /** Enable UX heuristics module. @default false */
        enabled?: boolean;
        /** Warn if a page exceeds this many links (cognitive load). @default 80 */
        max_links_per_page?: number;
        /** Warn if a page has fewer than this many CTA-like elements. @default 1 */
        min_cta_per_page?: number;
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
        /** Check for semantic landmark elements (`<main>`, `<nav>`, `<header>`, `<footer>`). @default true */
        check_landmarks?: boolean;
        /** Detect duplicate `id` attributes on a page. Duplicate ids break ARIA references. @default true */
        check_duplicate_ids?: boolean;
        /** Validate ARIA `role` values against the WAI-ARIA spec. @default true */
        check_aria_roles?: boolean;
        /** Flag low-quality `alt` text (file names, placeholder words like "image"/"logo", too short). @default true */
        check_alt_quality?: boolean;
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
        /** Require `og:type` meta tag (e.g. `"website"`, `"article"`). @default false */
        require_og_type?: boolean;
        /** Require `og:url` canonical property. @default false */
        require_og_url?: boolean;
        /** Error if `og:image` is a relative URL (must be absolute for social sharing). @default true */
        og_image_absolute_url?: boolean;
        /** Require `twitter:image` meta tag. @default false */
        require_twitter_image?: boolean;
        /** Validate `twitter:card` value against the allowed set (`summary`, `summary_large_image`, `app`, `player`). @default true */
        twitter_card_valid_values?: boolean;
        /** Warn when `og:title` and `<title>` differ significantly in length. @default false */
        og_title_consistency?: boolean;
        /** Verify that a local `og:image` (own domain or relative) exists in `dist/`. @default false */
        check_image_exists?: boolean;
        /** Warn if a local `og:image` is below the recommended 1200x630 dimensions. @default false */
        check_image_dimensions?: boolean;
        /** Warn if a local `og:image` file exceeds this size in KB. Disabled when unset. */
        og_image_max_size_kb?: number;
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
        /** Warn when an internal hreflang target does not exist in the build. @default false */
        require_target_exists?: boolean;
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
     * @example `{ "html/title-too-long": "off", "a11y/img-alt": "error" }`
     */
    severity?: Record<string, "error" | "warning" | "info" | "off">;
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
    /** I18n consistency audit across route locale, lang, hreflang, and canonical signals. */
    i18n_audit?: {
        /** Enable i18n consistency checks in dist output. @default false */
        enabled?: boolean;
    };
    /** Crawl budget audit for URL variants, indexability mismatches, and duplicate clusters. */
    crawl_budget?: {
        /** Enable crawl budget checks in dist output. @default false */
        enabled?: boolean;
    };
    /** Static render-blocking audit for critical resources and connection hints. */
    render_blocking?: {
        /** Enable render-blocking checks in dist output. @default false */
        enabled?: boolean;
    };
    /** Static privacy and security posture audit (third-party, SRI, CSP readiness, consent indicators). */
    privacy_security?: {
        /** Enable privacy/security checks in dist output. @default false */
        enabled?: boolean;
        /** Enable GDPR/DSGVO third-party transfer checks (Google Fonts, YouTube, Maps, public CDNs, external images). @default false */
        gdpr?: boolean;
    };
    /** Cross-page structured-data graph consistency checks. */
    structured_data_graph?: {
        /** Enable cross-page JSON-LD consistency checks in dist output. @default false */
        enabled?: boolean;
    };
    /** Static meta-refresh redirect analysis (links to redirects, chains, loops). */
    redirects?: {
        /** Enable redirect analysis in dist output. @default false */
        enabled?: boolean;
    };
    /** Client-side JavaScript bloat detection per route. */
    js_bloat?: {
        /** Enable JS bloat detection. @default false */
        enabled?: boolean;
        /** Warn when a route's total local JS exceeds this size in KB. @default 100 */
        max_kb?: number;
    };
    /**
     * Cross-check `src/content/` collection items against generated pages.
     * Requires the project root, which the integration passes automatically.
     */
    content_sync?: {
        /** Warn about content items with no corresponding build page. @default false */
        enabled?: boolean;
    };
    /** Native HTML5 syntax validation using the html5ever tokenizer (offline). */
    html_validation?: {
        /** Report HTML5 parse/syntax errors. @default false */
        enabled?: boolean;
        /** Maximum distinct syntax errors reported per page. @default 20 */
        max_per_page?: number;
    };
}
export type GroupValue = boolean | "warn";
export interface GroupsConfig {
    /** Enable SEO-focused rules (canonical, meta description, OG tags, structured data). */
    seo?: GroupValue;
    /** Enable accessibility rules (a11y, headings, lang attribute). */
    a11y?: GroupValue;
    /** Enable link-integrity rules (broken links, fragments, orphan pages). */
    links?: GroupValue;
    /** Enable performance rules (image dimensions, hashed filenames, render blocking). */
    performance?: GroupValue;
    /** Enable privacy/security rules (third-party domains, SRI, inline scripts). */
    privacy?: GroupValue;
}
export interface ReportsConfig {
    /** Write a JSON report to this file path (relative to project root). */
    json?: string;
    /** Write a Markdown summary report to this file path (relative to project root). */
    markdown?: string;
    /** Write a SARIF 2.1.0 report to this file path (relative to project root). For use with GitHub Code Scanning. */
    sarif?: string;
}
export interface GoLiveConfig {
    /** Enable go-live production gate checks. @default false */
    enabled?: boolean;
    /**
     * Expected production origin. Auto-detected from Astro's `site` config if not set.
     * Only set this when the go-live target intentionally differs from Astro `site`.
     */
    expectedSite?: string;
    /** Domains that must not appear in canonical URLs, sitemaps, OG tags, or absolute links. */
    forbiddenDomains?: string[];
}
export interface PostAuditOptions {
    /** Inline rules config — all check settings go here. */
    rules?: RulesConfig;
    /** Preset to apply before user overrides. `"strict"` enables all checks, `"relaxed"` is lenient. */
    preset?: "strict" | "relaxed" | "seo" | "accessibility" | "performance" | "production" | "standard";
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
    /** Write a Markdown summary report to this file path (relative to project root). */
    outputMarkdown?: string;
    /**
     * Write one or more report files. Supports `json`, `markdown`, and `sarif` (SARIF 2.1.0) formats.
     * Multiple formats can be active simultaneously. Takes precedence over `output`/`outputMarkdown` when both are set.
     */
    reports?: ReportsConfig;
    /** Heuristic hints for source file locations in Content Collections / MDX projects. */
    hints?: {
        /** Show likely source file paths next to dist/ findings. Heuristic — may not always match. @default false */
        sourceFiles?: boolean;
    };
    /** Print per-check timing benchmarks in the output. */
    benchmark?: boolean;
    /**
     * Show a live progress bar on stderr while checks run.
     * Defaults to auto: on in an interactive terminal, silent in CI.
     * Set `true`/`false` to force it.
     */
    progress?: boolean;
    /** Disable the integration (useful for dev mode). */
    disable?: boolean;
    /** Throw an error when the audit finds issues (fails the build). Default: false */
    throwOnError?: boolean;
    /**
     * Path to a baseline file (relative to project root). When set, only findings that are
     * *new* since the baseline was written will be reported. If the file does not exist,
     * a warning is logged and the audit runs normally.
     */
    baseline?: string;
    /**
     * Write current findings as the new baseline and exit with code 0.
     * Use this once to adopt the plugin on a site with existing issues.
     */
    writeBaseline?: boolean;
    /**
     * Build fail strategy. `'errors'`: fail on errors only (default when `throwOnError` is true).
     * `'warnings'`: fail on any finding (implies `strict`). `'never'`: never fail the build.
     */
    failOn?: "never" | "errors" | "warnings";
    /** Fail the build if the warning count exceeds this number. */
    maxWarnings?: number;
    /** Shorthand rule groups. `true` enables the group, `"warn"` enables but downgrades all findings to warnings. */
    groups?: GroupsConfig;
    /**
     * Production readiness gate. Catches staging/dev leftovers before a build goes live.
     * Use `process.env.DEPLOY_CONTEXT === "production"` to enable only in production.
     * @example
     * goLive: { enabled: process.env.DEPLOY_CONTEXT === "production", forbiddenDomains: ["staging.example.com"] }
     */
    goLive?: GoLiveConfig;
    /**
     * Enable AI visibility scoring. Checks static signals (word count, schema, OG tags, semantic HTML)
     * that influence how AI search systems (Perplexity, ChatGPT Search, Claude) cite your content.
     * Pass `true` to enable with defaults, or an object for fine-grained control.
     * @default false
     */
    aiVisibility?: boolean;
    /**
     * Enable UX heuristic checks. Evaluates CTA clarity, trust signals, and cognitive load.
     * Pass `true` to enable with defaults, or an object for fine-grained control.
     * @default false
     */
    uxHeuristics?: boolean | {
        maxLinksPerPage?: number;
        minCtaPerPage?: number;
    };
}
interface RuntimeDeps {
    execFileSync: typeof execFileSync;
    existsSync: typeof existsSync;
    writeFileSync: typeof writeFileSync;
}
export default function postAudit(options?: PostAuditOptions, deps?: RuntimeDeps): AstroIntegration;
export {};
//# sourceMappingURL=integration.d.ts.map