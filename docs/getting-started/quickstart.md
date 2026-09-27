---
title: Quickstart
description: Register the integration, build the site, read the report.
order: 2
---

## Register the integration

```js
// astro.config.mjs
import { defineConfig } from 'astro/config';
import postAudit from '@casoon/astro-post-audit';

export default defineConfig({
  site: 'https://example.com',
  integrations: [postAudit()],
});
```

Set `site`: canonical, sitemap, Open Graph and go-live checks compare against it.

If you use `@astrojs/sitemap`, put `postAudit()` **after** `sitemap()`. Both run in the
`astro:build:done` hook in array order, and the sitemap has to exist before it can be audited.

## Build

```sh
npm run build
```

The audit runs automatically once the build is done and prints its report to the terminal.
Findings are grouped per file, each with the rule id, the element and a remedy that points to
the Astro idiom to fix it (`BaseHead`, `astro:assets`, `Astro.site`).

## Choose how strict it is

```js
postAudit({
  preset: 'standard',
  failOn: 'errors',
});
```

`failOn: 'errors'` fails the build on errors, `'warnings'` also on warnings, `'never'` only
reports. Continue with [Presets](../../guides/presets/) to pick a starting point.

## Skip a single build

```sh
SKIP_AUDIT=1 astro build
```

`postAudit({ disable: true })` switches the integration off permanently.
