import { execFileSync } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
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
function logJsonSummary(json, logger) {
    try {
        const report = JSON.parse(json);
        const s = report.summary;
        if (!s)
            return;
        const parts = [];
        if (s.errors)
            parts.push(`${s.errors} error${s.errors === 1 ? "" : "s"}`);
        if (s.warnings)
            parts.push(`${s.warnings} warning${s.warnings === 1 ? "" : "s"}`);
        if (s.info)
            parts.push(`${s.info} info`);
        const filesMsg = s.files_checked !== undefined ? ` (${s.files_checked} files checked)` : "";
        if (parts.length === 0) {
            logger.info(`Audit passed${filesMsg}.`);
        }
        else {
            logger.warn(`Audit: ${parts.join(", ")}${filesMsg}.`);
        }
    }
    catch {
        // non-JSON output, skip summary
    }
}
const GROUP_DEFS = {
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
function deepMerge(base, override) {
    const result = { ...base };
    for (const [key, val] of Object.entries(override)) {
        if (val !== null &&
            typeof val === "object" &&
            !Array.isArray(val) &&
            typeof result[key] === "object" &&
            result[key] !== null) {
            result[key] = deepMerge(result[key], val);
        }
        else {
            result[key] = val;
        }
    }
    return result;
}
function expandGroups(groups, userRules) {
    let accumulated = {};
    const warnOverrides = {};
    for (const [name, value] of Object.entries(groups)) {
        if (!value)
            continue;
        const def = GROUP_DEFS[name];
        accumulated = deepMerge(accumulated, def.rules);
        if (value === "warn") {
            for (const id of def.ruleIds) {
                warnOverrides[id] = "warning";
            }
        }
    }
    const merged = deepMerge(accumulated, userRules);
    if (Object.keys(warnOverrides).length > 0) {
        merged.severity = { ...warnOverrides, ...userRules.severity };
    }
    return merged;
}
export default function postAudit(options = {}, deps = defaultDeps) {
    let siteUrl;
    let rootDir;
    let astroTrailingSlash;
    let astroOutput;
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
                if (options.disable ||
                    process.env.SKIP_AUDIT === "1" ||
                    process.env.SKIP_AUDIT === "true") {
                    if (process.env.SKIP_AUDIT) {
                        logger.info("Audit skipped (SKIP_AUDIT is set).");
                    }
                    return;
                }
                const shouldFail = options.throwOnError === true ||
                    (options.failOn !== undefined && options.failOn !== "never");
                const resolvedRules = options.groups
                    ? expandGroups(options.groups, options.rules ?? {})
                    : (options.rules ?? {});
                // Validate that rules is a non-empty object if provided
                if (options.rules &&
                    typeof options.rules === "object" &&
                    Object.keys(options.rules).length === 0) {
                    logger.warn('astro-post-audit: "rules" is an empty object — using default config.');
                }
                // Warn: `site` missing but canonical/sitemap checks active
                const effectiveSite = options.site ?? siteUrl;
                if (!effectiveSite) {
                    const canonicalActive = resolvedRules.canonical?.require !== false;
                    const sitemapActive = resolvedRules.sitemap?.canonical_must_be_in_sitemap !== false;
                    if (canonicalActive || sitemapActive) {
                        logger.warn("`site` is not set in astro.config.mjs. Canonical and sitemap URL checks will be limited — set `site` to enable full URL verification.");
                    }
                }
                const binaryPath = resolveBinaryPath(deps.existsSync);
                if (!binaryPath) {
                    const msg = 'astro-post-audit binary not found. Run "npm rebuild @casoon/astro-post-audit".';
                    if (shouldFail)
                        throw new Error(msg);
                    logger.warn(msg);
                    return;
                }
                if (!supportsConfigStdin(binaryPath, deps.execFileSync)) {
                    const msg = 'astro-post-audit binary is outdated and does not support --config-stdin. Run "npm rebuild @casoon/astro-post-audit".';
                    if (shouldFail)
                        throw new Error(msg);
                    logger.error(msg);
                    return;
                }
                let distPath = fileURLToPath(dir);
                // For SSR/hybrid builds, static HTML lives under dist/client/ (adapter convention)
                if (astroOutput === "server" || astroOutput === "hybrid") {
                    const clientPath = join(distPath, "client");
                    if (deps.existsSync(clientPath)) {
                        distPath = clientPath;
                        logger.info("SSR/hybrid build detected: auditing static output in dist/client/");
                    }
                    else {
                        logger.info("SSR/hybrid build detected: no dist/client/ found, auditing dist/ directly.");
                    }
                }
                const args = [distPath, "--config-stdin"];
                // Info: sitemap checks active but no sitemap.xml in dist
                const sitemapChecksActive = resolvedRules.sitemap?.canonical_must_be_in_sitemap !== false ||
                    resolvedRules.sitemap?.entries_must_exist_in_dist !== false;
                if (sitemapChecksActive &&
                    !deps.existsSync(join(distPath, "sitemap.xml")) &&
                    !deps.existsSync(join(distPath, "sitemap-index.xml"))) {
                    logger.info("No sitemap.xml found in dist/. Sitemap checks are limited. Add @astrojs/sitemap or @casoon/astro-sitemap to generate a sitemap.");
                }
                // Build the full JSON config for the Rust binary
                const site = effectiveSite;
                const stdinConfig = {
                    ...resolvedRules,
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
                if (options.failOn === "warnings" && options.strict !== false) {
                    stdinConfig.strict = true;
                }
                else if (options.strict !== undefined) {
                    stdinConfig.strict = options.strict;
                }
                if (options.benchmark !== undefined)
                    stdinConfig.benchmark = options.benchmark;
                if (options.progress !== undefined)
                    stdinConfig.progress = options.progress;
                if (options.debug !== undefined)
                    stdinConfig.debug = options.debug;
                if (options.maxWarnings != null)
                    stdinConfig.max_warnings = options.maxWarnings;
                if (options.baseline)
                    stdinConfig.baseline = resolve(rootDir ?? process.cwd(), options.baseline);
                if (options.writeBaseline)
                    stdinConfig.write_baseline = true;
                if (options.hints?.sourceFiles && rootDir) {
                    stdinConfig.hints = { source_files: true };
                    stdinConfig.project_root = rootDir;
                }
                // content_sync needs the project root to locate src/content/.
                if (resolvedRules.content_sync?.enabled && rootDir && !stdinConfig.project_root) {
                    stdinConfig.project_root = rootDir;
                }
                if (options.maxErrors != null)
                    stdinConfig.max_errors = options.maxErrors;
                if (options.pageOverview !== undefined)
                    stdinConfig.page_overview = options.pageOverview;
                if (options.aiVisibility !== undefined) {
                    stdinConfig.ai_visibility = { enabled: options.aiVisibility === true };
                }
                if (options.uxHeuristics !== undefined) {
                    if (options.uxHeuristics === true) {
                        stdinConfig.ux_heuristics = { enabled: true };
                    }
                    else if (options.uxHeuristics && typeof options.uxHeuristics === "object") {
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
                if (outputPath)
                    stdinConfig.format = "json";
                const outputMarkdownPath = options.reports?.markdown
                    ? resolve(root, options.reports.markdown)
                    : options.outputMarkdown
                        ? resolve(root, options.outputMarkdown)
                        : undefined;
                const outputSarifPath = options.reports?.sarif
                    ? resolve(root, options.reports.sarif)
                    : undefined;
                // Pass extra report formats to the binary so a single run produces all outputs
                const extraReports = [];
                if (outputMarkdownPath)
                    extraReports.push({ format: "markdown", path: outputMarkdownPath });
                if (outputSarifPath)
                    extraReports.push({ format: "sarif", path: outputSarifPath });
                if (extraReports.length > 0)
                    stdinConfig.extra_reports = extraReports;
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
                        deps.writeFileSync(outputPath, result);
                        logger.info(`Report written to ${outputPath}`);
                        logJsonSummary(result, logger);
                    }
                    for (const r of extraReports) {
                        logger.info(`${r.format.charAt(0).toUpperCase() + r.format.slice(1)} report written to ${r.path}`);
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
                            const stdoutStr = typeof stdout === "string" ? stdout : stdout.toString("utf-8");
                            deps.writeFileSync(outputPath, stdoutStr);
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
