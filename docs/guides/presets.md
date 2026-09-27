---
title: Presets and speed mode
sidebarLabel: Presets
description: Start from a predefined rule set and override single rules on top. mode switches expensive checks off for local builds.
order: 1
---

## Presets

A preset applies a predefined configuration before your `rules`. Anything in `rules` wins.

```js
postAudit({
  preset: 'seo',
  rules: {
    headings: { no_skip: true }, // added on top of the preset
  },
});
```

| Preset | What it enables |
| --- | --- |
| `standard` | Broad quality checks: canonical self-reference, heading gaps, meta description, Open Graph and Twitter card validation, accessibility, image dimensions and lazy loading, fragments, sitemap, `target="_blank"`, hreflang, assets, JSON-LD, duplicate titles, descriptions and H1s. Warnings stay warnings. |
| `strict` | `standard` plus orphan pages, extended robots.txt, inline-script warnings, OG type/url, structured data completeness, i18n audit, crawl budget, render blocking, privacy/security, structured data graph. Warnings become errors. |
| `production` | Alias for `strict`. |
| `seo` | Canonical, HTML basics, Open Graph, JSON-LD syntax, sitemap. |
| `accessibility` | `lang`, title, viewport, heading hierarchy, the full accessibility rule set, fragments. |
| `performance` | Broken assets, image dimensions, lazy loading, `srcset`, hashed filenames, render-blocking scripts. |
| `relaxed` | Core SEO and link checks only; broken links are warnings. A starting point for sites with known issues. |
| `editorial` | Only the content style module. Combine it with another preset through `rules`. |

## Speed mode

`mode` is independent of the preset. `mode: 'fast'` disables `rules.html_validation` even if a
preset or shared config enables it; that check runs a content-model validation per page and
dominates build time on large sites. `mode: 'full'` (default) leaves the configuration as is.

```js
postAudit({
  preset: 'seo',
  mode: process.env.POST_AUDIT_FAST === '1' ? 'fast' : 'full',
  rules: { html_validation: { enabled: true } },
});
```

## Environments

There is no profile system. Select per environment with plain JavaScript in `astro.config.mjs`:

```js
const isProd = process.env.DEPLOY_CONTEXT === 'production';

postAudit({
  preset: isProd ? 'production' : 'relaxed',
  failOn: isProd ? 'errors' : 'never',
  maxWarnings: isProd ? 0 : undefined,
  goLive: {
    enabled: isProd,
    forbiddenDomains: ['staging.example.com', 'localhost'],
  },
});
```

The go-live gate is described in [Rolling out](../rollout/).
