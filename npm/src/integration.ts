import type { AstroIntegration } from 'astro';
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export interface PostAuditOptions {
  /** Path to rules.toml config file */
  config?: string;
  /** Base URL (auto-detected from Astro site config if not set) */
  site?: string;
  /** Treat warnings as errors */
  strict?: boolean;
  /** Output format: 'text' or 'json' */
  format?: 'text' | 'json';
  /** Glob patterns to exclude */
  exclude?: string[];
  /** Skip sitemap checks */
  noSitemapCheck?: boolean;
  /** Enable asset reference checking */
  checkAssets?: boolean;
  /** Enable structured data validation */
  checkStructuredData?: boolean;
  /** Enable security heuristic checks */
  checkSecurity?: boolean;
  /** Enable duplicate content detection */
  checkDuplicates?: boolean;
  /** Disable the integration (useful for dev) */
  disable?: boolean;
}

export default function postAudit(
  options: PostAuditOptions = {},
): AstroIntegration {
  let siteUrl: string | undefined;

  return {
    name: 'astro-post-audit',
    hooks: {
      'astro:config:done': ({ config }) => {
        siteUrl = config.site?.toString();
      },

      'astro:build:done': ({ dir, logger }) => {
        if (options.disable) return;

        const distPath = fileURLToPath(dir);
        const binDir = join(
          dirname(fileURLToPath(import.meta.url)),
          '..',
          'bin',
        );
        const binaryName =
          process.platform === 'win32'
            ? 'astro-post-audit.exe'
            : 'astro-post-audit';
        const binaryPath = join(binDir, binaryName);

        if (!existsSync(binaryPath)) {
          logger.warn(
            'astro-post-audit binary not found. Run "npm rebuild astro-post-audit".',
          );
          return;
        }

        const args: string[] = [distPath];

        // Use site from options, or auto-detect from Astro config
        const site = options.site ?? siteUrl;
        if (site) args.push('--site', site);

        if (options.strict) args.push('--strict');
        if (options.format) args.push('--format', options.format);
        if (options.config) args.push('--config', options.config);
        if (options.noSitemapCheck) args.push('--no-sitemap-check');
        if (options.checkAssets) args.push('--check-assets');
        if (options.checkStructuredData) args.push('--check-structured-data');
        if (options.checkSecurity) args.push('--check-security');
        if (options.checkDuplicates) args.push('--check-duplicates');

        if (options.exclude) {
          for (const pattern of options.exclude) {
            args.push('--exclude', pattern);
          }
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
            logger.warn('Audit found issues. See output above.');
          } else {
            logger.error(`Audit failed with exit code ${exitCode ?? 'unknown'}`);
          }
        }
      },
    },
  };
}
