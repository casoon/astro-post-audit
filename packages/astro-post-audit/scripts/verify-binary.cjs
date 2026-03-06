#!/usr/bin/env node

/**
 * CI guard: if a local binary is present in bin/, ensure it supports
 * the integration protocol (--config-stdin) and matches package version.
 */

const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const pkg = require("../package.json");
const binDir = path.join(__dirname, "..", "bin");
const binName =
  process.platform === "win32" ? "astro-post-audit.exe" : "astro-post-audit";
const binPath = path.join(binDir, binName);

if (!fs.existsSync(binPath)) {
  console.log("verify:binary: no bundled binary found, skipping.");
  process.exit(0);
}

function run(args) {
  return execFileSync(binPath, args, {
    stdio: ["ignore", "pipe", "pipe"],
    encoding: "utf-8",
  });
}

try {
  const help = run(["--help"]);
  if (!help.includes("--config-stdin")) {
    console.error(
      "verify:binary: binary does not support --config-stdin (likely stale).",
    );
    process.exit(1);
  }

  const versionOut = run(["--version"]).trim();
  const version = versionOut.split(/\s+/).pop();
  if (version !== pkg.version) {
    console.error(
      `verify:binary: binary version ${version} does not match package.json ${pkg.version}.`,
    );
    process.exit(1);
  }

  console.log(`verify:binary: OK (${version})`);
} catch (err) {
  console.error("verify:binary: failed to execute bundled binary.");
  console.error(err && err.message ? err.message : String(err));
  process.exit(1);
}
