import type { AstroIntegration } from 'astro';
export interface PostAuditOptions {
    /** Path to rules.toml config file */
    config?: string;
    /** Base URL (auto-detected from Astro site config if not set) */
    site?: string;
    /** Treat warnings as errors */
    strict?: boolean;
    /** Output format: 'text' or 'json' */
    format?: 'text' | 'json';
    /** Glob patterns to exclude */
    exclude?: string[];
    /** Skip sitemap checks */
    noSitemapCheck?: boolean;
    /** Enable asset reference checking */
    checkAssets?: boolean;
    /** Enable structured data validation */
    checkStructuredData?: boolean;
    /** Enable security heuristic checks */
    checkSecurity?: boolean;
    /** Enable duplicate content detection */
    checkDuplicates?: boolean;
    /** Disable the integration (useful for dev) */
    disable?: boolean;
}
export default function postAudit(options?: PostAuditOptions): AstroIntegration;
//# sourceMappingURL=integration.d.ts.map