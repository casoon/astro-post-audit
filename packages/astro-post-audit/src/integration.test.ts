import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import postAudit from './integration.js';
import type { PostAuditOptions } from './integration.js';

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
      maxErrors: 5,
      pageOverview: false,
      output: 'audit-report.json',
      disable: false,
      throwOnError: true,
      rules: { canonical: { require: true } },
    };
    const integration = postAudit(options);
    assert.equal(integration.name, 'astro-post-audit');
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
