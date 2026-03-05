#!/usr/bin/env node

/**
 * Thin wrapper that executes the platform-specific astro-post-audit binary.
 */

const { execFileSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const binaryName =
  process.platform === "win32" ? "astro-post-audit.exe" : "astro-post-audit";
const binaryPath = path.join(__dirname, binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(
    `Error: ${binaryName} not found at ${binaryPath}\n` +
      "Run 'npm rebuild astro-post-audit' or reinstall the package."
  );
  process.exit(2);
}

try {
  execFileSync(binaryPath, process.argv.slice(2), {
    stdio: "inherit",
  });
} catch (err) {
  // execFileSync throws on non-zero exit code, forward it
  process.exit(err.status ?? 2);
}
