# Project Rules

## Distribution Model
- This tool is ONLY distributed as an Astro integration plugin
- End users configure it exclusively via `astro.config.mjs`
- The Rust binary is an internal implementation detail — it receives parameters from the Astro integration only
- Do NOT propose, design, or implement standalone CLI features, flags, or workflows for end users
- No `npx astro-post-audit` usage, no extra CLI flags, no standalone config files
- Every new feature must be configurable through the Astro integration options in `astro.config.mjs`

## Scope Boundaries
- Do NOT implement WCAG 1.4.3 contrast ratio checking — this requires browser rendering (computed styles, CSS variables, transparency, media queries) and cannot be done reliably via static HTML analysis. Users should use Lighthouse, axe-core, or pa11y for contrast checks.
