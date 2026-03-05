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
  --max-errors <n>          Maximum number of errors (exact cap, output shows truncation)
  --include <glob>          Only check files matching pattern (repeatable)
  --exclude <glob>          Skip files matching pattern (repeatable)
  --no-sitemap-check        Skip sitemap.xml checks
  --check-assets            Enable asset reference checking
  --check-structured-data   Enable JSON-LD validation
  --check-security          Enable security heuristic checks
  --check-duplicates        Enable content duplicate detection
  --page-overview           Show page properties overview (no checks)
  --update-baseline         Generate/update baseline file from current findings
  --baseline <path>         Path to baseline file [default: .astro-post-audit-baseline]
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

Create a `rules.toml` in your project root (also auto-discovered as `.astro-post-audit.toml`). All fields are optional — only set what you want to override. Copy-paste the full reference below:

```toml
# ── Site ────────────────────────────────────────────────────────────────
[site]
# Base URL, also settable via --site. Used for canonical/sitemap checks.
# base_url = "https://example.com"

# ── Filters ─────────────────────────────────────────────────────────────
[filters]
# Glob patterns to exclude files from all checks (merged with CLI --exclude).
# Typical use: error pages, drafts, or generated pages without SEO relevance.
# exclude = ["404.html", "drafts/**", "preview/**"]
# Only check files matching these patterns (merged with CLI --include).
# include = []

# ── URL Normalization ───────────────────────────────────────────────────
[url_normalization]
trailing_slash = "always"   # "always" | "never" | "ignore" (default: always)
index_html = "forbid"       # "forbid" | "allow" (default: forbid)

# ── Canonical ───────────────────────────────────────────────────────────
[canonical]
require = true              # Every page needs a canonical tag (default: true)
absolute = true             # Canonical must be absolute URL (default: true)
same_origin = true          # Canonical must point to same origin (default: true)
self_reference = false      # Canonical must point to the page itself (default: false)

# ── Robots Meta ─────────────────────────────────────────────────────────
[robots_meta]
allow_noindex = true        # Don't warn on noindex pages (default: true)
fail_if_noindex = false     # Error if any page has noindex (default: false)

# ── Internal Links ──────────────────────────────────────────────────────
[links]
check_internal = true                # Check internal links resolve (default: true)
fail_on_broken = true                # Broken internal links are errors (default: true)
forbid_query_params_internal = true  # Warn on ?query in internal links (default: true)
check_fragments = false              # Validate #fragment targets exist (default: false)
detect_orphan_pages = false          # Warn about pages with no incoming links (default: false)
check_mixed_content = true           # Warn on http:// in internal links (default: true)

# ── Sitemap ─────────────────────────────────────────────────────────────
[sitemap]
require = false                          # sitemap.xml must exist (default: false)
canonical_must_be_in_sitemap = true      # Canonicals should be in sitemap (default: true)
forbid_noncanonical_in_sitemap = false   # Sitemap must not contain non-canonical URLs (default: false)
entries_must_exist_in_dist = true         # Sitemap URLs must match dist/ pages (default: true)

# ── robots.txt ──────────────────────────────────────────────────────────
[robots_txt]
require = false              # robots.txt must exist (default: false)
require_sitemap_link = false # robots.txt must link to sitemap (default: false)

# ── HTML Basics ─────────────────────────────────────────────────────────
[html_basics]
lang_attr_required = true              # <html lang> required (default: true)
title_required = true                  # <title> required (default: true)
meta_description_required = false      # <meta name="description"> required (default: false)
viewport_required = true               # <meta name="viewport"> required (default: true)
title_max_length = 60                  # Warn if title exceeds this length (default: 60)
meta_description_max_length = 160      # Warn if description exceeds this (default: 160)

# ── Headings ────────────────────────────────────────────────────────────
[headings]
require_h1 = true   # Page must have an <h1> (default: true)
single_h1 = true    # Only one <h1> per page (default: true)
no_skip = false      # No heading level gaps, e.g. h2→h4 (default: false)

# ── Accessibility ───────────────────────────────────────────────────────
[a11y]
img_alt_required = true              # <img> must have alt attribute (default: true)
allow_decorative_images = true       # role="presentation" / aria-hidden exempts alt (default: true)
a_accessible_name_required = true    # <a> must have accessible name (default: true)
button_name_required = true          # <button> must have accessible name (default: true)
label_for_required = true            # Form controls need associated <label> (default: true)
warn_generic_link_text = true        # Warn on "click here", "mehr" etc. (default: true)
aria_hidden_focusable_check = true   # Warn if aria-hidden on focusable element (default: true)
require_skip_link = false            # Require skip navigation link (default: false)

# ── Assets ──────────────────────────────────────────────────────────────
[assets]
# Enable via --check-assets or set here.
check_broken_assets = false          # Check img/src, script/src, link/href exist (default: false)
check_image_dimensions = false       # Warn if <img> missing width/height (default: false)
# max_image_size_kb = 500            # Warn if image file exceeds size (off by default)
# max_js_size_kb = 300               # Warn if JS file exceeds size (off by default)
# max_css_size_kb = 100              # Warn if CSS file exceeds size (off by default)
require_hashed_filenames = false     # Warn if assets lack cache-busting hash (default: false)

# ── Open Graph ──────────────────────────────────────────────────────────
[opengraph]
require_og_title = false        # Require og:title (default: false)
require_og_description = false  # Require og:description (default: false)
require_og_image = false        # Require og:image (default: false)
require_twitter_card = false    # Require twitter:card (default: false)

# ── Structured Data (JSON-LD) ──────────────────────────────────────────
[structured_data]
# Enable via --check-structured-data or set here.
check_json_ld = false    # Validate JSON-LD syntax + semantics (default: false)
require_json_ld = false  # Every page must have JSON-LD (default: false)

# ── Hreflang ────────────────────────────────────────────────────────────
[hreflang]
check_hreflang = false          # Enable hreflang checks (default: false)
require_x_default = false       # Require x-default hreflang (default: false)
require_self_reference = false  # Hreflang must include self-reference (default: false)
require_reciprocal = false      # Hreflang links must be reciprocal (default: false)

# ── Security ────────────────────────────────────────────────────────────
[security]
# Enable via --check-security or set here.
check_target_blank = true    # Warn target="_blank" without rel="noopener" (default: true)
check_mixed_content = true   # Warn http:// resources on pages (default: true)
warn_inline_scripts = false  # Warn on inline <script> tags (default: false)

# ── Content Quality ─────────────────────────────────────────────────────
[content_quality]
# Enable via --check-duplicates or set here.
detect_duplicate_titles = false        # Warn if multiple pages share same title (default: false)
detect_duplicate_descriptions = false  # Warn if multiple pages share same description (default: false)
detect_duplicate_h1 = false            # Warn if multiple pages share same H1 (default: false)
detect_duplicate_pages = false         # Warn if pages have identical content hash (default: false)

# ── Severity Overrides ──────────────────────────────────────────────────
[severity]
# Override severity per rule ID: "error" | "warning" | "info" | "off"
# "canonical/missing" = "warning"    # downgrade to warning
# "links/orphan-page" = "info"       # downgrade to informational
# "html/title-too-long" = "off"      # suppress entirely
# "a11y/img-alt-missing" = "error"   # upgrade to error
```

### Severity Overrides

The `[severity]` section lets you reclassify any rule per project:

```toml
[severity]
"links/orphan-page" = "info"       # downgrade to informational
"html/title-too-long" = "off"      # suppress entirely
"a11y/img-alt-missing" = "error"   # upgrade to error
```

Supported levels: `error`, `warning`, `info`, `off`. Applied after all checks, before `--max-errors` cap.

### Baseline / Ignore

When adopting astro-post-audit in an existing project, you can baseline current findings so they don't block CI:

```bash
# Generate baseline from current state
astro-post-audit dist --update-baseline

# Subsequent runs suppress baselined findings
astro-post-audit dist --strict
```

The baseline file (`.astro-post-audit-baseline`) is a tab-separated list of `rule_id<TAB>file_path` entries. Commit it to your repo and fix issues incrementally. The report shows how many findings were suppressed by the baseline.

## What it checks

### SEO / Index Signals
- Canonical tag: present, unique, absolute, same-origin, self-referencing
- Robots meta: noindex detection
- URL normalization: trailing slash / index.html consistency

### Internal Links
- Broken link detection (target not in dist/)
- Query parameter detection on internal links
- Fragment validation (`#id` targets exist)
- Orphan page detection

### Sitemap
- Cross-reference canonical URLs with sitemap entries (normalized comparison)
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
- Semantic validation: `@context` plausibility, `@type` presence
- Type-specific required properties (Article → headline, Organization → name, etc.)
- `@graph` array support

### Open Graph
- `og:title`, `og:description`, `og:image`, `twitter:card` (all optional)

### Security
- `target="_blank"` without `rel="noopener"`
- Mixed content detection (http in https pages)

### Assets
- Broken asset references (img, script, link)
- Image dimension attributes (width/height)
- Hashed filename detection for cache-busting

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
