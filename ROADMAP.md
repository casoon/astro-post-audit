# astro-post-audit Roadmap

## Phase 1: Foundation (MVP) -- DONE
**Goal:** Core infrastructure, basic checks, CLI working end-to-end.

- [x] Project structure (Rust crate + npm wrapper)
- [x] CLI argument parsing (clap)
- [x] TOML config loading with sane defaults
- [x] HTML discovery (walkdir) + parallel parsing (rayon + scraper)
- [x] URL normalization (trailing slash, index.html, absolute URLs)
- [x] Site index (route <-> file mapping)
- [x] SEO checks: canonical (missing/multiple/empty/not-absolute/cross-origin)
- [x] SEO checks: robots meta (noindex detection)
- [x] Internal link checking (broken links, query params)
- [x] HTML basics: lang, title, viewport, meta description
- [x] A11y: img alt, link accessible name, button name, form labels
- [x] Heading checks: require h1, single h1, no skip
- [x] Sitemap parsing + cross-checks
- [x] Text + JSON reporter with file grouping
- [x] Exit codes (0/1/2)
- [x] Unit tests for URL normalization (8 tests)
- [x] Test fixtures (good + bad HTML pages)

## Phase 2: Extended Checks -- DONE
**Goal:** Full coverage of SEO, assets, security, content quality.

- [x] Asset checks: broken img/script/link references, image dimensions (CLS)
- [x] Asset size heuristics: configurable max KB for images, JS, CSS
- [x] OpenGraph + Twitter Card meta validation
- [x] JSON-LD structured data: existence check + JSON syntax validation
- [x] Hreflang: x-default, self-reference, reciprocal link checks
- [x] Security: target="_blank" noopener, mixed content, inline scripts
- [x] Content quality: duplicate titles, descriptions, H1s, pages (hash-based)
- [x] robots.txt: existence + sitemap directive
- [x] Fragment target validation (href="#id" -> id exists?)
- [x] Orphan page detection (pages with no incoming internal links)
- [x] Mixed content detection on internal links (http:// vs https://)
- [x] aria-hidden on focusable elements check
- [x] Title/description length heuristics (configurable max chars)
- [x] CLI flags: --check-assets, --check-security, --check-duplicates, --check-structured-data
- [x] Summary includes files_checked count

## Phase 3: Polish & Testing -- DONE
**Goal:** Robust test suite, edge cases, CI green.

- [x] HTML fixture test suite (edge cases: empty files, whitespace-only, malformed HTML, no doctype)
- [x] Edge-case fixtures: security issues, a11y edge cases, complex headings, opengraph, structured data
- [x] Snapshot tests for text + JSON output
- [x] Integration tests for each check module (57 tests)
- [x] `--max-errors` early stop optimization (skips remaining check modules once cap reached)
- [x] Include/exclude glob filter tests
- [x] Config file auto-discovery (rules.toml / .astro-post-audit.toml in CWD or dist parent)
- [x] Bug fix: assets/img-dimensions check was always-on (hardcoded `if true`) regardless of config
- [x] Code review: 15 fixes across P0/P1/P2 priorities (see below)
- [ ] CI workflow green on all 3 OS

### Phase 3b: Code Review Fixes -- DONE
**P0 — Correctness:**
- [x] UTF-8 panic: `&title[..50]` → `truncate_str()` with `chars()` iterator
- [x] `..` path resolution: `collapse_dots()` in normalize + resolve_href
- [x] Orphan-check used `Default::default()` instead of user's `UrlNormalizationConfig`
- [x] Sitemap canonical comparison now normalizes both sides before comparing
- [x] `.htm` file support in `normalize_path()` and `file_path_to_route()`

**P1 — Performance:**
- [x] `sitemap_urls` Vec → HashSet for O(1) lookups
- [x] Running error counter in `run_check!` macro (replaces linear scan)
- [x] Orphan-check link collection parallelized via `par_iter()`

**P2 — Robustness/UX:**
- [x] Silent file read errors now emit stderr warnings
- [x] Config auto-discovery parse errors now emit stderr warnings
- [x] `process::exit` → `run()` returns `Result<i32>`
- [x] Removed dead `--check-external` CLI flag (no implementation exists)
- [x] One Finding per file in content_quality (was comma-joined paths)
- [x] Symlink-loop protection: `follow_links(false)` in WalkDir

## Phase 4: Packaging & Distribution -- READY
**Goal:** npm install works on all platforms.

- [x] CI workflow: fmt, clippy (-D warnings), test on 3 OS
- [x] Release workflow: 5 targets (mac-x64, mac-arm64, linux-x64, linux-arm64, win-x64)
- [x] npm wrapper: postinstall downloads binary from GitHub Releases
- [x] Version sync (Cargo.toml = package.json = 0.1.0)
- [x] Clippy clean (zero warnings with -D warnings)
- [x] Release binary: 3.0 MB (1.3 MB gzipped), LTO + strip
- [ ] Push to GitHub, verify CI green on all 3 OS
- [ ] Tag v0.1.0 → triggers release workflow → GitHub Release + npm publish

## Phase 5: Advanced Features
**Goal:** Production-ready, opinionated presets, DX polish.

- [ ] Presets: `--preset strict` / `--preset relaxed`
- [ ] Canonical cluster analysis (multiple pages -> same canonical report)
- [ ] Performance: benchmark with 500+ page site, ensure <1s
- [ ] `--fix` suggestions (machine-readable fix hints in JSON)
- [ ] Colored output with miette for rich diagnostics
- [ ] External link checker implementation (async HTTP with reqwest/ureq)
- [ ] Astro config bridge (read site/trailingSlash from astro.config.mjs)

## Phase 6: Ecosystem Integration
**Goal:** Seamless Astro/CI integration.

- [ ] Astro integration package (`@astro-post-audit/astro`)
- [ ] GitHub Action (`astro-post-audit/action`)
- [ ] Markdown report output (for PR comments)
- [ ] SARIF output (for GitHub Code Scanning)
- [ ] Config schema (JSON Schema for IDE autocomplete)

---

## Module Overview

### Core (always on)
| Module | Checks |
|---|---|
| `seo` | canonical, robots meta |
| `links` | broken links, query params, fragments, orphans, mixed content |
| `a11y` | img alt, link/button names, form labels, generic text, aria-hidden |
| `html_basics` | lang, title, viewport, meta description, length heuristics |
| `headings` | require h1, single h1, no skip |
| `sitemap` | canonical cross-ref, stale entries |
| `robots_txt` | existence, sitemap directive |
| `security` | target_blank noopener, mixed content, inline scripts |

### Optional (via flags/config)
| Module | Flag | Checks |
|---|---|---|
| `assets` | `--check-assets` | broken references, dimensions, file sizes |
| `opengraph` | config only | og:title, og:description, og:image, twitter:card |
| `structured_data` | `--check-structured-data` | JSON-LD existence + syntax |
| `hreflang` | config only | x-default, self-ref, reciprocal |
| `content_quality` | `--check-duplicates` | duplicate titles, descriptions, h1, pages |

---

## Severity Matrix

### Errors (exit code 1)
| Rule ID | Description |
|---|---|
| `canonical/missing` | No canonical tag found |
| `canonical/multiple` | More than one canonical tag |
| `canonical/empty` | Canonical href is empty |
| `canonical/not-absolute` | Canonical URL is relative |
| `canonical/cross-origin` | Canonical points to different origin |
| `links/broken` | Internal link target not found |
| `links/query-params` | Internal link contains query parameters |
| `html/lang-missing` | Missing lang attribute |
| `html/title-missing` | Missing title tag |
| `html/title-empty` | Title tag is empty |
| `html/viewport-missing` | Missing viewport meta |
| `headings/no-h1` | No h1 heading |
| `a11y/img-alt` | Image missing alt |
| `a11y/link-name` | Link has no accessible name |
| `a11y/button-name` | Button has no accessible name |
| `a11y/form-label` | Form control has no label |
| `assets/broken` | Broken asset reference |
| `structured-data/empty` | Empty JSON-LD script |
| `structured-data/invalid-json` | Invalid JSON in JSON-LD |

### Warnings (exit code 0, unless --strict)
| Rule ID | Description |
|---|---|
| `canonical/not-self` | Canonical doesn't match page URL |
| `canonical/target-missing` | Canonical target not found |
| `robots/noindex` | Page has noindex |
| `sitemap/*` | Various sitemap cross-check issues |
| `html/title-too-long` | Title exceeds recommended length |
| `html/meta-description-too-long` | Description exceeds recommended length |
| `html/meta-description-missing` | Missing meta description |
| `headings/multiple-h1` | Multiple h1 headings |
| `headings/skip-level` | Heading level skip |
| `a11y/generic-link-text` | Generic link text |
| `a11y/aria-hidden-focusable` | aria-hidden on focusable element |
| `assets/img-dimensions` | Missing width/height (CLS) |
| `assets/large-*` | Oversized assets |
| `opengraph/*` | Missing OG/Twitter meta |
| `hreflang/*` | Hreflang consistency issues |
| `security/*` | Security heuristic findings |
| `content/*` | Duplicate content across pages |
| `links/orphan-page` | Page with no incoming links |
| `links/broken-fragment` | Fragment target not found |
| `links/mixed-content` | HTTP internal link |
| `robots-txt/*` | robots.txt issues |
