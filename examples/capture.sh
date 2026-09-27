#!/bin/sh
# Regenerates the terminal captures in examples/ from the test fixtures.
# The site (site/src/showcase.ts) renders these files; run this after changes
# to the report output:  cargo build --release && sh examples/capture.sh
#
# `script` (BSD/macOS syntax) gives the binary a terminal, so the report
# keeps its colours.
# The config JSON is what the Astro integration passes on stdin.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/target/release/astro-post-audit"
out="$root/examples"
fixtures="$root/tests/fixtures"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# capture <file> <dist> <config-json> [stdout|stderr]
capture() {
  printf '%s' "$3" > "$tmp/config.json"
  if [ "${4:-stdout}" = stderr ]; then
    redirect='2>&1 >/dev/null'
  else
    redirect='2>/dev/null'
  fi
  script -q "$tmp/raw" sh -c "\"$bin\" \"$2\" --config-stdin < \"$tmp/config.json\" $redirect" </dev/null >/dev/null || true
  # Drop the pty's leading ^D/backspaces and CRLF line endings.
  perl -0pe 's/\A\^D\x08\x08//; s/\r\n/\n/g; s/\A\n+//; s/\n+\z/\n/' "$tmp/raw" > "$out/$1"
}

site='"site":{"base_url":"https://example.com"}'

# A page with many problems, audited with the defaults.
capture report.ansi "$fixtures/bad" "{$site,\"progress\":false}"

# A clean three-page site: no errors, no warnings.
capture check.ansi "$fixtures/good" "{$site,\"progress\":false}"

# Baseline: record the clean site, add one page, report only what is new.
cp -R "$fixtures/good" "$tmp/dist"
printf '{%s,"progress":false,"baseline":"%s","write_baseline":true}' "$site" "$tmp/baseline.json" \
  | "$bin" "$tmp/dist" --config-stdin >/dev/null 2>&1
cp "$fixtures/edge-cases/long-title.html" "$tmp/dist/long-title.html"
capture baseline.ansi "$tmp/dist" "{$site,\"progress\":false,\"baseline\":\"$tmp/baseline.json\"}"

# progress: 'verbose' prints one line per check on stderr.
capture progress.ansi "$fixtures/edge-cases" "{$site,\"progress_verbose\":true}" stderr
