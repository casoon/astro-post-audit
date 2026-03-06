import { execFileSync } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
const defaultDeps = {
    execFileSync,
    existsSync,
    writeFileSync,
};
function resolveBinaryPath(exists) {
    const binDir = join(dirname(fileURLToPath(import.meta.url)), "..", "bin");
    const binaryName = process.platform === "win32" ? "astro-post-audit.exe" : "astro-post-audit";
    const binaryPath = join(binDir, binaryName);
    return exists(binaryPath) ? binaryPath : null;
}
function supportsConfigStdin(binaryPath, run) {
    try {
        const help = run(binaryPath, ["--help"], {
            stdio: ["ignore", "pipe", "ignore"],
            encoding: "utf-8",
        });
        return typeof help === "string" && help.includes("--config-stdin");
    }
    catch {
        return false;
    }
}
export default function postAudit(options = {}, deps = defaultDeps) {
    let siteUrl;
    let astroTrailingSlash;
    return {
        name: "astro-post-audit",
        hooks: {
            "astro:config:done": ({ config }) => {
                siteUrl = config.site?.toString();
                // Bridge Astro's trailingSlash config automatically
                if (config.trailingSlash) {
                    astroTrailingSlash = config.trailingSlash;
                }
            },
            "astro:build:done": ({ dir, logger }) => {
                if (options.disable ||
                    process.env.SKIP_AUDIT === "1" ||
                    process.env.SKIP_AUDIT === "true") {
                    if (process.env.SKIP_AUDIT) {
                        logger.info("Audit skipped (SKIP_AUDIT is set).");
                    }
                    return;
                }
                // Validate that rules is a non-empty object if provided
                if (options.rules &&
                    typeof options.rules === "object" &&
                    Object.keys(options.rules).length === 0) {
                    logger.warn('astro-post-audit: "rules" is an empty object — using default config.');
                }
                const binaryPath = resolveBinaryPath(deps.existsSync);
                if (!binaryPath) {
                    logger.warn('astro-post-audit binary not found. Run "npm rebuild @casoon/astro-post-audit".');
                    return;
                }
                if (!supportsConfigStdin(binaryPath, deps.execFileSync)) {
                    logger.error('astro-post-audit binary is outdated and does not support --config-stdin. Run "npm rebuild @casoon/astro-post-audit".');
                    return;
                }
                const distPath = fileURLToPath(dir);
                const args = [distPath, "--config-stdin"];
                // Build the full JSON config for the Rust binary
                const site = options.site ?? siteUrl;
                const stdinConfig = {
                    ...options.rules,
                };
                if (site)
                    stdinConfig.site = { base_url: site };
                if (options.preset)
                    stdinConfig.preset = options.preset;
                // Auto-bridge trailingSlash from Astro config if not explicitly set in rules
                if (astroTrailingSlash &&
                    !options.rules?.url_normalization?.trailing_slash) {
                    stdinConfig.url_normalization = {
                        ...(stdinConfig.url_normalization ??
                            {}),
                        trailing_slash: astroTrailingSlash,
                    };
                }
                if (options.strict)
                    stdinConfig.strict = true;
                if (options.benchmark)
                    stdinConfig.benchmark = true;
                if (options.maxErrors != null)
                    stdinConfig.max_errors = options.maxErrors;
                if (options.pageOverview)
                    stdinConfig.page_overview = true;
                if (options.output)
                    stdinConfig.format = "json";
                const stdinInput = JSON.stringify(stdinConfig);
                logger.info("Running post-build audit...");
                const captureOutput = !!options.output;
                try {
                    const result = deps.execFileSync(binaryPath, args, {
                        stdio: ["pipe", captureOutput ? "pipe" : "inherit", "inherit"],
                        input: stdinInput,
                        encoding: captureOutput ? "utf-8" : undefined,
                    });
                    if (captureOutput && result) {
                        deps.writeFileSync(options.output, result);
                        logger.info(`Report written to ${options.output}`);
                    }
                    logger.info("All checks passed!");
                }
                catch (err) {
                    const exitCode = err && typeof err === "object" && "status" in err
                        ? err.status
                        : undefined;
                    // Write output file even on exit code 1 (findings exist but run succeeded)
                    if (captureOutput &&
                        exitCode === 1 &&
                        err &&
                        typeof err === "object" &&
                        "stdout" in err) {
                        const stdout = err.stdout;
                        if (stdout) {
                            deps.writeFileSync(options.output, stdout);
                            logger.info(`Report written to ${options.output}`);
                        }
                    }
                    if (exitCode === 1) {
                        if (options.throwOnError) {
                            throw new Error("astro-post-audit found issues. See output above.");
                        }
                        logger.warn("Audit found issues. See output above.");
                    }
                    else {
                        logger.error(`Audit failed with exit code ${exitCode ?? "unknown"}`);
                    }
                }
            },
        },
    };
}
