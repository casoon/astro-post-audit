# Project Rules

## Distribution Model
- This tool is ONLY distributed as an Astro integration plugin
- End users configure it exclusively via `astro.config.mjs`
- The Rust binary is an internal implementation detail — it receives parameters from the Astro integration only
- Do NOT propose, design, or implement standalone CLI features, flags, or workflows for end users
- No `npx astro-post-audit` usage, no extra CLI flags, no standalone config files
- Every new feature must be configurable through the Astro integration options in `astro.config.mjs`
