# astro-post-audit

Fast post-build auditor for Astro sites. Checks SEO signals, internal link consistency, and lightweight WCAG heuristics against your `dist/` output. No browser, no network — runs in <1s on typical sites.

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

## Configuration

All options are optional. Your editor provides autocomplete with descriptions and defaults for every field.

```js
postAudit({
  strict: true,              // Treat warnings as errors
  throwOnError: true,        // Fail the build on errors
  maxErrors: 20,             // Stop after 20 errors
  output: 'audit-report.json', // Write JSON report to file
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
      'a11y/img-alt-missing': 'error',
    },
  },
})
```

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

  // Override severity per rule ID
  severity: {
    // 'rule-id': 'error' | 'warning' | 'info' | 'off'
  },
}
```

## What it checks

- **SEO** — Canonical tags (including cluster detection), robots meta, URL normalization (trailing slash, index.html)
- **Links** — Broken internal links, query parameters, fragment validation, orphan pages
- **External Links** — HEAD requests to verify external URLs return 2xx, with domain filtering and concurrency control
- **Sitemap** — Cross-reference with canonical URLs, stale entries
- **HTML** — `<html lang>`, `<title>`, viewport, meta description, heading hierarchy
- **Accessibility** — img alt, link/button names, form labels, generic link text, skip link, aria-hidden
- **Open Graph** — og:title, og:description, og:image, twitter:card
- **Structured Data** — JSON-LD syntax, semantics, type-specific properties
- **Security** — target="_blank" without noopener, mixed content, inline scripts
- **Assets** — Broken references, image dimensions, file size limits, cache-busting hashes
- **Content Quality** — Duplicate titles, descriptions, H1s, pages

## License

MIT
