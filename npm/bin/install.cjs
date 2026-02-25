#!/usr/bin/env node

/**
 * Post-install script: downloads the prebuilt astro-post-audit binary
 * for the current platform from GitHub Releases.
 */

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");

const PACKAGE = "astro-post-audit";
const VERSION = require("../package.json").version;
const REPO = "casoon/astro-post-audit";

function getPlatformTarget() {
  const platform = process.platform;
  const arch = process.arch;

  const targets = {
    "darwin-x64": "x86_64-apple-darwin",
    "darwin-arm64": "aarch64-apple-darwin",
    "linux-x64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "win32-x64": "x86_64-pc-windows-msvc",
    "win32-arm64": "aarch64-pc-windows-msvc",
  };

  const key = `${platform}-${arch}`;
  const target = targets[key];

  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`Supported platforms: ${Object.keys(targets).join(", ")}`);
    process.exit(1);
  }

  return target;
}

function getBinaryName() {
  return process.platform === "win32"
    ? `${PACKAGE}.exe`
    : PACKAGE;
}

function getDownloadUrl(target) {
  const ext = process.platform === "win32" ? ".zip" : ".tar.gz";
  return `https://github.com/${REPO}/releases/download/v${VERSION}/${PACKAGE}-v${VERSION}-${target}${ext}`;
}

async function download(url, dest) {
  return new Promise((resolve, reject) => {
    const follow = (url, redirects = 0) => {
      if (redirects > 5) {
        reject(new Error("Too many redirects"));
        return;
      }

      https.get(url, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          follow(res.headers.location, redirects + 1);
          return;
        }

        if (res.statusCode !== 200) {
          reject(new Error(`Download failed: HTTP ${res.statusCode} from ${url}`));
          return;
        }

        const file = fs.createWriteStream(dest);
        res.pipe(file);
        file.on("finish", () => {
          file.close(resolve);
        });
      }).on("error", reject);
    };

    follow(url);
  });
}

async function main() {
  const target = getPlatformTarget();
  const binaryName = getBinaryName();
  const binDir = path.join(__dirname);
  const binaryPath = path.join(binDir, binaryName);

  // Skip if binary already exists (e.g., local dev)
  if (fs.existsSync(binaryPath)) {
    console.log(`${PACKAGE} binary already exists, skipping download.`);
    return;
  }

  const url = getDownloadUrl(target);
  const archiveExt = process.platform === "win32" ? ".zip" : ".tar.gz";
  const archivePath = path.join(binDir, `download${archiveExt}`);

  console.log(`Downloading ${PACKAGE} v${VERSION} for ${target}...`);
  console.log(`  URL: ${url}`);

  try {
    await download(url, archivePath);

    // Extract
    if (process.platform === "win32") {
      // Use PowerShell to extract zip on Windows
      execSync(
        `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${binDir}' -Force"`,
        { stdio: "inherit" }
      );
    } else {
      execSync(`tar -xzf "${archivePath}" -C "${binDir}"`, {
        stdio: "inherit",
      });
    }

    // Ensure binary is executable
    if (process.platform !== "win32") {
      fs.chmodSync(binaryPath, 0o755);
    }

    // Cleanup archive
    fs.unlinkSync(archivePath);

    console.log(`${PACKAGE} v${VERSION} installed successfully.`);
  } catch (err) {
    console.warn(`Failed to download ${PACKAGE}: ${err.message}`);
    console.warn(
      "You can install it manually from: " +
        `https://github.com/${REPO}/releases`
    );
    // Don't fail the install — the integration will warn if the binary is missing
  }
}

main();
