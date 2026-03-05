import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
function resolveBinaryPath() {
    const binDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'bin');
    const binaryName = process.platform === 'win32' ? 'astro-post-audit.exe' : 'astro-post-audit';
    const binaryPath = join(binDir, binaryName);
    return existsSync(binaryPath) ? binaryPath : null;
}
export default function postAudit(options = {}) {
    let siteUrl;
    return {
        name: 'astro-post-audit',
        hooks: {
            'astro:config:done': ({ config }) => {
                siteUrl = config.site?.toString();
            },
            'astro:build:done': ({ dir, logger }) => {
                if (options.disable)
                    return;
                // Validate that rules is a non-empty object if provided
                if (options.rules && typeof options.rules === 'object' && Object.keys(options.rules).length === 0) {
                    logger.warn('astro-post-audit: "rules" is an empty object — using default config.');
                }
                const binaryPath = resolveBinaryPath();
                if (!binaryPath) {
                    logger.warn('astro-post-audit binary not found. Run "npm rebuild @casoon/astro-post-audit".');
                    return;
                }
                const distPath = fileURLToPath(dir);
                const args = [distPath];
                // --site: explicit option > Astro config auto-detect
                const site = options.site ?? siteUrl;
                if (site)
                    args.push('--site', site);
                // Boolean flags
                if (options.strict)
                    args.push('--strict');
                if (options.noSitemapCheck)
                    args.push('--no-sitemap-check');
                if (options.checkAssets)
                    args.push('--check-assets');
                if (options.checkStructuredData)
                    args.push('--check-structured-data');
                if (options.checkSecurity)
                    args.push('--check-security');
                if (options.checkDuplicates)
                    args.push('--check-duplicates');
                if (options.pageOverview)
                    args.push('--page-overview');
                // Value flags
                if (options.format)
                    args.push('--format', options.format);
                if (options.maxErrors != null)
                    args.push('--max-errors', String(options.maxErrors));
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
                // Pipe inline rules config via stdin as JSON
                let stdinInput;
                if (options.rules) {
                    args.push('--config-stdin');
                    stdinInput = JSON.stringify(options.rules);
                }
                logger.info('Running post-build audit...');
                try {
                    execFileSync(binaryPath, args, {
                        stdio: stdinInput ? ['pipe', 'inherit', 'inherit'] : 'inherit',
                        input: stdinInput,
                    });
                    logger.info('All checks passed!');
                }
                catch (err) {
                    const exitCode = err && typeof err === 'object' && 'status' in err
                        ? err.status
                        : undefined;
                    if (exitCode === 1) {
                        if (options.throwOnError) {
                            throw new Error('astro-post-audit found issues. See output above.');
                        }
                        logger.warn('Audit found issues. See output above.');
                    }
                    else {
                        logger.error(`Audit failed with exit code ${exitCode ?? 'unknown'}`);
                    }
                }
            },
        },
    };
}
