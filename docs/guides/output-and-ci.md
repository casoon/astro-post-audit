---
title: Output and CI
sidebarLabel: Output and CI
description: Terminal output, JSON, Markdown, SARIF and HTML report files, GitHub Code Scanning, and the stderr diagnostics.
order: 3
---

## Terminal output

The text report groups findings per file with rule id, selector and remedy. It uses colour in
an interactive terminal and plain ASCII when piped or in CI. With 20 or more findings a summary
of the most frequent rules comes first. The [showcase](../../../showcase/) shows real runs.

## Report files

```js
postAudit({
  reports: {
    json: 'audit-report.json',   // one entry per finding
    markdown: 'audit-summary.md', // table for CI artifacts or PR comments
    sarif: 'audit.sarif',         // SARIF 2.1.0 for GitHub Code Scanning
    html: 'audit-report.html',    // standalone HTML report
  },
});
```

All four can be written in the same run. The legacy `output` option writes JSON only.

## The finding model

Since 0.7.0 a finding carries two axes: `outcome` says how certain the statement is (`fail`,
`review`, `pass`, `untested`), `severity` how heavy the problem is (`low` … `critical`). The JSON
report also lists `rule_runs`. Rules the tool cannot serve, such as colour contrast without
`rules.a11y.contrast` or anything that needs layout, appear there as `capability_missing`.

With `rules.a11y.contrast` the report adds `inputs`: one note per stylesheet that could not be
read from `dist/` (`stylesheet_unavailable`: remote URL, missing file, path outside `dist/`) or an
`@import` left unresolved (`stylesheet_unresolved`), with the number of pages affected. While such
a note applies to a page, its contrast findings are `untested`, never `pass`. The text and
Markdown reports show the same notes under "Stylesheets not read".

Accessibility and document rule ids come from `a11y-rules`, so a finding has the same id in
astro-post-audit, auditmysite and LiveAudit, for example `images/alt-missing`.

## GitHub Code Scanning

```yaml
- name: Build
  run: npm run build

- name: Upload SARIF
  if: always()
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: audit.sarif
```

## Diagnostics

Both diagnostics write to stderr, so report output on stdout stays clean.

- `progress`: a single-line progress bar over the check phases. On by default in an
  interactive terminal, silent in CI. `'verbose'` prints each check as its own line with
  finding count and timing instead.
- `debug: true`: resolved configuration, discovery counts and per-check finding counts and
  timings. Replaces the progress bar.
- `benchmark: true`: per-check timing breakdown.
- `color`: `'auto'` (default) styles only an interactive terminal and honours `NO_COLOR`.
  `'always'` keeps colour and Unicode symbols in piped build logs that a colour-capable
  terminal still renders; `'never'` turns styling off.
