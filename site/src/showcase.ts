import { ansiToHtml } from '@casoon/pages-theme/ansi';
import type { ShowcaseExample } from '@casoon/pages-theme/showcase';

// Real terminal output of the audit binary, captured from the test fixtures by
// examples/capture.sh. The report itself is rendered in Rust, so the site only
// converts the captured ANSI to HTML at build time.
const captures = import.meta.glob<string>('../../examples/*.ansi', {
  query: '?raw',
  import: 'default',
  eager: true,
});

// The fixture pages the Rust test suite audits, shown as the input.
const fixtures = import.meta.glob<string>('../../tests/fixtures/**/*.html', {
  query: '?raw',
  import: 'default',
  eager: true,
});

function read(files: Record<string, string>, path: string): string {
  const found = files[path];
  if (found === undefined) throw new Error(`Missing file: ${path}`);
  return found;
}

export const capture = (file: string) => read(captures, `../../examples/${file}`);

const examples_ = [
  {
    slug: 'failing-page',
    title: 'A page with many problems',
    file: 'report.ansi',
    input: { code: read(fixtures, '../../tests/fixtures/bad/index.html'), lang: 'html' },
    tags: ['links', 'accessibility', 'seo', 'images'],
    description:
      'tests/fixtures/bad/index.html audited with the default rules: broken links, missing lang, title and labels, unnamed controls. The build fails.',
  },
  {
    slug: 'clean-site',
    title: 'A clean site',
    file: 'check.ansi',
    input: { code: read(fixtures, '../../tests/fixtures/good/index.html'), lang: 'html' },
    tags: ['pass', 'advisory'],
    description:
      'The three pages in tests/fixtures/good. No errors and no warnings; advisory findings do not fail the build.',
  },
  {
    slug: 'baseline',
    title: 'Only new findings with a baseline',
    file: 'baseline.ansi',
    input: {
      code: read(fixtures, '../../tests/fixtures/edge-cases/long-title.html'),
      lang: 'html',
    },
    tags: ['baseline'],
    description:
      'A baseline written for the clean site, then this page added. The report lists only the findings of the new page.',
  },
  {
    slug: 'verbose-progress',
    title: 'Verbose progress',
    file: 'progress.ansi',
    input: { code: "postAudit({ progress: 'verbose' })", lang: 'js' },
    tags: ['diagnostics', 'stderr'],
    description:
      "progress: 'verbose' prints one line per check with finding count and timing on stderr, here for the 14 edge-case fixtures.",
  },
];

export const examples: ShowcaseExample[] = examples_.map(({ file, input, ...meta }) => ({
  ...meta,
  file: `examples/${file}`,
  input,
  output: { html: ansiToHtml(capture(file)), kind: 'terminal' },
}));
