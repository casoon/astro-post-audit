---
title: Check modules
sidebarLabel: Checks
description: What each module looks at. Most checks are deterministic; the dist-only audits and the opt-in heuristics should be tuned with severity for your project.
order: 2
---

## Default and preset checks

| Area | Checks |
| --- | --- |
| SEO | Canonical tags and cluster detection, robots meta, URL normalisation (trailing slash, `index.html`) |
| Links | Broken internal links, query parameters, fragments, orphan pages, URL depth |
| Sitemap | Cross-reference with canonical URLs, stale entries, missing pages |
| robots.txt | Existence, sitemap link, disallow-all, crawl-delay, AI bot policy, noindex/Disallow contradictions, sitemap entries blocked by robots |
| HTML | `<html lang>`, `<title>`, viewport, meta description, heading hierarchy |
| Accessibility | Alt text and alt-text quality, link and button names, form labels, generic link text, skip link, `aria-hidden` on focusable elements, landmarks, duplicate ids, ARIA roles |
| Open Graph | `og:title`, `og:description`, `og:image` (absolute URL, existence, dimensions, size), `og:type`, `og:url`, Twitter card |
| Structured data | JSON-LD syntax, semantics, duplicate types, property completeness |
| Images | Missing `width`/`height`, `loading="lazy"`, `srcset`, modern format hints |
| Hreflang | x-default, self-reference, reciprocal links, target existence |
| Security | `target="_blank"` without `noopener`, mixed content, inline scripts |
| Assets | Broken references, file size limits, cache-busting hashes |
| Content quality | Duplicate titles, descriptions and H1s, near-identical pages |

## Dist-only audits (heuristic)

Off by default, enabled by the `strict` and `production` presets or one by one with
`enabled: true`: `i18n_audit` (routes, `html[lang]`, hreflang and canonical agree), `crawl_budget` (query and
variant URL dilution, duplicate canonical clusters), `render_blocking` (sync head scripts,
missing preconnect), `privacy_security` (third-party inventory, SRI, CSP readiness, consent
signals) and `structured_data_graph` (cross-page JSON-LD entity conflicts).

## Opt-in modules

| Module | Enable with | Looks at |
| --- | --- | --- |
| HTML5 conformance | `rules.html_validation.enabled` | Content model, parser, ARIA, attributes, tables, offline |
| External links | `rules.external_links.enabled` | HEAD requests for 2xx, with domain filter and concurrency limit |
| Redirects | `rules.redirects.enabled` | Meta-refresh chains and loops, links to redirect pages |
| JS weight | `rules.js_bloat.enabled` | Local client JavaScript per route |
| Fonts | `rules.fonts.enabled` | Font-loading hints |
| CSS architecture | `rules.css_architecture.enabled` | CSS per route against a limit, median-based outliers |
| Content sync | `rules.content_sync.enabled` | `src/content/` items without a generated page |
| Source analysis | `rules.source_analysis.enabled` | Static Astro/Tailwind class inventory, duplicate signatures, conflicting utilities, oversized components |
| AI visibility | `aiVisibility: true` | Word count, `lang`, citability, semantic sections, AI bot policy, `llms.txt` links |
| UX heuristics | `uxHeuristics: true` | Calls to action, generic link text, trust signals, link and interactive density |
| Content style | `contentStyle: true` or `preset: 'editorial'` | Recurring "reads like AI" patterns in German and English |
| View Transitions | `rules.view_transitions.enabled` | Duplicate `transition:name`, external-link reload hints |
| C2PA | `rules.c2pa.enabled` | Embedded Content Credentials in JPEG, PNG and WebP |
| GDPR | `rules.privacy_security.gdpr` | Google Fonts, YouTube and Maps embeds, public CDNs, external images |

Rule ids, levels and all options per module are listed in the
[README](https://github.com/casoon/astro-post-audit#what-it-checks).
