// @ts-check
import { readFileSync } from 'node:fs';
import casoonPages from '@casoon/pages-theme';
import { defineConfig } from 'astro/config';

const { version } = JSON.parse(
  readFileSync(new URL('../packages/astro-post-audit/package.json', import.meta.url), 'utf8'),
);

// Project page: https://casoon.github.io/astro-post-audit/ — `base` is the GitHub Pages path.
export default defineConfig({
  site: 'https://casoon.github.io/astro-post-audit',
  base: '/astro-post-audit/',
  integrations: [
    casoonPages({
      name: 'astro-post-audit',
      description:
        'Astro integration that audits the built dist/ output for SEO, broken links and lightweight WCAG heuristics, offline and in Rust.',
      tagline: 'SEO, link and a11y checks after build',
      repo: 'casoon/astro-post-audit',
      version,
      license: 'MIT',
      packages: [
        { label: 'npm', href: 'https://www.npmjs.com/package/@casoon/astro-post-audit' },
        { label: 'crates.io', href: 'https://crates.io/crates/astro-post-audit' },
      ],
      docsGroups: {
        'getting-started': 'Getting started',
        guides: 'Guides',
        reference: 'Reference',
      },
    }),
  ],
});
