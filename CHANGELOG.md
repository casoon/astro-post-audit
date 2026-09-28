# Changelog

All notable changes to this project are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Entries before 0.7.0 were
reconstructed from the git history and the README release notes.

## [Unreleased]

## [0.8.0] - 2026-09-28

### Added

- `color` option (`'auto'`, `'always'`, `'never'`) for the text report and progress output, so a
  piped build log can still get colour and Unicode symbols (#56).

### Changed

- Structured data is evaluated by `web-checks` 0.4 (`structured_data`), shared with auditmysite.
  The rule tables follow Google's structured-data docs, so finding counts change:
  - **No longer reported:** Article/BlogPosting/NewsArticle `headline`, Organization/Person
    `name` and WebSite `name`/`url` as `structured-data/missing-property` (Google lists no required
    properties for these types).
  - **Now reported:** missing required properties of Event, Recipe, VideoObject, JobPosting,
    SoftwareApplication, ProfilePage, ItemList and Product (`name` plus one of
    `offers`/`review`/`aggregateRating`) as `structured-data/missing-property`; BreadcrumbList with
    fewer than 2 items or without `item` on a non-last entry; `structured-data/invalid-structure`
    for a non-object root, a non-array `@graph` or an empty array/`@graph`.
  - **Severity changed:** `structured-data/news-article-missing-publisher` High → Medium
    (recommended, not required); `structured-data/local-business-missing-address` Low → Medium
    (required by Google).
  - **Behaviour:** empty values (`""`, `null`, `[]`, `{}`) count as missing; `@context` must be
    exactly `http(s)://schema.org` (also as array entry or `@vocab`) and is inherited into
    `@graph`; top-level arrays are expanded instead of reported as missing context; every `@type`
    entry is assessed, full IRIs included; `structured-data/duplicate-type` counts blocks, not
    repeats inside one block.
- Title and meta description length come from `web-checks` 0.3 and count **characters**, not
  bytes: before, every umlaut counted twice, so a 60-character German title could be reported as
  too long. Whitespace is collapsed as in the browser before counting.
- OpenGraph / Twitter Card presence, `twitter:card` values and the absolute-`og:image` rule come
  from `web-checks` (shared with auditmysite). `opengraph/image-missing` and
  `opengraph/twitter-card-missing` now also fire when the tag exists with empty `content`, like
  the other presence checks already did.
- robots.txt is evaluated by the shared `web-checks` crate instead of a local copy.
- `a11y-*` crates updated to 0.12.0. Accessibility finding texts now come in English from
  `a11y-rules`, and a duplicate `id` is only reported when an IDREF (e.g. `aria-describedby`,
  `for`) points to it, since WCAG 2.2 removed 4.1.1.
- `progress: 'verbose'` prints its per-check lines through runemark instead of raw stderr output.
- runemark updated from 0.1.1 to 0.9.0.

## [0.7.0] - 2026-09-23

### Breaking

- Accessibility and document rules come from `a11y-rules` (a11y-core), shared with auditmysite
  and LiveAudit. Rule ids changed (`images/alt-missing` instead of `a11y/img-alt`); old ids are
  still accepted in `severity` overrides and baseline files and translated with one warning per id.
- Findings carry two axes, `outcome` (`fail`, `review`, `pass`, `untested`) and `severity`
  (`low` … `critical`). The JSON report gained `rule_runs`, which lists rules static HTML cannot
  serve, such as contrast, as `capability_missing`.
- Intel macOS (`darwin-x64`) is no longer built or shipped.

## [0.6.0] - 2026-09-09

### Added

- `mode: 'fast' | 'full'` option. `'fast'` disables `rules.html_validation` regardless of the
  rest of the configuration.

### Fixed

- `links.known_routes` is honoured for cross-page fragment checks.

## [0.5.7] - 2026-09-09

### Added

- CSS architecture check: local and inline CSS per route with payload limits and median-based
  outlier detection (`css-architecture/route-payload`, `css-architecture/route-outlier`).

### Changed

- Tailwind source analysis handles `class:list`, ignores build and dependency directories and
  detects same-axis spacing conflicts.
- Runtime, TypeScript and JSON Schema defaults are aligned and covered by regression tests.
- Render blocking no longer recommends preloading every stylesheet.

## [0.5.6] - 2026-09-04

### Changed

- crates.io publishing moved to a manual maintainer step.

## [0.5.5] - 2026-09-03

### Fixed

- `density_per_1000_words` matches the documented TypeScript and schema value; invalid rule
  types fail early.
- Astro's client-island runtime style no longer produces HTML conformance findings.

## [0.5.4] - 2026-09-03

### Added

- Opt-in C2PA Content Credentials validation (`rules.c2pa`) with `require_trusted`.
- `links.known_routes` to exclude SSR and dynamic routes from broken-link checks.
- Configurable content style thresholds.

## [0.5.2] - 2026-07-29

### Fixed

- Content style text extraction.

## [0.5.0] - 2026-07-29

### Added

- Content style heuristics for recurring "reads like AI" writing patterns (`contentStyle`,
  `preset: 'editorial'`).

## [0.4.6] - 2026-06-23

### Added

- Astro 7 compatibility and end-to-end tests of the integration.

## [0.4.5] - 2026-06-01

### Added

- `progress: 'verbose'`, which prints each check as its own line with finding count and timing.

### Fixed

- The progress bar stays visible after the run.

## [0.4.4] - 2026-06-01

### Fixed

- The progress bar fills up to total/total.

## [0.4.3] - 2026-06-01

### Fixed

- The progress bar is clamped to the terminal width.

## [0.4.2] - 2026-06-01

### Added

- `debug` option with verbose diagnostics on stderr.

## [0.4.1] - 2026-06-01

### Added

- Live progress bar on stderr during the audit.

## [0.4.0] - 2026-06-01

### Added

- New check modules: meta-refresh redirects, client-side JS weight per route, content sync
  (collection items without a page) and HTML5 parse errors.
- Open Graph image existence, dimensions and file size; URL depth limit; robots.txt
  noindex/Disallow contradictions; hreflang target existence; alt-text quality heuristics;
  GDPR third-party transfers.

## [0.3.2] - 2026-05-26

### Fixed

- An empty `Disallow:` no longer counts as disallow-all (RFC 9309).
- Landmark false positives when header or footer sit inside a wrapper `div`.

## [0.3.0] - 2026-05-25

### Added

- Landmark, ARIA and image checks, AI visibility, UX heuristics, extended Open Graph and
  robots.txt checks.
- Go-live gate, several report formats in one run, SSR detection, Astro 7 support.

## [0.2.11] - 2026-05-11

### Added

- Top-issue summary for large result sets and Astro-specific remedies.

### Changed

- Files under `/_astro/` are exempt from hashed-filename checks.

## [0.2.10] - 2026-05-09

### Added

- `standard` preset.

## [0.2.9] - 2026-05-09

### Added

- Presets `seo`, `accessibility`, `performance` and `production`, `maxWarnings` and
  selector-based baselines.

## [0.2.7] - 2026-05-09

### Added

- Markdown and SARIF reports, baselines, source file hints and rule groups.

## [0.2.6] - 2026-03-06

### Added

- Crawl budget, i18n audit, privacy/security, render blocking and structured data graph checks.

## [0.2.5] - 2026-03-06

### Added

- Presets, benchmark output, fix suggestions and a JSON Schema for the options.

## [0.2.4] - 2026-03-05

### Added

- `SKIP_AUDIT` environment variable, canonical cluster detection, external link checking.

### Fixed

- Fragment check false positives with percent-encoded umlauts.

## [0.2.2] - 2026-03-05

### Added

- JSON-LD duplicate type check.

### Removed

- TOML configuration; options are passed as JSON by the integration.

## [0.2.0] - 2026-03-05

### Added

- Astro integration, page overview and skip-link check.

## [0.1.3] - 2026-02-25

### Changed

- Package renamed to `@casoon/astro-post-audit`.
