# astro-post-audit

Fast post-build auditor for Astro sites. Checks SEO signals, internal link consistency, and lightweight WCAG heuristics against your `dist/` output. No browser, no network -- runs in <1s on typical sites.

## Installation

```bash
npm i -D astro-post-audit
```

## Usage

Add to your build pipeline:

```json
{
  "scripts": {
    "build": "astro build && astro-post-audit dist --site https://example.com"
  }
}
```

### CLI

```
astro-post-audit [dist_path] [OPTIONS]

Arguments:
  [dist_path]  Path to the dist/ directory [default: dist]

Options:
  --site <url>         Base URL (for URL normalization + canonical checks)
  --strict             Treat warnings as errors (exit 1)
  --format <fmt>       Output format: text | json [default: text]
  --config <path>      Path to rules.toml config file
  --max-errors <n>     Stop after n errors
  --include <glob>     Only check files matching pattern (repeatable)
  --exclude <glob>     Skip files matching pattern (repeatable)
  --no-sitemap-check   Skip sitemap.xml checks
  -V, --version        Print version
  -h, --help           Print help
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
