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
  impl: (_file: string, args: string[], options?: unknown) => string,
): typeof ExecFileSync {
  return ((file: string, argsOrOptions?: unknown, options?: unknown) => {
    const args = Array.isArray(argsOrOptions) ? argsOrOptions : [];
    return impl(file, args as string[], options);
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

  it("merges contentStyle with detailed content-style rules", () => {
    let auditConfig: Record<string, unknown> | undefined;
    const deps = {
      existsSync: () => true,
      writeFileSync: () => {},
      execFileSync: makeExecMock((_file, args, execOptions) => {
        if (args[0] === "--help") return "Usage: ... --config-stdin ...";
        auditConfig = JSON.parse(
          (execOptions as { input: string }).input,
        ) as Record<string, unknown>;
        return "";
      }),
    };
    const integration = postAudit(
      {
        contentStyle: true,
        rules: {
          content_style: {
            content_selector: ".article-copy",
            extra_rules: [
              { id: "custom", type: "presence", pattern: "Leuchtturm" },
            ],
          },
        },
      },
      deps,
    );
    const hook = integration.hooks["astro:build:done"] as Function;

    hook({
      dir: new URL("file:///tmp/dist/"),
      logger: { info: () => {}, warn: () => {}, error: () => {} },
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
      } as unknown as PostAuditOptions["rules"],
    });
    const hook = integration.hooks["astro:build:done"] as Function;
    const { logger } = makeLogger();

    assert.throws(
      () => hook({ dir: new URL("file:///tmp/dist/"), logger }),
      /content_style\.rules\[0\]\.type.*density_per_1000_words.*density_per1000_words/,
    );
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

  it("throws on binary failures when build failure is enabled", () => {
    const deps = {
      existsSync: () => true,
      writeFileSync: () => {},
      execFileSync: makeExecMock((_file: string, args: string[]) => {
        if (args[0] === "--help") return "Usage: ... --config-stdin ...";
        throw Object.assign(new Error("binary failed"), { status: 2 });
      }),
    };
    const integration = postAudit({ throwOnError: true }, deps);
    const hook = integration.hooks["astro:build:done"] as Function;
    const { logger } = makeLogger();

    assert.throws(
      () => hook({ dir: new URL("file:///tmp/dist/"), logger }),
      /exit code 2/i,
    );
  });

  it("only logs binary failures when build failure is disabled", () => {
    const deps = {
      existsSync: () => true,
      writeFileSync: () => {},
      execFileSync: makeExecMock((_file: string, args: string[]) => {
        if (args[0] === "--help") return "Usage: ... --config-stdin ...";
        throw Object.assign(new Error("binary failed"), { status: 2 });
      }),
    };
    const integration = postAudit({ failOn: "never" }, deps);
    const hook = integration.hooks["astro:build:done"] as Function;
    const { logger, error } = makeLogger();

    assert.doesNotThrow(() =>
      hook({ dir: new URL("file:///tmp/dist/"), logger }),
    );
    assert.deepEqual(error, ["Audit failed with exit code 2"]);
  });

  it("lets failOn never override throwOnError", () => {
    const deps = {
      existsSync: () => true,
      writeFileSync: () => {},
      execFileSync: makeExecMock((_file: string, args: string[]) => {
        if (args[0] === "--help") return "Usage: ... --config-stdin ...";
        throw Object.assign(new Error("findings"), { status: 1 });
      }),
    };
    const integration = postAudit(
      {
        site: "https://example.com",
        failOn: "never",
        throwOnError: true,
      },
      deps,
    );
    const hook = integration.hooks["astro:build:done"] as Function;
    const { logger, warn } = makeLogger();

    assert.doesNotThrow(() =>
      hook({ dir: new URL("file:///tmp/dist/"), logger }),
    );
    assert.deepEqual(warn, ["Audit found issues. See output above."]);
  });

  it("makes failOn warnings imply strict even when strict is false", () => {
    let auditConfig: Record<string, unknown> | undefined;
    const deps = {
      existsSync: () => true,
      writeFileSync: () => {},
      execFileSync: makeExecMock((_file, args, execOptions) => {
        if (args[0] === "--help") return "Usage: ... --config-stdin ...";
        auditConfig = JSON.parse(
          (execOptions as { input: string }).input,
        ) as Record<string, unknown>;
        throw Object.assign(new Error("warnings"), { status: 1 });
      }),
    };
    const integration = postAudit(
      {
        site: "https://example.com",
        failOn: "warnings",
        strict: false,
      },
      deps,
    );
    const hook = integration.hooks["astro:build:done"] as Function;
    const { logger } = makeLogger();

    assert.throws(
      () => hook({ dir: new URL("file:///tmp/dist/"), logger }),
      /found issues/i,
    );
    assert.equal(auditConfig?.strict, true);
  });

  it("makes maxWarnings enable build failure unless failOn is never", () => {
    const deps = {
      existsSync: () => true,
      writeFileSync: () => {},
      execFileSync: makeExecMock((_file: string, args: string[]) => {
        if (args[0] === "--help") return "Usage: ... --config-stdin ...";
        throw Object.assign(new Error("warning threshold exceeded"), {
          status: 1,
        });
      }),
    };
    const integration = postAudit(
      { site: "https://example.com", maxWarnings: 0 },
      deps,
    );
    const hook = integration.hooks["astro:build:done"] as Function;
    const { logger } = makeLogger();

    assert.throws(
      () => hook({ dir: new URL("file:///tmp/dist/"), logger }),
      /found issues/i,
    );

    const neverIntegration = postAudit(
      {
        site: "https://example.com",
        maxWarnings: 0,
        failOn: "never",
      },
      deps,
    );
    const neverHook = neverIntegration.hooks[
      "astro:build:done"
    ] as Function;
    assert.doesNotThrow(() =>
      neverHook({ dir: new URL("file:///tmp/dist/"), logger }),
    );
  });
});
