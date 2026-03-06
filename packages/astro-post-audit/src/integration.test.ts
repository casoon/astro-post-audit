import { describe, it } from "node:test";
import assert from "node:assert/strict";
import postAudit from "./integration.js";
import type { PostAuditOptions } from "./integration.js";
import type { execFileSync as ExecFileSync } from "node:child_process";

function makeLogger() {
  const info: string[] = [];
  const warn: string[] = [];
  const error: string[] = [];
  return {
    logger: {
      info: (msg: string) => info.push(msg),
      warn: (msg: string) => warn.push(msg),
      error: (msg: string) => error.push(msg),
    },
    info,
    warn,
    error,
  };
}

function makeExecMock(
  impl: (_file: string, args: string[]) => string,
): typeof ExecFileSync {
  return ((file: string, argsOrOptions?: unknown) => {
    const args = Array.isArray(argsOrOptions) ? argsOrOptions : [];
    return impl(file, args as string[]);
  }) as typeof ExecFileSync;
}

// ==========================================================================
// postAudit integration factory
// ==========================================================================

describe("postAudit", () => {
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
    const options: PostAuditOptions = {
      strict: true,
      maxErrors: 5,
      pageOverview: false,
      output: "audit-report.json",
      disable: false,
      throwOnError: true,
      rules: { canonical: { require: true } },
    };
    const integration = postAudit(options);
    assert.equal(integration.name, "astro-post-audit");
  });

  it("does not throw when only rules is set", () => {
    const execCalls: Array<{ args: string[] }> = [];
    const deps = {
      existsSync: () => true,
      writeFileSync: () => {},
      execFileSync: makeExecMock((_file: string, args: string[]) => {
        execCalls.push({ args });
        if (args[0] === "--help") return "Usage: ... --config-stdin ...";
        return "";
      }),
    };
    const integration = postAudit(
      {
        rules: { canonical: { require: true } },
      },
      deps,
    );
    const hook = integration.hooks["astro:build:done"] as Function;
    const { logger, error } = makeLogger();
    assert.doesNotThrow(() =>
      hook({
        dir: new URL("file:///tmp/dist/"),
        logger,
      }),
    );
    assert.equal(error.length, 0);
    assert.ok(execCalls.some((c) => c.args[0] === "--help"));
    assert.ok(execCalls.some((c) => c.args.includes("--config-stdin")));
  });

  it("skips execution when disabled", () => {
    const integration = postAudit({ disable: true });
    const hook = integration.hooks["astro:build:done"] as Function;
    // Should return immediately without doing anything
    assert.doesNotThrow(() =>
      hook({
        dir: new URL("file:///tmp/dist/"),
        logger: {
          info: () => {},
          warn: () => {},
          error: () => {},
        },
      }),
    );
  });

  it("logs an error and skips when binary is outdated", () => {
    const execCalls: Array<{ args: string[] }> = [];
    const deps = {
      existsSync: () => true,
      writeFileSync: () => {},
      execFileSync: makeExecMock((_file: string, args: string[]) => {
        execCalls.push({ args });
        if (args[0] === "--help") return "Usage: ... --config <CONFIG> ...";
        return "";
      }),
    };
    const integration = postAudit({}, deps);
    const hook = integration.hooks["astro:build:done"] as Function;
    const { logger, error } = makeLogger();

    hook({
      dir: new URL("file:///tmp/dist/"),
      logger,
    });

    assert.equal(execCalls.filter((c) => c.args[0] === "--help").length, 1);
    assert.equal(
      execCalls.filter((c) => c.args.includes("--config-stdin")).length,
      0,
    );
    assert.equal(error.length, 1);
    assert.match(error[0], /outdated/i);
  });
});
