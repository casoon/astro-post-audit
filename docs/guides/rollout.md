---
title: Rolling out on an existing site
sidebarLabel: Rolling out
description: Adopt the audit on a site with existing findings, tighten it in steps, and gate production builds.
order: 2
---

## Baseline

A baseline records the current findings so only new ones are reported.

```js
// Step 1: write the current state once
postAudit({ writeBaseline: true, baseline: '.audit-baseline.json' });

// Step 2: from now on, only new findings are reported
postAudit({ baseline: '.audit-baseline.json' });
```

Commit `.audit-baseline.json`. Delete entries from it to bring findings back. Baselines written
before 0.7.0 keep working; old rule ids are translated on read.

## Warn first, then gate

The dist-only audits `i18n_audit`, `crawl_budget`, `render_blocking`, `privacy_security` and
`structured_data_graph` are heuristic. Run them without failing the build first and tune single
rules with `severity`:

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
      'privacy-security/third-party-domains': 'info',
      'crawl-budget/noindex-with-internal-demand': 'info',
    },
  },
});
```

Once the report is clean enough, switch to `failOn: 'errors'` and raise the rules you care about
to `'error'`.

## Groups

```js
postAudit({
  groups: {
    seo: true,       // enable all SEO rules
    a11y: 'warn',    // enable, but never block the build
    performance: true,
  },
});
```

Groups: `seo`, `a11y`, `links`, `performance`, `privacy`.

## Go-live gate

`goLive` catches staging leftovers before a production deploy. It runs only when
`enabled: true` and uses Astro's `site` as the expected origin unless `expectedSite` is set.

| Rule id | Finding |
| --- | --- |
| `golive/noindex` | Page has a `noindex` robots directive |
| `golive/canonical-origin` | Canonical URL uses the wrong origin |
| `golive/og-origin` | `og:url` or `og:image` uses the wrong origin |
| `golive/sitemap-origin` | Sitemap entry uses the wrong origin |
| `golive/forbidden-domain` | A link, script, canonical or sitemap entry contains a forbidden domain |
| `golive/robots-blocked` | `robots.txt` blocks all crawlers with `Disallow: /` |
| `golive/config-missing-site` | Enabled, but no expected site could be resolved |

Go-live findings are always errors and cannot be downgraded with `severity`.
