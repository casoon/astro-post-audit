import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, mkdirSync, readFileSync, symlinkSync, writeFileSync, } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = dirname(here);
const packageNodeModules = join(packageRoot, "node_modules");
const astroCli = join(packageNodeModules, "astro", "astro.js");
const integrationUrl = pathToFileURL(join(here, "integration.js")).href;
function writeProject(root, pageContent, optionsLiteral) {
    mkdirSync(join(root, "src", "pages"), { recursive: true });
    symlinkSync(packageNodeModules, join(root, "node_modules"), "junction");
    writeFileSync(join(root, "astro.config.mjs"), `import { defineConfig } from "astro/config";
import postAudit from "${integrationUrl}";

export default defineConfig({
  site: "https://example.com",
  integrations: [postAudit(${optionsLiteral})],
});`);
    writeFileSync(join(root, "src", "pages", "index.astro"), pageContent);
}
function runAstroBuild(cwd, envOverrides = {}) {
    try {
        const out = execFileSync(process.execPath, [astroCli, "build"], {
            cwd,
            stdio: ["ignore", "pipe", "pipe"],
            encoding: "utf-8",
            env: { ...process.env, NO_COLOR: "1", ...envOverrides },
        });
        return { ok: true, output: out };
    }
    catch (err) {
        const stdout = err && typeof err === "object" && "stdout" in err
            ? String(err.stdout ?? "")
            : "";
        const stderr = err && typeof err === "object" && "stderr" in err
            ? String(err.stderr ?? "")
            : "";
        return { ok: false, output: `${stdout}\n${stderr}` };
    }
}
describe("postAudit e2e", () => {
    it("passes on a valid Astro build fixture", () => {
        const root = mkdtempSync(join(tmpdir(), "astro-post-audit-good-"));
        writeProject(root, `---
const title = "Home";
---
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
    <link rel="canonical" href="https://example.com/" />
  </head>
  <body>
    <h1>Home</h1>
    <a href="/">Home</a>
  </body>
</html>`, "{ throwOnError: true }");
        const result = runAstroBuild(root);
        assert.equal(result.ok, true, result.output);
    });
    it("fails build on invalid fixture when throwOnError is enabled", () => {
        const root = mkdtempSync(join(tmpdir(), "astro-post-audit-bad-"));
        writeProject(root, `---
const title = "";
---
<html>
  <head>
    <meta charset="utf-8" />
    <title>{title}</title>
  </head>
  <body>
    <h1></h1>
    <img src="/x.png" />
  </body>
</html>`, "{ throwOnError: true }");
        const result = runAstroBuild(root);
        assert.equal(result.ok, false, "build should fail for invalid fixture");
        assert.match(result.output, /astro-post-audit found issues/i);
    });
    it("writes JSON report when output option is set", () => {
        const root = mkdtempSync(join(tmpdir(), "astro-post-audit-output-"));
        writeProject(root, `---
const title = "";
---
<html>
  <head>
    <meta charset="utf-8" />
    <title>{title}</title>
  </head>
  <body><h1>Home</h1></body>
</html>`, "{ throwOnError: false, output: 'audit-report.json' }");
        const result = runAstroBuild(root);
        assert.equal(result.ok, true, result.output);
        const reportPath = join(root, "audit-report.json");
        assert.equal(existsSync(reportPath), true, "report should exist");
        const report = JSON.parse(readFileSync(reportPath, "utf-8"));
        assert.ok(report.summary);
        assert.ok(Array.isArray(report.findings));
    });
    it("fails on warnings when strict is enabled", () => {
        const root = mkdtempSync(join(tmpdir(), "astro-post-audit-strict-"));
        writeProject(root, `---
const title = "This title is intentionally much longer than sixty characters to trigger warning";
---
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
    <link rel="canonical" href="https://example.com/" />
  </head>
  <body><h1>Home</h1></body>
</html>`, "{ throwOnError: true, strict: true }");
        const result = runAstroBuild(root);
        assert.equal(result.ok, false, "strict mode should fail on warning");
        assert.match(result.output, /found issues/i);
    });
    it("skips audit when SKIP_AUDIT=1", () => {
        const root = mkdtempSync(join(tmpdir(), "astro-post-audit-skip-"));
        writeProject(root, `---
const title = "";
---
<html>
  <head>
    <title>{title}</title>
  </head>
  <body><img src="/x.png" /></body>
</html>`, "{ throwOnError: true }");
        const result = runAstroBuild(root, { SKIP_AUDIT: "1" });
        assert.equal(result.ok, true, result.output);
        assert.match(result.output, /Audit skipped/i);
    });
});
