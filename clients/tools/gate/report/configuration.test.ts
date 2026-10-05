import { readFile } from 'node:fs/promises';
import { expect, test } from 'vitest';
import lint from '../../../eslint.config.js';
import format from '../../../prettier.config.js';
import tests from '../../../vitest.config.ts';

const client = new URL('../../../', import.meta.url);
async function json(name: string): Promise<unknown> {
  return JSON.parse(await readFile(new URL(name, client), 'utf8'));
}

// The covered set of the client plan: three globs, less test files and the planted-fault fixtures.
// Every threshold is 100 for each file, and at most four workers run, whatever machine this is.
test('the test configuration covers exactly the stated set at 100% per file with four workers', () => {
  expect(tests).toStrictEqual({
    test: {
      maxWorkers: 4,
      testTimeout: 120000,
      coverage: {
        enabled: true,
        provider: 'v8',
        include: ['packages/*/src/**/*.{ts,tsx}', 'apps/*/src/**/*.{ts,tsx}', 'tools/**/*.ts'],
        exclude: ['**/*.test.{ts,tsx}', 'tools/fixtures/**'],
        thresholds: { perFile: true, lines: 100, functions: 100, statements: 100, branches: 100 },
        reporter: ['text'],
        reportsDirectory: '../target/clients/coverage',
      },
      projects: [
        {
          extends: true,
          test: {
            name: 'packages and apps',
            environment: 'jsdom',
            include: ['packages/*/src/**/*.test.{ts,tsx}', 'apps/*/src/**/*.test.{ts,tsx}'],
          },
        },
        { extends: true, test: { name: 'tools', environment: 'node', include: ['tools/**/*.test.ts'] } },
      ],
    },
  });
});

test('the formatter configuration is the committed two settings', () => {
  expect(format).toStrictEqual({ printWidth: 120, singleQuote: true });
});

// The whole file is compared, as the base file is, so no added option can switch a check off.
test('the type configuration checks every package, app and tool with the strict base settings', async () => {
  expect(await json('tsconfig.json')).toStrictEqual({
    extends: './tsconfig.base.json',
    compilerOptions: {
      target: 'es2024',
      lib: ['es2024', 'dom', 'dom.iterable'],
      module: 'nodenext',
      moduleResolution: 'nodenext',
      allowImportingTsExtensions: true,
      allowJs: true,
      erasableSyntaxOnly: true,
      verbatimModuleSyntax: true,
      jsx: 'react-jsx',
      types: ['node'],
      skipLibCheck: true,
    },
    include: ['apps', 'packages', 'tools', 'vitest.config.ts'],
  });
});

// `files` and `ignores` of every lint configuration object, in order. The one path the lint skips is the
// planted-fault fixtures; every other `ignores` only says where a rule that is scoped by path stops.
test('the lint skips the planted-fault fixtures and nothing else', () => {
  expect(
    (lint as { name?: string; files?: unknown; ignores?: unknown }[])
      .filter((entry) => entry.name?.startsWith('gunmetal') === true)
      .map(({ name, files, ignores }) => ({ name, files, ignores })),
  ).toStrictEqual([
    { name: 'gunmetal/skipped', files: undefined, ignores: ['tools/fixtures/**'] },
    { name: 'gunmetal/everywhere', files: ['**/*.{js,ts,tsx}'], ignores: undefined },
    { name: 'gunmetal/tests', files: ['**/*.test.{ts,tsx}'], ignores: undefined },
  ]);
  expect((lint as { name?: string; ignores?: unknown }[]).filter((entry) => entry.name?.startsWith('gunmetal') !== true && entry.ignores !== undefined)).toStrictEqual([]);
});

test('inline lint switches have no effect and are reported', () => {
  expect((lint as { name?: string; linterOptions?: unknown }[]).find((entry) => entry.name === 'gunmetal/everywhere')?.linterOptions).toStrictEqual({
    noInlineConfig: true,
    reportUnusedDisableDirectives: 'error',
  });
});
