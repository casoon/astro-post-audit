# astro-post-audit

Fast post-build auditor for Astro sites. Checks SEO signals, internal link consistency, and lightweight WCAG heuristics against your `dist/` output. No browser, no network — runs in <1s on typical sites.

## Migrating to 0.3.0

Version 0.3.0 enables several new checks **by default** that were previously opt-in or did not exist:

| New default check | Rule ID | Effect |
|-------------------|---------|--------|
| Landmark structure | `a11y/missing-main` | **Error** if page has no `<main>` |
| Duplicate IDs | `a11y/duplicate-id` | **Error** on duplicate `id` attributes |
| ARIA roles | `a11y/invalid-role` | **Error** on invalid `role=` values |
| Image dimensions | `images/missing-dimensions` | **Error** on `<img>` without `width`/`height` |
| Lazy loading | `images/missing-lazy` | **Warning** on images below the fold without `loading="lazy"` |
| Absolute OG image | `opengraph/image-not-absolute` | **Error** when `og:image` is not an absolute URL |
| Twitter card values | `opengraph/twitter-card-invalid` | **Error** on invalid `twitter:card` values |

**Recommended upgrade path:** run the audit once with `failOn: 'never'` to see what fires, fix findings incrementally, then tighten `failOn` again. Or use `writeBaseline: true` to adopt all current findings as a baseline before upgrading.

To silence a new default check on existing sites, set it explicitly in `rules`:

```js
postAudit({
  rules: {
    a11y: { check_landmarks: false },
    images: { check_missing_dimensions: false, warn_missing_lazy: false },
    opengraph: { og_image_absolute_url: false, twitter_card_valid_values: false },
  },
})
```

## Installation

```bash
npm i -D @casoon/astro-post-audit
```

## Setup

```js
// astro.config.mjs
import { defineConfig } from 'astro/config';
import postAudit from '@casoon/astro-post-audit';

export default defineConfig({
  site: 'https://example.com',
  integrations: [postAudit()],
});
```

That's it. The audit runs automatically after every `astro build`.

> **Note:** If you use `@astrojs/sitemap`, make sure `postAudit()` comes **after** `sitemap()` in the integrations array. Both plugins use the `astro:build:done` hook and run in array order — the sitemap file needs to exist before the audit can check it.

## Skipping the audit

Set the `SKIP_AUDIT` environment variable to skip the audit for a single build. Useful for quick dev builds when external link checking would slow things down:

```bash
SKIP_AUDIT=1 astro build
```

Or add a dedicated script to your `package.json`:

```json
{
  "scripts": {
    "build": "astro build",
    "build:fast": "SKIP_AUDIT=1 astro build"
  }
}
```

Then run `npm run build:fast` (or `pnpm build:fast`) when you want to skip the audit.

You can also disable the audit permanently via config: `postAudit({ disable: true })`.

## Presets

Use a preset to start with a predefined configuration. Individual `rules` override preset defaults.

```js
postAudit({ preset: 'standard' })

// Preset + custom overrides
postAudit({
  preset: 'seo',
  rules: {
    headings: { no_skip: true },   // add a rule on top of the preset
  },
})
```

| Preset | What it enables |
|--------|----------------|
| `standard` | Comprehensive quality checks without aggressive extras. Canonical self-reference, heading gaps, meta description, Open Graph (title/description/image + absolute image URL + twitter card validation), a11y (skip link, img alt, button/label names, landmark structure, duplicate IDs, ARIA roles), image dimensions/lazy loading, fragment validation, sitemap, security (target-blank), hreflang, assets, JSON-LD, content quality (duplicate titles/descriptions/H1). Warnings stay warnings. |
| `strict` | Everything in `standard` plus orphan detection, extended robots.txt (disallow-all, crawl-delay), inline-script warnings, Twitter Card, OG type/url requirements, structured data property completeness, i18n audit, crawl budget, render blocking, privacy/security, structured data graph. Sets `strict: true` (warnings become errors). |
| `production` | Alias for `strict`. |
| `seo` | SEO signals only — canonical (self-reference, clusters), html basics (lang, title, meta description, viewport, length limits), Open Graph (all four tags), JSON-LD syntax, sitemap. |
| `accessibility` | WCAG heuristics — lang attr, title, viewport, heading hierarchy (H1 required, single, no gaps), full a11y ruleset (img alt, link/button names, form labels, generic link text, aria-hidden, skip link, landmark structure, duplicate IDs, ARIA roles), fragment validation. |
| `performance` | Static performance signals — broken asset references, missing image dimensions (CLS), lazy loading, srcset hints, hashed filenames, render blocking scripts. |
| `relaxed` | Core SEO and link checks only. No heading gaps, no Open Graph, no structured data, no content quality. Broken links are warnings, not errors. Good starting point for existing sites with known issues. |

## Example configurations

A working reference implementation can be found in [casoon/astro-v6-template](https://github.com/casoon/astro-v6-template).

### Simple site

Minimal config for a small personal or marketing site — the `relaxed` preset covers core SEO with lenient settings, then we add a meta description requirement on top.

```js
postAudit({
  preset: 'relaxed',
  failOn: 'errors',
  rules: {
    filters: { exclude: ['404.html'] },
    html_basics: { meta_description_required: true },
    sitemap: { require: true },
  },
})
```

### Blog with Content Collections

The `seo` preset handles canonical, html basics, Open Graph, JSON-LD and sitemap. Source-file hints show MDX paths next to `dist/` findings.

```js
postAudit({
  preset: 'seo',
  failOn: 'errors',
  hints: { sourceFiles: true },
  rules: {
    filters: { exclude: ['404.html', 'blog/index.html'] },
    headings: { no_skip: true },
    content_quality: {
      detect_duplicate_titles: true,
      detect_duplicate_descriptions: true,
    },
  },
})
```

### Multilingual site (hreflang)

The `standard` preset already includes hreflang (self-reference, x-default, reciprocal). Only exclusions and `failOn` need to be set.

```js
postAudit({
  preset: 'standard',
  failOn: 'errors',
  rules: {
    filters: { exclude: ['404.html'] },
  },
})
```

### Accessibility-focused

The `accessibility` preset covers the full WCAG heuristic ruleset. Add `seo` rules on top if needed.

```js
postAudit({
  preset: 'accessibility',
  failOn: 'errors',
  rules: {
    filters: { exclude: ['404.html'] },
    html_basics: { meta_description_required: true },
    sitemap: { require: true },
  },
})
```

### Strict production gate

`production` (= `strict`) enables everything. Only additional opt-in checks and output config need to be specified.

```js
postAudit({
  preset: 'production',
  failOn: 'errors',
  maxWarnings: 0,
  hints: { sourceFiles: true },
  reports: {
    json: 'audit-report.json',
    sarif: 'audit.sarif',
  },
  rules: {
    filters: { exclude: ['404.html'] },
    external_links: { enabled: true, fail_on_broken: true },
  },
})
```

### AI-optimised content

Enable AI visibility checks for content that should surface well in LLM-generated answers. Useful for documentation, blogs, and product pages targeting AI-assisted search.

```js
postAudit({
  preset: 'seo',
  failOn: 'errors',
  aiVisibility: true,
  rules: {
    filters: { exclude: ['404.html'] },
    headings: { no_skip: true },
    structured_data: { check_json_ld: true },
  },
})
```

`aiVisibility: true` enables the opt-in AI visibility module, which checks LLM-readability (word count, lang attribute), citability (og:title, og:description, canonical, author schema), chunk quality (semantic sectioning, heading structure on long pages), and AI bot policy in robots.txt.

### UX heuristics for marketing sites

Enable UX heuristics to catch missing calls-to-action, generic link text, and missing trust signals:

```js
postAudit({
  preset: 'standard',
  failOn: 'errors',
  uxHeuristics: {
    maxLinksPerPage: 60,
    minCtaPerPage: 2,
  },
  rules: {
    filters: { exclude: ['404.html'] },
  },
})
```

Pass `uxHeuristics: true` to use the defaults (`maxLinksPerPage: 80`, `minCtaPerPage: 1`), or pass an object to override them.

### Environment-based gate (dev / staging / prod)

Use plain JavaScript in `astro.config.mjs` — no profile system needed. The `goLive` option automatically uses Astro's `site` as the expected production origin so you don't have to repeat the domain.

```js
const isProd = process.env.DEPLOY_CONTEXT === 'production'

postAudit({
  preset: isProd ? 'production' : 'relaxed',
  failOn: isProd ? 'errors' : 'never',
  maxWarnings: isProd ? 0 : undefined,
  goLive: {
    enabled: isProd,
    forbiddenDomains: ['staging.example.com', 'localhost'],
  },
})
```

When `goLive.enabled` is `true`, the integration verifies that the expected production origin matches:
1. `goLive.expectedSite` — only needed when the go-live target differs from Astro's `site` config
2. `Astro config.site` — used automatically otherwise

If neither is set and `goLive.enabled` is `true`, the audit fails with a config error.

Environment selection always belongs in `astro.config.mjs` via normal JavaScript — the plugin does not have a built-in profile system. If you find yourself duplicating a lot of config across environments, a profile system may be worth revisiting in a future release.

## Production rollout

The new dist-only audits (`i18n_audit`, `crawl_budget`, `render_blocking`, `privacy_security`, `structured_data_graph`) are intentionally heuristic. They are useful in production, but best rolled out in two steps.

### Step 1: Recommended baseline (warn-first)

```js
postAudit({
  failOn: 'never',
  reports: { json: 'audit-report.json' },
  rules: {
    i18n_audit: { enabled: true },
    crawl_budget: { enabled: true },
    render_blocking: { enabled: true },
    privacy_security: { enabled: true },
    structured_data_graph: { enabled: true },
    severity: {
      'render-blocking/missing-style-preload': 'info',
      'privacy-security/third-party-domains': 'info',
      'crawl-budget/noindex-with-internal-demand': 'info',
    },
  },
})
```

### Step 2: Strict gate (after tuning)

```js
postAudit({
  failOn: 'errors',
  maxErrors: 50,
  rules: {
    i18n_audit: { enabled: true },
    crawl_budget: { enabled: true },
    render_blocking: { enabled: true },
    privacy_security: { enabled: true },
    structured_data_graph: { enabled: true },
    severity: {
      'privacy-security/missing-sri-script': 'error',
      'privacy-security/missing-sri-stylesheet': 'error',
      'structured-data-graph/type-conflict': 'error',
      'crawl-budget/redirect-target-missing': 'error',
    },
  },
})
```

## Configuration

All options are optional. Your editor provides autocomplete with descriptions and defaults for every field.

```js
postAudit({
  preset: 'standard',        // Apply a predefined config (see Presets)
  failOn: 'errors',          // Fail the build on errors (or 'warnings' / 'never')
  maxErrors: 20,             // Stop after 20 errors
  reports: {                 // Write report files (multiple formats at once)
    json: 'audit-report.json',
    markdown: 'audit-summary.md',
    sarif: 'audit.sarif',
  },
  benchmark: true,           // Print per-check timing breakdown
  pageOverview: true,        // Show page properties overview instead of checks
  rules: {
    filters: { exclude: ['404.html', 'drafts/**'] },
    canonical: { self_reference: true },
    a11y: { require_skip_link: true },
    assets: { check_broken_assets: true, check_image_dimensions: true },
    structured_data: { check_json_ld: true },
    security: { check_target_blank: true },
    content_quality: { detect_duplicate_titles: true },
    opengraph: { require_og_title: true, require_og_image: true },
    external_links: { enabled: true, timeout_ms: 5000 },
    headings: { no_skip: true },
    severity: {
      'html/title-too-long': 'off',
      'a11y/img-alt': 'error',
    },
  },
})
```

### Top-level options reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `preset` | `'standard' \| 'strict' \| 'production' \| 'seo' \| 'accessibility' \| 'performance' \| 'relaxed'` | — | Apply a predefined config before your `rules` overrides. See [Presets](#presets). |
| `strict` | `boolean` | `false` | Treat warnings as errors (exit code 1). |
| `throwOnError` | `boolean` | `false` | Throw an error (fail the build) when the audit finds issues. |
| `failOn` | `'errors' \| 'warnings' \| 'never'` | — | Fail the build on errors only (`'errors'`), on any finding (`'warnings'`), or never. Implies `throwOnError`. |
| `maxErrors` | `number` | — | Truncate output after this many errors. |
| `maxWarnings` | `number` | — | Fail the build if the warning count exceeds this number. |
| `site` | `string` | auto | Base URL — auto-detected from Astro's `site` config. |
| `reports` | `ReportsConfig` | — | Write report files. See [Report files](#report-files). |
| `output` | `string` | — | Write a JSON report to this path. Legacy alias for `reports.json`. |
| `baseline` | `string` | — | Path to a baseline file. Only new findings since the baseline are reported. |
| `writeBaseline` | `boolean` | `false` | Write current findings as the new baseline and exit 0. Run once to adopt the plugin on a site with existing issues. |
| `hints.sourceFiles` | `boolean` | `false` | Show likely source file paths (e.g. `src/content/blog/post.mdx`) next to `dist/` findings. Heuristic — may not always match. |
| `groups` | `GroupsConfig` | — | Enable rule groups: `seo`, `a11y`, `links`, `performance`, `privacy`. `true` enables the group, `"warn"` enables but downgrades all findings to warnings. |
| `goLive` | `GoLiveConfig` | — | Production readiness gate. See [Go-live gate](#go-live-gate). |
| `pageOverview` | `boolean` | `false` | Print a page properties table (title, description, canonical, OG, H1, lang, JSON-LD) instead of running checks. |
| `benchmark` | `boolean` | `false` | Print per-check timing breakdown. |
| `aiVisibility` | `boolean` | `false` | Enable AI visibility checks (LLM-readability, citability, chunk quality). See [AI visibility](#ai-visibility). |
| `uxHeuristics` | `boolean \| { maxLinksPerPage?: number, minCtaPerPage?: number }` | `false` | Enable UX heuristic checks (CTAs, generic link text, trust signals). See [UX heuristics](#ux-heuristics). |
| `disable` | `boolean` | `false` | Disable the integration entirely. |
| `rules` | `RulesConfig` | — | Inline check configuration — see full reference below. |

#### Baseline workflow

Use `baseline` + `writeBaseline` to adopt the plugin on a site that already has findings:

```js
// Step 1: write the current state as a baseline (run once)
postAudit({ writeBaseline: true, baseline: '.audit-baseline.json' })

// Step 2: from now on, only new findings are reported
postAudit({ baseline: '.audit-baseline.json' })
```

Commit `.audit-baseline.json` to version control. Delete entries from it to re-enable specific checks.

#### Groups shorthand

```js
postAudit({
  groups: {
    seo: true,           // enable all SEO rules
    a11y: 'warn',        // enable a11y rules but never block the build
    performance: true,
  },
})
```

#### Go-live gate

The `goLive` option adds a set of production-readiness checks that run only when `enabled: true`. These checks catch staging/dev leftovers before a build goes to production.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `goLive.enabled` | `boolean` | `false` | Enable go-live checks. |
| `goLive.expectedSite` | `string` | auto | Expected production origin. Defaults to Astro's `site` config. Only set this when the go-live target differs from Astro `site`. |
| `goLive.forbiddenDomains` | `string[]` | `[]` | Domains that must not appear in canonical URLs, sitemaps, OG tags, or absolute links. |

**What it checks (rule IDs prefixed with `golive/`):**

| Rule ID | Description |
|---------|-------------|
| `golive/noindex` | Page has a `noindex` robots directive |
| `golive/canonical-origin` | Canonical URL uses the wrong origin |
| `golive/og-origin` | `og:url` or `og:image` uses the wrong origin |
| `golive/sitemap-origin` | Sitemap entry uses the wrong origin |
| `golive/forbidden-domain` | Absolute link, script, canonical, or sitemap entry contains a forbidden domain |
| `golive/robots-blocked` | `robots.txt` globally blocks crawlers with `Disallow: /` |
| `golive/config-missing-site` | `goLive.enabled` is `true` but no expected site could be resolved |

All go-live findings are errors (exit code 1). They cannot be downgraded via `severity` overrides — they are explicit production gates.

### Full rules reference

All fields are optional — shown here with their defaults.

```js
rules: {
  // Site settings
  site: {
    base_url: undefined,               // Auto-detected from Astro's `site` config
  },

  // File filters
  filters: {
    include: [],                        // Glob patterns to include
    exclude: [],                        // Glob patterns to exclude (e.g. ["404.html", "drafts/**"])
  },

  // URL normalization
  url_normalization: {
    trailing_slash: 'always',           // 'always' | 'never' | 'ignore'
    index_html: 'forbid',              // 'forbid' | 'allow'
  },

  // Canonical tag checks
  canonical: {
    require: true,                      // Every page must have a canonical tag
    absolute: true,                     // Canonical URL must be absolute
    same_origin: true,                  // Must point to same origin as site
    self_reference: false,              // Must be a self-referencing canonical
    detect_clusters: true,              // Warn when multiple pages share the same canonical
  },

  // Robots meta
  robots_meta: {
    allow_noindex: true,                // Don't warn on noindex pages
    fail_if_noindex: false,             // Treat noindex as error
  },

  // Internal link checks
  links: {
    check_internal: true,               // Verify internal links resolve
    fail_on_broken: true,               // Broken links are errors (not warnings)
    forbid_query_params_internal: true,  // Warn on ?query in internal links
    check_fragments: false,             // Validate #fragment targets exist
    detect_orphan_pages: false,         // Warn about pages with no incoming links
    check_mixed_content: true,          // Warn on http:// in internal links
  },

  // Sitemap cross-reference
  sitemap: {
    require: false,                     // sitemap.xml must exist
    canonical_must_be_in_sitemap: true,  // Canonical URLs should appear in sitemap
    forbid_noncanonical_in_sitemap: false, // Sitemap must not contain non-canonical URLs
    entries_must_exist_in_dist: true,    // Sitemap URLs must correspond to pages
  },

  // robots.txt
  robots_txt: {
    require: false,                     // robots.txt must exist
    require_sitemap_link: false,        // Must contain a sitemap link
    check_disallow_all: true,           // Error when User-agent: * blocks everything
    max_crawl_delay: 10,                // Warn when Crawl-delay exceeds this value (seconds)
    ai_bot_policy: false,               // Check AI bot (GPTBot, ClaudeBot, CCBot …) rules
  },

  // HTML basics
  html_basics: {
    lang_attr_required: true,           // <html lang="..."> required
    title_required: true,               // <title> required and non-empty
    meta_description_required: false,   // <meta name="description"> required
    viewport_required: true,            // <meta name="viewport"> required
    title_max_length: 60,               // Warn if title exceeds this length
    meta_description_max_length: 160,   // Warn if description exceeds this length
  },

  // Heading hierarchy
  headings: {
    require_h1: true,                   // Page must have at least one <h1>
    single_h1: true,                    // Only one <h1> per page
    no_skip: false,                     // No heading level gaps (h2 → h4)
  },

  // Accessibility
  a11y: {
    img_alt_required: true,             // <img> must have alt attribute
    allow_decorative_images: true,      // role="presentation" skips alt check
    a_accessible_name_required: true,   // <a> must have accessible name
    button_name_required: true,         // <button> must have accessible name
    label_for_required: true,           // Form controls need associated <label>
    warn_generic_link_text: true,       // Warn on "click here", "mehr", "weiter"
    aria_hidden_focusable_check: true,  // Warn on aria-hidden on focusable elements
    require_skip_link: false,           // Require skip navigation link
    check_landmarks: true,              // Require proper landmark structure (<main>, <nav>, …)
    check_duplicate_ids: true,          // Error on duplicate id attributes
    check_aria_roles: true,             // Validate role= values against WAI-ARIA spec
  },

  // Asset checks
  assets: {
    check_broken_assets: false,         // Verify img/script/link references
    check_image_dimensions: false,      // Warn on missing width/height (CLS)
    max_image_size_kb: undefined,       // Warn if image exceeds size in KB
    max_js_size_kb: undefined,          // Warn if JS file exceeds size in KB
    max_css_size_kb: undefined,         // Warn if CSS file exceeds size in KB
    require_hashed_filenames: false,    // Warn if filenames lack cache-busting hash
  },

  // Open Graph & Twitter Cards
  opengraph: {
    require_og_title: false,            // Require og:title
    require_og_description: false,      // Require og:description
    require_og_image: false,            // Require og:image
    require_twitter_card: false,        // Require twitter:card
    require_og_type: false,             // Require og:type
    require_og_url: false,              // Require og:url
    og_image_absolute_url: true,        // og:image must be an absolute URL
    require_twitter_image: false,       // Require twitter:image
    twitter_card_valid_values: true,    // Validate twitter:card value against the spec
    og_title_consistency: false,        // Warn when og:title and <title> differ significantly
  },

  // Structured data (JSON-LD)
  structured_data: {
    check_json_ld: false,               // Validate JSON-LD syntax and semantics
    require_json_ld: false,             // Every page must have JSON-LD
    detect_duplicate_types: false,      // Warn on duplicate @type per page
  },

  // Hreflang (multilingual sites)
  hreflang: {
    check_hreflang: false,              // Enable hreflang checks
    require_x_default: false,           // Require x-default entry
    require_self_reference: false,      // Must include self-referencing entry
    require_reciprocal: false,          // Links must be reciprocal (A→B and B→A)
  },

  // Security
  security: {
    check_target_blank: true,           // Warn on target="_blank" without rel="noopener"
    check_mixed_content: true,          // Warn on http:// resource URLs
    warn_inline_scripts: false,         // Warn on inline <script> tags
  },

  // Content quality
  content_quality: {
    detect_duplicate_titles: false,     // Warn on duplicate <title> across pages
    detect_duplicate_descriptions: false, // Warn on duplicate meta descriptions
    detect_duplicate_h1: false,         // Warn on duplicate <h1> across pages
    detect_duplicate_pages: false,      // Warn on identical page content
  },

  // External link checking (network requests)
  external_links: {
    enabled: false,                     // Enable external link checking via HEAD requests
    timeout_ms: 3000,                   // Timeout per request in milliseconds
    max_concurrent: 10,                 // Maximum concurrent requests
    fail_on_broken: false,              // Broken external links are errors (not just warnings)
    allow_domains: [],                  // Only check links to these domains (empty = all)
    block_domains: [],                  // Skip links to these domains
  },

  // Image checks (HTML attribute-level)
  images: {
    check_missing_dimensions: true,     // Error on <img> without width/height (CLS risk)
    warn_missing_lazy: true,            // Warn when images below the fold lack loading="lazy"
    info_missing_srcset: true,          // Info when <img> has no srcset (responsive images)
    format_hints: false,                // Info hint when JPEG/PNG/GIF could use a modern format
  },

  // AI Visibility — opt-in module
  // Enable via top-level aiVisibility option or set enabled: true here
  ai_visibility: {
    enabled: false,                     // Enable AI visibility checks
  },

  // UX Heuristics — opt-in module
  // Enable via top-level uxHeuristics option or set enabled: true here
  ux_heuristics: {
    enabled: false,                     // Enable UX heuristic checks
    max_links_per_page: 80,             // Info when a page exceeds this link count
    min_cta_per_page: 1,                // Warn when a page has fewer CTAs than this
  },

  // Innovative dist-only audits
  i18n_audit: {
    enabled: false,                     // lang/hreflang/canonical consistency by locale route
  },
  crawl_budget: {
    enabled: false,                     // URL variants, duplicate clusters, indexability mismatches
  },
  render_blocking: {
    enabled: false,                     // Sync head scripts, missing preload/preconnect hints
  },
  privacy_security: {
    enabled: false,                     // Third-party domains, SRI/CSP readiness, consent indicators
  },
  structured_data_graph: {
    enabled: false,                     // Cross-page JSON-LD entity consistency and missing internal URLs
  },

  // Override severity per rule ID
  severity: {
    // 'rule-id': 'error' | 'warning' | 'info' | 'off'
  },
}
```

## What it checks

- **Note on signal type** — Most checks are deterministic (broken links, missing tags). The five dist-only audits are heuristic by design and should be tuned via `severity` for your project.
- **SEO** — Canonical tags (including cluster detection), robots meta, URL normalization (trailing slash, index.html)
- **Links** — Broken internal links, query parameters, fragment validation, orphan pages
- **External Links** — HEAD requests to verify external URLs return 2xx, with domain filtering and concurrency control
- **Sitemap** — Cross-reference with canonical URLs, stale entries, missing pages
- **robots.txt** — Existence check, sitemap link, disallow-all detection, crawl-delay threshold, AI bot policy (GPTBot, ClaudeBot, CCBot …)
- **HTML** — `<html lang>`, `<title>`, viewport, meta description, heading hierarchy
- **Accessibility** — img alt, link/button names, form labels (including wrapping labels), generic link text, skip link, aria-hidden on focusable elements, landmark structure (`<main>`, `<nav>`, `<header>`, `<footer>`), duplicate IDs, WAI-ARIA role validation
- **Open Graph** — og:title, og:description, og:image (absolute URL), og:type, og:url, twitter:card (valid values), twitter:image, title consistency
- **Structured Data** — JSON-LD syntax, semantics, duplicate type detection, property completeness (author, datePublished, image, publisher, breadcrumb positions …)
- **Images** — Missing `width`/`height` attributes (CLS), missing `loading="lazy"`, missing `srcset`, modern format hints
- **Hreflang** — Multilingual link validation, x-default, self-reference, reciprocal links
- **Security** — target="_blank" without noopener, mixed content, inline scripts
- **Assets** — Broken references, image dimensions, file size limits, cache-busting hashes
- **Content Quality** — Duplicate titles, descriptions, H1s, near-identical pages
- **I18n Audit** — Consistency between localized routes, `html[lang]`, `hreflang`, and canonical
- **Crawl Budget** — Query/variant URL dilution, duplicate canonical clusters, and indexability mismatches
- **Render Blocking** — Sync `<head>` scripts and missing `preload`/`preconnect` hints for critical resources
- **Privacy/Security (Static)** — Third-party domain inventory, missing SRI, CSP-readiness, consent signals
- **Structured Data Graph** — Cross-page JSON-LD entity conflicts (`@id`, type/name/url) and missing internal entity URLs
- **AI Visibility** *(opt-in)* — LLM readability (word count, lang), citability (OG metadata, canonical, author schema), semantic structure, AI bot policy
- **UX Heuristics** *(opt-in)* — Missing CTAs, generic link text ("click here", "mehr"), missing trust signals (Impressum, Datenschutz, contact), link density, interactive element density

## AI visibility

Enable via `aiVisibility: true` (top-level option) or `rules.ai_visibility.enabled: true`.

| Rule ID | Level | Description |
|---------|-------|-------------|
| `ai-visibility/short-content` | Info | Page word count < 300 — may not be chunked by LLMs |
| `ai-visibility/missing-lang` | Warning | Missing `<html lang>` — LLMs may not infer language |
| `ai-visibility/missing-og-title` | Warning | No `og:title` — reduces citation quality |
| `ai-visibility/missing-og-description` | Info | No `og:description` — reduces citation quality |
| `ai-visibility/missing-canonical` | Warning | No canonical URL — LLMs may attribute content incorrectly |
| `ai-visibility/missing-author-schema` | Info | No `author` in JSON-LD — reduces attribution quality |
| `ai-visibility/no-semantic-sections` | Info | No `<article>` or `<section>` — poor chunk boundaries |
| `ai-visibility/missing-subheadings` | Warning | Long page with no H2/H3 — poor chunk quality |
| `ai-visibility/noindex-page` | Info | Page is noindexed — will not be crawled by AI bots |

## UX heuristics

Enable via `uxHeuristics: true` (top-level option) or `rules.ux_heuristics.enabled: true`. Pass an object to configure thresholds: `uxHeuristics: { maxLinksPerPage: 60, minCtaPerPage: 2 }`.

| Rule ID | Level | Description |
|---------|-------|-------------|
| `ux/no-cta` | Warning | Page has no call-to-action (links/buttons with CTA keywords) |
| `ux/generic-link-text` | Warning | Link with generic text ("click here", "mehr", "weiter", "read more") |
| `ux/no-trust-signals` | Warning | No links to Impressum, Datenschutz, contact, or about pages |
| `ux/high-link-density` | Info | Link count exceeds `max_links_per_page` (default 80) |
| `ux/high-interactive-density` | Info | More than 20 interactive elements on a single page |

## Output

Rich diagnostic output with colored severity markers, location pointers, and Astro-specific help text. The help text references Astro idioms (`BaseHead`, `astro:assets`, Content Collections, `Astro.site`) so fixes are actionable in context:

```
  ──▶ blog/post/index.html
  × error[canonical/missing] Missing canonical tag
    ╰─▶ head
    help: Set `site` in astro.config.mjs and render
          <link rel="canonical" href={new URL(Astro.url.pathname, Astro.site)} />
          in your BaseHead component
  ⚠ warning[a11y/img-alt] <img> missing alt attribute
    ╰─▶ img[src='/photo.jpg']
    help: Add an `alt` prop to <Image>/<Picture> or the <img> tag.
          Use alt="" only for decorative images.

  × 1 error, 1 warning (12 files checked)
```

When the result set is large (more than 20 findings), the report prepends a Top issues summary so you get an at-a-glance view before the per-file detail:

```
  Top issues
       8×  html/meta-description-missing
       3×  canonical/missing
       2×  a11y/img-alt
       1×  links/broken-internal
```

### Report files

Use `reports` to write one or more report files alongside the terminal output. All three formats can be active at the same time:

```js
postAudit({
  reports: {
    json:     'audit-report.json',   // Machine-readable, one finding per entry
    markdown: 'audit-summary.md',    // Human-readable table, useful as CI artifact or PR comment
    sarif:    'audit.sarif',         // SARIF 2.1.0 — consumed by GitHub Code Scanning
  },
})
```

The legacy `output` option (JSON only) remains supported for backwards compatibility.

#### GitHub Code Scanning (SARIF)

Upload the SARIF file with the `github/codeql-action/upload-sarif` action to get inline PR annotations:

```yaml
- name: Build
  run: npm run build

- name: Upload SARIF
  if: always()
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: audit.sarif
```

Set `benchmark: true` to see a per-check timing breakdown — useful for identifying slow checks on large sites.

## License

MIT
