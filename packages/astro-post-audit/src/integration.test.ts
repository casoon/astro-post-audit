import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import postAudit, { rulesToToml } from './integration.js';
import type { RulesConfig, PostAuditOptions } from './integration.js';

// ==========================================================================
// rulesToToml serialization
// ==========================================================================

describe('rulesToToml', () => {
  it('serializes boolean values', () => {
    const rules: RulesConfig = {
      canonical: { require: true, absolute: false },
    };
    const toml = rulesToToml(rules);
    assert.ok(toml.includes('[canonical]'));
    assert.ok(toml.includes('require = true'));
    assert.ok(toml.includes('absolute = false'));
  });

  it('serializes string values with quotes', () => {
    const rules: RulesConfig = {
      url_normalization: { trailing_slash: 'always', index_html: 'forbid' },
    };
    const toml = rulesToToml(rules);
    assert.ok(toml.includes('[url_normalization]'));
    assert.ok(toml.includes('trailing_slash = "always"'));
    assert.ok(toml.includes('index_html = "forbid"'));
  });

  it('serializes numeric values', () => {
    const rules: RulesConfig = {
      html_basics: { title_max_length: 60, meta_description_max_length: 160 },
    };
    const toml = rulesToToml(rules);
    assert.ok(toml.includes('title_max_length = 60'));
    assert.ok(toml.includes('meta_description_max_length = 160'));
  });

  it('serializes array values', () => {
    const rules: RulesConfig = {
      external_links: {
        allow_domains: ['example.com', 'test.org'],
      },
    };
    const toml = rulesToToml(rules);
    assert.ok(toml.includes('[external_links]'));
    assert.ok(toml.includes('allow_domains = ["example.com", "test.org"]'));
  });

  it('skips undefined values', () => {
    const rules: RulesConfig = {
      canonical: { require: true },
    };
    const toml = rulesToToml(rules);
    // Only 'require' should be present, not 'absolute', 'same_origin', 'self_reference'
    assert.ok(toml.includes('require = true'));
    assert.ok(!toml.includes('absolute'));
    assert.ok(!toml.includes('same_origin'));
  });

  it('handles empty rules', () => {
    const toml = rulesToToml({});
    assert.equal(toml, '');
  });

  it('handles multiple sections', () => {
    const rules: RulesConfig = {
      canonical: { require: true },
      headings: { require_h1: true, single_h1: false },
    };
    const toml = rulesToToml(rules);
    assert.ok(toml.includes('[canonical]'));
    assert.ok(toml.includes('[headings]'));
    assert.ok(toml.includes('require_h1 = true'));
    assert.ok(toml.includes('single_h1 = false'));
  });
});

// ==========================================================================
// postAudit integration factory
// ==========================================================================

describe('postAudit', () => {
  it('returns an AstroIntegration with correct name', () => {
    const integration = postAudit();
    assert.equal(integration.name, 'astro-post-audit');
    assert.ok(integration.hooks);
  });

  it('accepts empty options', () => {
    const integration = postAudit({});
    assert.equal(integration.name, 'astro-post-audit');
  });

  it('accepts all option types', () => {
    const options: PostAuditOptions = {
      strict: true,
      format: 'json',
      maxErrors: 5,
      include: ['**/*.html'],
      exclude: ['drafts/**'],
      noSitemapCheck: true,
      checkAssets: true,
      checkStructuredData: true,
      checkSecurity: true,
      checkDuplicates: true,
      pageOverview: false,
      disable: false,
      throwOnError: true,
    };
    const integration = postAudit(options);
    assert.equal(integration.name, 'astro-post-audit');
  });

  it('throws when both config and rules are set', () => {
    const integration = postAudit({
      config: '/path/to/rules.toml',
      rules: { canonical: { require: true } },
    });

    // Simulate astro:build:done hook
    const hook = integration.hooks['astro:build:done'] as Function;
    assert.throws(
      () =>
        hook({
          dir: new URL('file:///tmp/dist/'),
          logger: {
            info: () => {},
            warn: () => {},
            error: () => {},
          },
        }),
      {
        message: /mutually exclusive/,
      },
    );
  });

  it('does not throw when only config is set', () => {
    const integration = postAudit({ config: '/path/to/rules.toml' });
    const hook = integration.hooks['astro:build:done'] as Function;
    // Should not throw (will just warn about missing binary)
    assert.doesNotThrow(() =>
      hook({
        dir: new URL('file:///tmp/dist/'),
        logger: {
          info: () => {},
          warn: () => {},
          error: () => {},
        },
      }),
    );
  });

  it('does not throw when only rules is set', () => {
    const integration = postAudit({
      rules: { canonical: { require: true } },
    });
    const hook = integration.hooks['astro:build:done'] as Function;
    assert.doesNotThrow(() =>
      hook({
        dir: new URL('file:///tmp/dist/'),
        logger: {
          info: () => {},
          warn: () => {},
          error: () => {},
        },
      }),
    );
  });

  it('skips execution when disabled', () => {
    const integration = postAudit({ disable: true });
    const hook = integration.hooks['astro:build:done'] as Function;
    // Should return immediately without doing anything
    assert.doesNotThrow(() =>
      hook({
        dir: new URL('file:///tmp/dist/'),
        logger: {
          info: () => {},
          warn: () => {},
          error: () => {},
        },
      }),
    );
  });
});
