---
title: Overview
description: What astro-post-audit checks, where it stops, and how this documentation is organised.
order: 0
---

astro-post-audit is an Astro integration that audits the finished `dist/` output after every
`astro build`. It reads the generated HTML, sitemap, robots.txt and assets and reports SEO
signals, broken internal links and lightweight WCAG heuristics. The checks run in a native Rust
binary, offline and without a browser.

## What it covers

- **SEO:** canonical tags and clusters, robots meta, URL normalisation, sitemap and robots.txt
  consistency, hreflang, Open Graph and Twitter cards, JSON-LD.
- **Links:** broken internal links, fragments, orphan pages, redirect chains, and opt-in
  external link checks.
- **Accessibility:** image alt text, link and button names, form labels, landmarks, duplicate
  ids, ARIA roles, heading hierarchy. The rules come from the shared `a11y-rules` crates,
  developed in barrierlab.
- **Opt-in modules:** HTML5 conformance, AI visibility, UX heuristics, content style, source
  analysis, CSS architecture, C2PA provenance, GDPR transfers.

## Where it stops

Everything is static analysis of the built files; there is no browser and no layout. By default
there are no computed styles either, so contrast (WCAG 1.4.3) and the other rules that need
them are listed in the JSON report as `capability_missing` instead of being counted as passed.

With `rules.a11y.contrast: true` the tool computes styles from the stylesheets in `dist/` (linked,
inline, local `@import`) and checks text contrast without a browser, in the light scheme and, if a
stylesheet uses `prefers-color-scheme`, in the dark one. A stylesheet that is not on disk, such as
a remote URL, keeps the affected rules `untested` and is listed under `inputs`. Background images,
gradients, geometry (target sizes, spacing) and anything JavaScript builds stay out of reach; use
Lighthouse, axe-core or pa11y for those.

The tool is configured only through `astro.config.mjs`. The Rust binary is an internal detail
of the integration, not a standalone CLI.

## How the docs are organised

- **Getting started:** install the integration and run a first audit.
- **Guides:** presets, rolling the audit out on an existing site, report files and CI.
- **Reference:** the options of `postAudit()`, the check modules and the parts the package is
  built from.
