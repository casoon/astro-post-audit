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
  site?: { base_url?: string };
  filters?: {
    include?: string[];
    exclude?: string[];
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
