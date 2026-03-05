import type { AstroIntegration } from 'astro';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, writeFileSync, unlinkSync, rmdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

/**
 * Inline rules config that mirrors the Rust rules.toml structure.
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
    /** Only check files matching these glob patterns. Merged with the top-level `include` option. */
    include?: string[];
    /** Skip files matching these glob patterns (e.g. `["404.html", "drafts/**"]`). Merged with the top-level `exclude` option. */
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
  /** Asset reference and size checks. Enable via `checkAssets` option or set fields here. */
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
  /** Structured data (JSON-LD) validation. Enable via `checkStructuredData` option or set fields here. */
  structured_data?: {
    /** Validate JSON-LD syntax and semantics (`@context`, `@type`, required properties). @default false */
    check_json_ld?: boolean;
    /** Every page must contain at least one JSON-LD block. @default false */
    require_json_ld?: boolean;
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
  /** Security heuristic checks. Enable via `checkSecurity` option or set fields here. */
  security?: {
    /** Warn on `target="_blank"` without `rel="noopener"`. @default true */
    check_target_blank?: boolean;
    /** Warn on `http://` resource URLs (mixed content). @default true */
    check_mixed_content?: boolean;
    /** Warn on inline `<script>` tags. @default false */
    warn_inline_scripts?: boolean;
  };
  /** Duplicate content detection. Enable via `checkDuplicates` option or set fields here. */
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
export function rulesToToml(rules: RulesConfig): string {
  const lines: string[] = [];

  for (const [section, values] of Object.entries(rules)) {
    if (values == null || typeof values !== 'object') continue;
    lines.push(`[${section}]`);
    for (const [key, val] of Object.entries(values as Record<string, unknown>)) {
      if (val === undefined) continue;
      // Quote keys that contain special chars (e.g. rule IDs like "html/lang-missing")
      const tomlKey = /^[a-zA-Z0-9_-]+$/.test(key) ? key : `"${key}"`;
      if (typeof val === 'string') {
        lines.push(`${tomlKey} = "${val}"`);
      } else if (Array.isArray(val)) {
        const items = val.map((v) => `"${v}"`).join(', ');
        lines.push(`${tomlKey} = [${items}]`);
      } else {
        lines.push(`${tomlKey} = ${val}`);
      }
    }
    lines.push('');
  }

  return lines.join('\n');
}

function resolveBinaryPath(): string | null {
  const binDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'bin');
  const binaryName =
    process.platform === 'win32' ? 'astro-post-audit.exe' : 'astro-post-audit';
  const binaryPath = join(binDir, binaryName);
  return existsSync(binaryPath) ? binaryPath : null;
}

export default function postAudit(options: PostAuditOptions = {}): AstroIntegration {
  let siteUrl: string | undefined;

  return {
    name: 'astro-post-audit',
    hooks: {
      'astro:config:done': ({ config }) => {
        siteUrl = config.site?.toString();
      },

      'astro:build:done': ({ dir, logger }) => {
        if (options.disable) return;

        // Validate mutual exclusion: config and rules cannot both be set
        if (options.config && options.rules) {
          throw new Error(
            'astro-post-audit: "config" and "rules" are mutually exclusive. ' +
              'Use "config" to point to a rules.toml file, OR use "rules" to provide inline config — not both.',
          );
        }

        // Validate that rules is a non-empty object if provided
        if (options.rules && typeof options.rules === 'object' && Object.keys(options.rules).length === 0) {
          logger.warn('astro-post-audit: "rules" is an empty object — using default config.');
        }

        const binaryPath = resolveBinaryPath();
        if (!binaryPath) {
          logger.warn(
            'astro-post-audit binary not found. Run "npm rebuild @casoon/astro-post-audit".',
          );
          return;
        }

        const distPath = fileURLToPath(dir);
        const args: string[] = [distPath];

        // --site: explicit option > Astro config auto-detect
        const site = options.site ?? siteUrl;
        if (site) args.push('--site', site);

        // Boolean flags
        if (options.strict) args.push('--strict');
        if (options.noSitemapCheck) args.push('--no-sitemap-check');
        if (options.checkAssets) args.push('--check-assets');
        if (options.checkStructuredData) args.push('--check-structured-data');
        if (options.checkSecurity) args.push('--check-security');
        if (options.checkDuplicates) args.push('--check-duplicates');
        if (options.pageOverview) args.push('--page-overview');

        // Value flags
        if (options.format) args.push('--format', options.format);
        if (options.maxErrors != null) args.push('--max-errors', String(options.maxErrors));

        // Array flags
        if (options.include) {
          for (const pattern of options.include) {
            args.push('--include', pattern);
          }
        }
        if (options.exclude) {
          for (const pattern of options.exclude) {
            args.push('--exclude', pattern);
          }
        }

        // Config: explicit file path OR generate temp from inline rules
        let tempConfigPath: string | undefined;
        if (options.config) {
          args.push('--config', options.config);
        } else if (options.rules) {
          const tempDir = mkdtempSync(join(tmpdir(), 'astro-post-audit-'));
          tempConfigPath = join(tempDir, 'rules.toml');
          writeFileSync(tempConfigPath, rulesToToml(options.rules), 'utf-8');
          args.push('--config', tempConfigPath);
        }

        logger.info('Running post-build audit...');

        try {
          execFileSync(binaryPath, args, { stdio: 'inherit' });
          logger.info('All checks passed!');
        } catch (err: unknown) {
          const exitCode =
            err && typeof err === 'object' && 'status' in err
              ? (err as { status: number }).status
              : undefined;

          if (exitCode === 1) {
            if (options.throwOnError) {
              throw new Error(
                'astro-post-audit found issues. See output above.',
              );
            }
            logger.warn('Audit found issues. See output above.');
          } else {
            logger.error(`Audit failed with exit code ${exitCode ?? 'unknown'}`);
          }
        } finally {
          // Clean up temp config
          if (tempConfigPath) {
            try {
              unlinkSync(tempConfigPath);
              rmdirSync(dirname(tempConfigPath));
            } catch {
              // ignore cleanup errors
            }
          }
        }
      },
    },
  };
}
