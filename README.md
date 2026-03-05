# astro-post-audit

Fast post-build auditor for Astro sites. Checks SEO signals, internal link consistency, and lightweight WCAG heuristics against your `dist/` output. No browser, no network -- runs in <1s on typical sites.

## Installation

```bash
npm i -D @casoon/astro-post-audit
```

## Astro Integration

The recommended way to use astro-post-audit is as an Astro integration. It runs automatically after `astro build` via the `astro:build:done` hook.

```js
// astro.config.mjs
import { defineConfig } from 'astro/config';
import postAudit from '@casoon/astro-post-audit';

export default defineConfig({
  site: 'https://example.com', // auto-detected as --site
  integrations: [
    postAudit({
      strict: true,
      checkAssets: true,
      rules: {
        a11y: { require_skip_link: true },
        canonical: { self_reference: true },
        opengraph: { require_og_title: true, require_og_image: true },
      },
    }),
  ],
});
```

### Integration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `site` | `string` | auto from Astro `site` | Base URL for normalization |
| `strict` | `boolean` | `false` | Treat warnings as errors |
| `format` | `'text' \| 'json'` | `'text'` | Output format |
| `config` | `string` | — | Path to rules.toml (mutually exclusive with `rules`) |
| `rules` | `RulesConfig` | — | Inline rules config (see below) |
| `maxErrors` | `number` | — | Stop after n errors |
| `include` | `string[]` | — | Only check files matching patterns |
| `exclude` | `string[]` | — | Skip files matching patterns |
| `noSitemapCheck` | `boolean` | `false` | Skip sitemap checks |
| `checkAssets` | `boolean` | `false` | Enable asset reference checking |
| `checkStructuredData` | `boolean` | `false` | Enable JSON-LD validation |
| `checkSecurity` | `boolean` | `false` | Enable security checks |
| `checkDuplicates` | `boolean` | `false` | Enable duplicate detection |
| `pageOverview` | `boolean` | `false` | Show overview instead of checks |
| `throwOnError` | `boolean` | `false` | Fail the build on errors |
| `disable` | `boolean` | `false` | Disable the integration |

The `rules` option accepts the same structure as `rules.toml` — all sections (`a11y`, `canonical`, `links`, `security`, etc.) and their fields are optional. See `rules.toml` for all available options.

## CLI Usage

The CLI can also be used standalone (e.g., in CI without Astro):

```json
{
  "scripts": {
    "audit": "astro-post-audit dist --site https://example.com"
  }
}
```

### CLI

```
astro-post-audit [dist_path] [OPTIONS]

Arguments:
  [dist_path]  Path to the dist/ directory [default: dist]

Options:
  --site <url>              Base URL (for URL normalization + canonical checks)
  --strict                  Treat warnings as errors (exit 1)
  --format <fmt>            Output format: text | json [default: text]
  --config <path>           Path to rules.toml config file
  --max-errors <n>          Stop after n errors
  --include <glob>          Only check files matching pattern (repeatable)
  --exclude <glob>          Skip files matching pattern (repeatable)
  --no-sitemap-check        Skip sitemap.xml checks
  --check-assets            Enable asset reference checking
  --check-structured-data   Enable JSON-LD validation
  --check-security          Enable security heuristic checks
  --check-duplicates        Enable content duplicate detection
  --page-overview           Show page properties overview (no checks)
  -V, --version             Print version
  -h, --help                Print help
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed (or only warnings in non-strict mode) |
| 1 | Errors found (or warnings in --strict mode) |
| 2 | Tool/runtime failure (e.g., dist path not found) |

## Configuration

Create a `rules.toml` in your project root:

```toml
[site]
base_url = "https://example.com"

[url_normalization]
trailing_slash = "always"  # always | never | ignore
index_html = "forbid"      # forbid | allow

[canonical]
require = true
absolute = true
same_origin = true
self_reference = false

[links]
check_internal = true
fail_on_broken = true
forbid_query_params_internal = true

[html_basics]
lang_attr_required = true
title_required = true
viewport_required = true

[headings]
require_h1 = true
single_h1 = true

[a11y]
img_alt_required = true
a_accessible_name_required = true
button_name_required = true
label_for_required = true
warn_generic_link_text = true
require_skip_link = false

[structured_data]
check_json_ld = false
require_json_ld = false
```

See `rules.toml` in this repo for all options with documentation.

## What it checks

### SEO / Index Signals
- Canonical tag: present, unique, absolute, same-origin, self-referencing
- Robots meta: noindex detection
- URL normalization: trailing slash / index.html consistency

### Internal Links
- Broken link detection (target not in dist/)
- Query parameter detection on internal links

### Sitemap
- Cross-reference canonical URLs with sitemap entries
- Detect stale sitemap entries

### HTML Basics
- `<html lang>` attribute
- `<title>` tag (present + non-empty)
- Viewport meta tag
- Meta description (optional)

### Accessibility (lightweight, static)
- `<img>` alt attribute (with decorative image exceptions)
- `<a>` accessible name (text, aria-label, aria-labelledby)
- `<button>` accessible name
- Form control labels
- Generic link text detection ("click here", "mehr", etc.)
- Heading hierarchy (h1 presence, single h1, no level skip)
- Skip navigation link (`require_skip_link`)
- `aria-hidden` on focusable elements

### Structured Data
- JSON-LD presence check (`require_json_ld`)
- JSON-LD syntax validation (`check_json_ld`)

### Open Graph
- `og:title`, `og:description`, `og:image`, `twitter:card` (all optional)

### Security
- `target="_blank"` without `rel="noopener"`
- Mixed content detection (http in https pages)

### Content Quality
- Duplicate title, description, H1 detection
- Duplicate page detection (content hash)

## Page Properties Overview

Run `--page-overview` to get an informational overview of all pages without running checks:

```bash
astro-post-audit dist --page-overview --site https://example.com
```

Output shows a table with per-page properties:

```
Page Properties Overview (23 pages)

  File                          Title  Desc  Canon  OG  H1  Lang  LD  Skip  LD Types
  ──────────────────────────────────────────────────────────────────────────────────
  index.html                      ✓     ✓      ✓    ✓   1   de   ✓    ✓    ProfessionalService
  about/index.html                ✓     ✓      ✓    ✗   1   de   ✗    ✓    —

Summary:  Title 23/23 · Desc 22/23 · Canonical 23/23 · OG 20/23 · H1 23/23 · Lang 23/23 · JSON-LD 12/23 · Skip 23/23

JSON-LD Types:  Article ×8 · WebPage ×23 · BreadcrumbList ×23
```

Also supports `--format json` for machine-readable output.

## CI Example

### GitHub Actions

```yaml
- name: Build
  run: npm run build

- name: Audit
  run: npx astro-post-audit dist --site ${{ env.SITE_URL }} --strict --format json
```

## Development

```bash
# Build
cd crates/astro-post-audit
cargo build --release

# Test
cargo test

# Run against a local dist/
cargo run -- ../my-site/dist --site https://example.com
```

## License

MIT
