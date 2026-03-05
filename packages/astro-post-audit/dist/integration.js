import { execFileSync } from 'node:child_process';
import { existsSync, writeFileSync } from 'node:fs';
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
                const args = [distPath, '--config-stdin'];
                // Build the full JSON config for the Rust binary
                const site = options.site ?? siteUrl;
                const stdinConfig = {
                    ...options.rules,
                };
                if (site)
                    stdinConfig.site = { base_url: site };
                if (options.strict)
                    stdinConfig.strict = true;
                if (options.maxErrors != null)
                    stdinConfig.max_errors = options.maxErrors;
                if (options.pageOverview)
                    stdinConfig.page_overview = true;
                if (options.output)
                    stdinConfig.format = 'json';
                const stdinInput = JSON.stringify(stdinConfig);
                logger.info('Running post-build audit...');
                const captureOutput = !!options.output;
                try {
                    const result = execFileSync(binaryPath, args, {
                        stdio: ['pipe', captureOutput ? 'pipe' : 'inherit', 'inherit'],
                        input: stdinInput,
                        encoding: captureOutput ? 'utf-8' : undefined,
                    });
                    if (captureOutput && result) {
                        writeFileSync(options.output, result);
                        logger.info(`Report written to ${options.output}`);
                    }
                    logger.info('All checks passed!');
                }
                catch (err) {
                    const exitCode = err && typeof err === 'object' && 'status' in err
                        ? err.status
                        : undefined;
                    // Write output file even on exit code 1 (findings exist but run succeeded)
                    if (captureOutput && exitCode === 1 && err && typeof err === 'object' && 'stdout' in err) {
                        const stdout = err.stdout;
                        if (stdout) {
                            writeFileSync(options.output, stdout);
                            logger.info(`Report written to ${options.output}`);
                        }
                    }
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
