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

## Configuration

All options are optional. Your editor provides autocomplete with descriptions and defaults for every field.

```js
postAudit({
  strict: true,              // Treat warnings as errors
  throwOnError: true,        // Fail the build on errors
  checkAssets: true,         // Enable asset reference checking
  checkStructuredData: true, // Enable JSON-LD validation
  checkSecurity: true,       // Enable security checks
  checkDuplicates: true,     // Enable duplicate detection
  exclude: ['404.html', 'drafts/**'],
  rules: {
    canonical: { self_reference: true },
    a11y: { require_skip_link: true },
    opengraph: { require_og_title: true, require_og_image: true },
    headings: { no_skip: true },
    severity: {
      'html/title-too-long': 'off',
      'a11y/img-alt-missing': 'error',
    },
  },
})
```

## What it checks

- **SEO** — Canonical tags, robots meta, URL normalization (trailing slash, index.html)
- **Links** — Broken internal links, query parameters, fragment validation, orphan pages
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
