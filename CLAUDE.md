# Project Rules

## Distribution Model
- This tool is ONLY distributed as an Astro integration plugin
- End users configure it exclusively via `astro.config.mjs`
- The Rust binary is an internal implementation detail — it receives parameters from the Astro integration only
- Publishing `@casoon/astro-post-audit` to npm is ALWAYS a manual maintainer action — never add `npm publish` to CI or release workflows
- Do NOT propose, design, or implement standalone CLI features, flags, or workflows for end users
- No `npx astro-post-audit` usage, no extra CLI flags, no standalone config files
- Every new feature must be configurable through the Astro integration options in `astro.config.mjs`

## Scope Boundaries
- Contrast (WCAG 1.4.3) runs only through barrierlab's browser-free cascade (`style-cascade`) over the stylesheets in `dist/`, opt-in via `rules.a11y.contrast`. Never guess a colour: a stylesheet that is not on disk, a background image or a gradient makes the result `untested` with an input note, not a pass. Do not add geometry or size heuristics here; those stay `capability_missing`.
