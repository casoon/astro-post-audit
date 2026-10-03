// @ts-check
import casoonPages from '@casoon/pages-theme';
import { defineConfig } from 'astro/config';

// Project page: https://casoon.github.io/astro-post-audit/ — `base` is the GitHub Pages path.
export default defineConfig({
  site: 'https://casoon.github.io/astro-post-audit',
  base: '/astro-post-audit/',
  integrations: [
    casoonPages({
      name: 'astro-post-audit',
      description:
        'Astro integration that audits the built dist/ output for SEO, broken links and lightweight WCAG heuristics, offline and in Rust.',
      repo: 'casoon/astro-post-audit',
      version: '0.9.1',
      license: 'MIT',
      packages: [
        { label: 'npm', href: 'https://www.npmjs.com/package/@casoon/astro-post-audit' },
        { label: 'crates.io', href: 'https://crates.io/crates/astro-post-audit' },
        { label: 'Product page', href: 'https://astro-post-audit.casoon.de' },
      ],
      docsGroups: {
        'getting-started': 'Getting started',
        guides: 'Guides',
        reference: 'Reference',
      },
    }),
  ],
});
