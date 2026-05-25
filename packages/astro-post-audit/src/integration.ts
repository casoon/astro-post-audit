import type { AstroIntegration } from "astro";
import { execFileSync } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

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
  };
  /** Cross-page structured-data graph consistency checks. */
  structured_data_graph?: {
    /** Enable cross-page JSON-LD consistency checks in dist output. @default false */
    enabled?: boolean;
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
  uxHeuristics?: boolean | { maxLinksPerPage?: number; minCtaPerPage?: number };
}

interface RuntimeDeps {
  execFileSync: typeof execFileSync;
  existsSync: typeof existsSync;
  writeFileSync: typeof writeFileSync;
}

const defaultDeps: RuntimeDeps = {
  execFileSync,
  existsSync,
  writeFileSync,
};

function resolveBinaryPath(exists: RuntimeDeps["existsSync"]): string | null {
  const binDir = join(dirname(fileURLToPath(import.meta.url)), "..", "bin");
  const binaryName =
    process.platform === "win32" ? "astro-post-audit.exe" : "astro-post-audit";
  const binaryPath = join(binDir, binaryName);
  return exists(binaryPath) ? binaryPath : null;
}

function supportsConfigStdin(
  binaryPath: string,
  run: RuntimeDeps["execFileSync"],
): boolean {
  try {
    const help = run(binaryPath, ["--help"], {
      stdio: ["ignore", "pipe", "ignore"],
      encoding: "utf-8",
    });
    return typeof help === "string" && help.includes("--config-stdin");
  } catch {
    return false;
  }
}

function logJsonSummary(
  json: string,
  logger: { info: (msg: string) => void; warn: (msg: string) => void },
): void {
  try {
    const report = JSON.parse(json) as {
      summary?: {
        errors?: number;
        warnings?: number;
        info?: number;
        files_checked?: number;
      };
    };
    const s = report.summary;
    if (!s) return;
    const parts: string[] = [];
    if (s.errors) parts.push(`${s.errors} error${s.errors === 1 ? "" : "s"}`);
    if (s.warnings)
      parts.push(`${s.warnings} warning${s.warnings === 1 ? "" : "s"}`);
    if (s.info) parts.push(`${s.info} info`);
    const filesMsg =
      s.files_checked !== undefined ? ` (${s.files_checked} files checked)` : "";
    if (parts.length === 0) {
      logger.info(`Audit passed${filesMsg}.`);
    } else {
      logger.warn(`Audit: ${parts.join(", ")}${filesMsg}.`);
    }
  } catch {
    // non-JSON output, skip summary
  }
}

const GROUP_DEFS: Record<
  keyof GroupsConfig,
  { rules: Partial<RulesConfig>; ruleIds: string[] }
> = {
  seo: {
    rules: {
      html_basics: { meta_description_required: true },
      opengraph: {
        require_og_title: true,
        require_og_description: true,
        require_og_image: true,
      },
      structured_data: { check_json_ld: true },
    },
    ruleIds: [
      "canonical/missing", "canonical/multiple", "canonical/empty",
      "canonical/not-absolute", "canonical/cross-origin", "canonical/not-self",
      "canonical/target-missing", "canonical/cluster", "robots/noindex",
      "sitemap/missing", "sitemap/canonical-missing", "sitemap/entry-not-in-dist",
      "sitemap/non-canonical-entry", "opengraph/title-missing",
      "opengraph/description-missing", "opengraph/image-missing",
      "html/title-missing", "html/meta-description-missing",
      "html/meta-description-too-long", "structured-data/missing",
      "structured-data/invalid-json",
    ],
  },
  a11y: {
    rules: {
      a11y: { require_skip_link: true },
      headings: { no_skip: true },
    },
    ruleIds: [
      "a11y/img-alt", "a11y/link-name", "a11y/generic-link-text",
      "a11y/button-name", "a11y/form-label", "a11y/skip-link",
      "a11y/aria-hidden-focusable", "headings/no-h1", "headings/multiple-h1",
      "headings/skip-level", "html/lang-missing",
    ],
  },
  links: {
    rules: {
      links: { detect_orphan_pages: true, check_fragments: true },
    },
    ruleIds: [
      "links/broken", "links/broken-fragment", "links/query-params",
      "links/mixed-content", "links/orphan-page", "external-links/broken",
    ],
  },
  performance: {
    rules: {
      assets: { check_image_dimensions: true, require_hashed_filenames: true },
      render_blocking: { enabled: true },
    },
    ruleIds: [
      "assets/img-dimensions", "assets/unhashed-filename", "assets/large-image",
      "assets/large-js", "assets/large-css", "render-blocking/sync-head-scripts",
      "render-blocking/missing-style-preload", "render-blocking/missing-preconnect",
    ],
  },
  privacy: {
    rules: {
      privacy_security: { enabled: true },
      security: { warn_inline_scripts: true },
    },
    ruleIds: [
      "privacy-security/third-party-domains", "privacy-security/missing-sri-script",
      "privacy-security/missing-sri-stylesheet",
      "privacy-security/csp-readiness-inline-script",
      "privacy-security/missing-consent-indicator",
      "security/target-blank-noopener", "security/inline-scripts",
      "security/mixed-content",
    ],
  },
};

function deepMerge(
  base: Record<string, unknown>,
  override: Record<string, unknown>,
): Record<string, unknown> {
  const result = { ...base };
  for (const [key, val] of Object.entries(override)) {
    if (
      val !== null &&
      typeof val === "object" &&
      !Array.isArray(val) &&
      typeof result[key] === "object" &&
      result[key] !== null
    ) {
      result[key] = deepMerge(
        result[key] as Record<string, unknown>,
        val as Record<string, unknown>,
      );
    } else {
      result[key] = val;
    }
  }
  return result;
}

function expandGroups(
  groups: GroupsConfig,
  userRules: RulesConfig,
): RulesConfig {
  let accumulated: Record<string, unknown> = {};
  const warnOverrides: Record<string, "warning"> = {};

  for (const [name, value] of Object.entries(groups) as [
    keyof GroupsConfig,
    GroupValue | undefined,
  ][]) {
    if (!value) continue;
    const def = GROUP_DEFS[name];
    accumulated = deepMerge(accumulated, def.rules as Record<string, unknown>);
    if (value === "warn") {
      for (const id of def.ruleIds) {
        warnOverrides[id] = "warning";
      }
    }
  }

  const merged = deepMerge(
    accumulated,
    userRules as Record<string, unknown>,
  ) as RulesConfig;

  if (Object.keys(warnOverrides).length > 0) {
    merged.severity = { ...warnOverrides, ...userRules.severity };
  }

  return merged;
}

export default function postAudit(
  options: PostAuditOptions = {},
  deps: RuntimeDeps = defaultDeps,
): AstroIntegration {
  let siteUrl: string | undefined;
  let rootDir: string | undefined;
  let astroTrailingSlash: "always" | "never" | "ignore" | undefined;
  let astroOutput: string | undefined;

  return {
    name: "astro-post-audit",
    hooks: {
      "astro:config:done": ({ config }) => {
        siteUrl = config.site?.toString();
        rootDir = fileURLToPath(config.root);
        // Bridge Astro's trailingSlash config automatically
        if (config.trailingSlash) {
          astroTrailingSlash = config.trailingSlash;
        }
        astroOutput = config.output;
      },

      "astro:build:done": ({ dir, logger }) => {
        if (
          options.disable ||
          process.env.SKIP_AUDIT === "1" ||
          process.env.SKIP_AUDIT === "true"
        ) {
          if (process.env.SKIP_AUDIT) {
            logger.info("Audit skipped (SKIP_AUDIT is set).");
          }
          return;
        }

        const shouldFail =
          options.throwOnError === true ||
          (options.failOn !== undefined && options.failOn !== "never");

        const resolvedRules: RulesConfig = options.groups
          ? expandGroups(options.groups, options.rules ?? {})
          : (options.rules ?? {});

        // Validate that rules is a non-empty object if provided
        if (
          options.rules &&
          typeof options.rules === "object" &&
          Object.keys(options.rules).length === 0
        ) {
          logger.warn(
            'astro-post-audit: "rules" is an empty object — using default config.',
          );
        }

        // Warn: `site` missing but canonical/sitemap checks active
        const effectiveSite = options.site ?? siteUrl;
        if (!effectiveSite) {
          const canonicalActive = resolvedRules.canonical?.require !== false;
          const sitemapActive =
            resolvedRules.sitemap?.canonical_must_be_in_sitemap !== false;
          if (canonicalActive || sitemapActive) {
            logger.warn(
              "`site` is not set in astro.config.mjs. Canonical and sitemap URL checks will be limited — set `site` to enable full URL verification.",
            );
          }
        }

        const binaryPath = resolveBinaryPath(deps.existsSync);
        if (!binaryPath) {
          const msg =
            'astro-post-audit binary not found. Run "npm rebuild @casoon/astro-post-audit".';
          if (shouldFail) throw new Error(msg);
          logger.warn(msg);
          return;
        }
        if (!supportsConfigStdin(binaryPath, deps.execFileSync)) {
          const msg =
            'astro-post-audit binary is outdated and does not support --config-stdin. Run "npm rebuild @casoon/astro-post-audit".';
          if (shouldFail) throw new Error(msg);
          logger.error(msg);
          return;
        }

        let distPath = fileURLToPath(dir);

        // For SSR/hybrid builds, static HTML lives under dist/client/ (adapter convention)
        if (astroOutput === "server" || astroOutput === "hybrid") {
          const clientPath = join(distPath, "client");
          if (deps.existsSync(clientPath)) {
            distPath = clientPath;
            logger.info(
              "SSR/hybrid build detected: auditing static output in dist/client/",
            );
          } else {
            logger.info(
              "SSR/hybrid build detected: no dist/client/ found, auditing dist/ directly.",
            );
          }
        }

        const args: string[] = [distPath, "--config-stdin"];

        // Info: sitemap checks active but no sitemap.xml in dist
        const sitemapChecksActive =
          resolvedRules.sitemap?.canonical_must_be_in_sitemap !== false ||
          resolvedRules.sitemap?.entries_must_exist_in_dist !== false;
        if (
          sitemapChecksActive &&
          !deps.existsSync(join(distPath, "sitemap.xml")) &&
          !deps.existsSync(join(distPath, "sitemap-index.xml"))
        ) {
          logger.info(
            "No sitemap.xml found in dist/. Sitemap checks are limited. Add @astrojs/sitemap or @casoon/astro-sitemap to generate a sitemap.",
          );
        }

        // Build the full JSON config for the Rust binary
        const site = effectiveSite;
        const stdinConfig: Record<string, unknown> = {
          ...resolvedRules,
        };
        if (site) stdinConfig.site = { base_url: site };
        if (options.preset) stdinConfig.preset = options.preset;
        // Auto-bridge trailingSlash from Astro config if not explicitly set in rules
        if (
          astroTrailingSlash &&
          !options.rules?.url_normalization?.trailing_slash
        ) {
          stdinConfig.url_normalization = {
            ...((stdinConfig.url_normalization as Record<string, unknown>) ??
              {}),
            trailing_slash: astroTrailingSlash,
          };
        }
        if (options.failOn === "warnings" && options.strict !== false) {
          stdinConfig.strict = true;
        } else if (options.strict !== undefined) {
          stdinConfig.strict = options.strict;
        }
        if (options.benchmark !== undefined) stdinConfig.benchmark = options.benchmark;
        if (options.maxWarnings != null) stdinConfig.max_warnings = options.maxWarnings;
        if (options.baseline)
          stdinConfig.baseline = resolve(
            rootDir ?? process.cwd(),
            options.baseline,
          );
        if (options.writeBaseline) stdinConfig.write_baseline = true;
        if (options.hints?.sourceFiles && rootDir) {
          stdinConfig.hints = { source_files: true };
          stdinConfig.project_root = rootDir;
        }
        if (options.maxErrors != null)
          stdinConfig.max_errors = options.maxErrors;
        if (options.pageOverview !== undefined) stdinConfig.page_overview = options.pageOverview;
        if (options.aiVisibility !== undefined) {
          stdinConfig.ai_visibility = { enabled: options.aiVisibility === true };
        }
        if (options.uxHeuristics !== undefined) {
          if (options.uxHeuristics === true) {
            stdinConfig.ux_heuristics = { enabled: true };
          } else if (options.uxHeuristics && typeof options.uxHeuristics === "object") {
            stdinConfig.ux_heuristics = {
              enabled: true,
              ...(options.uxHeuristics.maxLinksPerPage !== undefined
                ? { max_links_per_page: options.uxHeuristics.maxLinksPerPage }
                : {}),
              ...(options.uxHeuristics.minCtaPerPage !== undefined
                ? { min_cta_per_page: options.uxHeuristics.minCtaPerPage }
                : {}),
            };
          }
        }
        if (options.goLive) {
          stdinConfig.go_live = {
            enabled: options.goLive.enabled ?? false,
            ...(options.goLive.expectedSite
              ? { expected_site: options.goLive.expectedSite }
              : effectiveSite
                ? { expected_site: effectiveSite }
                : {}),
            forbidden_domains: options.goLive.forbiddenDomains ?? [],
          };
        }
        const root = rootDir ?? process.cwd();
        const outputPath = options.reports?.json
          ? resolve(root, options.reports.json)
          : options.output
            ? resolve(root, options.output)
            : undefined;
        if (outputPath) stdinConfig.format = "json";
        const outputMarkdownPath = options.reports?.markdown
          ? resolve(root, options.reports.markdown)
          : options.outputMarkdown
            ? resolve(root, options.outputMarkdown)
            : undefined;
        const outputSarifPath = options.reports?.sarif
          ? resolve(root, options.reports.sarif)
          : undefined;

        // Pass extra report formats to the binary so a single run produces all outputs
        const extraReports: Array<{ format: string; path: string }> = [];
        if (outputMarkdownPath) extraReports.push({ format: "markdown", path: outputMarkdownPath });
        if (outputSarifPath) extraReports.push({ format: "sarif", path: outputSarifPath });
        if (extraReports.length > 0) stdinConfig.extra_reports = extraReports;

        const stdinInput = JSON.stringify(stdinConfig);

        logger.info("Running post-build audit...");

        const captureOutput = !!outputPath;

        try {
          const result = deps.execFileSync(binaryPath, args, {
            stdio: ["pipe", captureOutput ? "pipe" : "inherit", "inherit"],
            input: stdinInput,
            encoding: captureOutput ? "utf-8" : undefined,
          });

          if (captureOutput && result) {
            deps.writeFileSync(outputPath!, result as string);
            logger.info(`Report written to ${outputPath}`);
            logJsonSummary(result as string, logger);
          }
          for (const r of extraReports) {
            logger.info(`${r.format.charAt(0).toUpperCase() + r.format.slice(1)} report written to ${r.path}`);
          }

          logger.info("All checks passed!");
        } catch (err: unknown) {
          const exitCode =
            err && typeof err === "object" && "status" in err
              ? (err as { status: number }).status
              : undefined;

          // Write output file even on exit code 1 (findings exist but run succeeded)
          if (
            captureOutput &&
            exitCode === 1 &&
            err &&
            typeof err === "object" &&
            "stdout" in err
          ) {
            const stdout = (err as { stdout: string | Buffer }).stdout;
            if (stdout) {
              const stdoutStr =
                typeof stdout === "string" ? stdout : stdout.toString("utf-8");
              deps.writeFileSync(outputPath!, stdoutStr);
              logger.info(`Report written to ${outputPath}`);
              logJsonSummary(stdoutStr, logger);
            }
          }
          // extra_reports are written by the binary directly; log them on any non-crash exit
          if (exitCode === 1) {
            for (const r of extraReports) {
              logger.info(`${r.format.charAt(0).toUpperCase() + r.format.slice(1)} report written to ${r.path}`);
            }
          }

          if (exitCode === 1) {
            if (shouldFail) {
              throw new Error(
                "astro-post-audit found issues. See output above.",
              );
            }
            logger.warn("Audit found issues. See output above.");
          } else {
            logger.error(
              `Audit failed with exit code ${exitCode ?? "unknown"}`,
            );
          }
        }
      },
    },
  };
}
