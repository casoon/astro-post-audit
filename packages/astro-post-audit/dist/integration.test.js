import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import postAudit from "./integration.js";
function makeLogger() {
    const info = [];
    const warn = [];
    const error = [];
    return {
        logger: {
            info: (msg) => info.push(msg),
            warn: (msg) => warn.push(msg),
            error: (msg) => error.push(msg),
        },
        info,
        warn,
        error,
    };
}
function makeExecMock(impl) {
    return ((file, argsOrOptions, options) => {
        const args = Array.isArray(argsOrOptions) ? argsOrOptions : [];
        return impl(file, args, options);
    });
}
// ==========================================================================
// postAudit integration factory
// ==========================================================================
describe("postAudit", () => {
    it("publishes type-correct documented defaults in the JSON schema", () => {
        const schema = JSON.parse(readFileSync(new URL("../schema.json", import.meta.url), "utf-8"));
        const rules = schema.definitions.RulesConfig.properties;
        for (const section of ["ai_visibility", "c2pa", "content_style", "ux_heuristics"]) {
            assert.equal(rules[section].default, undefined);
        }
        assert.equal(schema.properties.strict.default, false);
        assert.equal(schema.properties.benchmark.default, false);
        assert.equal(schema.properties.pageOverview.default, false);
        assert.equal(schema.properties.disable.default, false);
        assert.equal(schema.properties.throwOnError.default, false);
        assert.equal(schema.properties.writeBaseline.default, false);
        assert.equal(rules.source_analysis.properties?.min_duplicate_occurrences.default, 3);
        assert.equal(rules.source_analysis.properties?.max_component_lines.default, 300);
        assert.equal(rules.source_analysis.properties?.max_component_props.default, 12);
        assert.equal(rules.source_analysis.properties?.max_component_slots.default, 6);
    });
    it("returns an AstroIntegration with correct name", () => {
        const integration = postAudit();
        assert.equal(integration.name, "astro-post-audit");
        assert.ok(integration.hooks);
    });
    it("accepts empty options", () => {
        const integration = postAudit({});
        assert.equal(integration.name, "astro-post-audit");
    });
    it("accepts all option types", () => {
        const options = {
            strict: true,
            maxErrors: 5,
            pageOverview: false,
            output: "audit-report.json",
            disable: false,
            throwOnError: true,
            rules: {
                canonical: { require: true },
                css_architecture: {
                    enabled: true,
                    max_route_kb: 60,
                    detect_route_outliers: true,
                    outlier_factor: 2.5,
                    min_outlier_kb: 25,
                },
            },
        };
        const integration = postAudit(options);
        assert.equal(integration.name, "astro-post-audit");
    });
    it("does not throw when only rules is set", () => {
        const execCalls = [];
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args) => {
                execCalls.push({ args });
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                return "";
            }),
        };
        const integration = postAudit({
            rules: { canonical: { require: true } },
        }, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger, error } = makeLogger();
        assert.doesNotThrow(() => hook({
            dir: new URL("file:///tmp/dist/"),
            logger,
        }));
        assert.equal(error.length, 0);
        assert.ok(execCalls.some((c) => c.args[0] === "--help"));
        assert.ok(execCalls.some((c) => c.args.includes("--config-stdin")));
    });
    it("merges contentStyle with detailed content-style rules", () => {
        let auditConfig;
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args, execOptions) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                auditConfig = JSON.parse(execOptions.input);
                return "";
            }),
        };
        const integration = postAudit({
            contentStyle: true,
            rules: {
                content_style: {
                    content_selector: ".article-copy",
                    extra_rules: [
                        { id: "custom", type: "presence", pattern: "Leuchtturm" },
                    ],
                },
            },
        }, deps);
        const hook = integration.hooks["astro:build:done"];
        hook({
            dir: new URL("file:///tmp/dist/"),
            logger: { info: () => { }, warn: () => { }, error: () => { } },
        });
        assert.deepEqual(auditConfig?.content_style, {
            content_selector: ".article-copy",
            extra_rules: [
                { id: "custom", type: "presence", pattern: "Leuchtturm" },
            ],
            enabled: true,
        });
    });
    it("rejects invalid content-style rule types from JavaScript config", () => {
        const integration = postAudit({
            rules: {
                content_style: {
                    rules: [
                        {
                            id: "typoed-density",
                            type: "density_per1000_words",
                            pattern: "—",
                            threshold: 12,
                        },
                    ],
                },
            },
        });
        const hook = integration.hooks["astro:build:done"];
        const { logger } = makeLogger();
        assert.throws(() => hook({ dir: new URL("file:///tmp/dist/"), logger }), /content_style\.rules\[0\]\.type.*density_per_1000_words.*density_per1000_words/);
    });
    it("skips execution when disabled", () => {
        const integration = postAudit({ disable: true });
        const hook = integration.hooks["astro:build:done"];
        // Should return immediately without doing anything
        assert.doesNotThrow(() => hook({
            dir: new URL("file:///tmp/dist/"),
            logger: {
                info: () => { },
                warn: () => { },
                error: () => { },
            },
        }));
    });
    it("logs an error and skips when binary is outdated", () => {
        const execCalls = [];
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args) => {
                execCalls.push({ args });
                if (args[0] === "--help")
                    return "Usage: ... --config <CONFIG> ...";
                return "";
            }),
        };
        const integration = postAudit({}, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger, error } = makeLogger();
        hook({
            dir: new URL("file:///tmp/dist/"),
            logger,
        });
        assert.equal(execCalls.filter((c) => c.args[0] === "--help").length, 1);
        assert.equal(execCalls.filter((c) => c.args.includes("--config-stdin")).length, 0);
        assert.equal(error.length, 1);
        assert.match(error[0], /outdated/i);
    });
    it("throws on binary failures when build failure is enabled", () => {
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                throw Object.assign(new Error("binary failed"), { status: 2 });
            }),
        };
        const integration = postAudit({ throwOnError: true }, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger } = makeLogger();
        assert.throws(() => hook({ dir: new URL("file:///tmp/dist/"), logger }), /exit code 2/i);
    });
    it("only logs binary failures when build failure is disabled", () => {
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                throw Object.assign(new Error("binary failed"), { status: 2 });
            }),
        };
        const integration = postAudit({ failOn: "never" }, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger, error } = makeLogger();
        assert.doesNotThrow(() => hook({ dir: new URL("file:///tmp/dist/"), logger }));
        assert.deepEqual(error, ["Audit failed with exit code 2"]);
    });
    it("lets failOn never override throwOnError", () => {
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                throw Object.assign(new Error("findings"), { status: 1 });
            }),
        };
        const integration = postAudit({
            site: "https://example.com",
            failOn: "never",
            throwOnError: true,
        }, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger, warn } = makeLogger();
        assert.doesNotThrow(() => hook({ dir: new URL("file:///tmp/dist/"), logger }));
        assert.deepEqual(warn, ["Audit found issues. See output above."]);
    });
    it("makes failOn warnings imply strict even when strict is false", () => {
        let auditConfig;
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args, execOptions) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                auditConfig = JSON.parse(execOptions.input);
                throw Object.assign(new Error("warnings"), { status: 1 });
            }),
        };
        const integration = postAudit({
            site: "https://example.com",
            failOn: "warnings",
            strict: false,
        }, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger } = makeLogger();
        assert.throws(() => hook({ dir: new URL("file:///tmp/dist/"), logger }), /found issues/i);
        assert.equal(auditConfig?.strict, true);
    });
    it("expands groups.viz_alternatives and warn groups to current rule ids", () => {
        let auditConfig;
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args, execOptions) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                auditConfig = JSON.parse(execOptions.input);
                return "";
            }),
        };
        const integration = postAudit({
            site: "https://example.com",
            groups: { viz_alternatives: true, a11y: "warn" },
        }, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger } = makeLogger();
        hook({ dir: new URL("file:///tmp/dist/"), logger });
        const a11y = auditConfig?.a11y;
        assert.equal(a11y.viz_alternatives, true);
        const severity = auditConfig?.severity;
        assert.equal(severity["images/alt-missing"], "warning");
        assert.equal(Object.keys(severity).some((id) => id.startsWith("a11y/")), false);
    });
    it("makes maxWarnings enable build failure unless failOn is never", () => {
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                throw Object.assign(new Error("warning threshold exceeded"), {
                    status: 1,
                });
            }),
        };
        const integration = postAudit({ site: "https://example.com", maxWarnings: 0 }, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger } = makeLogger();
        assert.throws(() => hook({ dir: new URL("file:///tmp/dist/"), logger }), /found issues/i);
        const neverIntegration = postAudit({
            site: "https://example.com",
            maxWarnings: 0,
            failOn: "never",
        }, deps);
        const neverHook = neverIntegration.hooks["astro:build:done"];
        assert.doesNotThrow(() => neverHook({ dir: new URL("file:///tmp/dist/"), logger }));
    });
    /** Runs the build hook with `options` and returns the config sent to the binary. */
    function auditConfigFor(options, readReport = () => "") {
        let config = {};
        let stdio;
        const deps = {
            existsSync: () => true,
            readFileSync: readReport,
            execFileSync: makeExecMock((_file, args, execOptions) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                const opts = execOptions;
                config = JSON.parse(opts.input);
                stdio = opts.stdio;
                return "";
            }),
        };
        const integration = postAudit({ site: "https://example.com", ...options }, deps);
        const hook = integration.hooks["astro:build:done"];
        const { logger, info } = makeLogger();
        hook({ dir: new URL("file:///tmp/dist/"), logger });
        return { config, stdio, info };
    }
    it("lets uxHeuristics switch the check without dropping rules.ux_heuristics (#101)", () => {
        const rules = { ux_heuristics: { enabled: true, max_links_per_page: 20 } };
        assert.deepEqual(auditConfigFor({ rules, uxHeuristics: false }).config.ux_heuristics, {
            enabled: false,
            max_links_per_page: 20,
        });
        assert.deepEqual(auditConfigFor({ rules, uxHeuristics: true }).config.ux_heuristics, {
            enabled: true,
            max_links_per_page: 20,
        });
        assert.deepEqual(auditConfigFor({ rules, uxHeuristics: { minCtaPerPage: 2 } }).config.ux_heuristics, { enabled: true, max_links_per_page: 20, min_cta_per_page: 2 });
    });
    it("passes the check modules of warn groups (#98)", () => {
        assert.deepEqual(auditConfigFor({ groups: { a11y: "warn", performance: true } }).config.warn_modules, ["a11y_core"]);
        assert.equal(auditConfigFor({ groups: { a11y: true } }).config.warn_modules, undefined);
    });
    it("lets the binary write the JSON report instead of capturing stdout (#96)", () => {
        const { config, stdio, info } = auditConfigFor({ reports: { json: "audit-report.json" } }, () => JSON.stringify({ summary: { errors: 0, files_checked: 3 } }));
        const reports = config.extra_reports;
        assert.equal(reports.length, 1);
        assert.equal(reports[0].format, "json");
        assert.ok(reports[0].path.endsWith("audit-report.json"));
        assert.deepEqual(stdio, ["pipe", "ignore", "inherit"]);
        assert.ok(info.includes("Audit passed (3 files checked)."));
    });
    it("passes Astro's base to the binary (#97)", () => {
        let config = {};
        const deps = {
            existsSync: () => true,
            readFileSync: () => "",
            execFileSync: makeExecMock((_file, args, execOptions) => {
                if (args[0] === "--help")
                    return "Usage: ... --config-stdin ...";
                config = JSON.parse(execOptions.input);
                return "";
            }),
        };
        const integration = postAudit({}, deps);
        const configDone = integration.hooks["astro:config:done"];
        configDone({
            config: { site: new URL("https://example.com"), base: "/site/", root: new URL("file:///tmp/") },
            buildOutput: "static",
        });
        const hook = integration.hooks["astro:build:done"];
        hook({ dir: new URL("file:///tmp/dist/"), logger: makeLogger().logger });
        assert.deepEqual(config.site, { base_url: "https://example.com/", base_path: "/site" });
    });
});
