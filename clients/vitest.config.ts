import { defineConfig } from 'vitest/config';

// Unit and component tests of the client workspace, with coverage at 100% for every file of the covered
// set (docs/plan/client-packages.md, "What is covered and mutated, and what is not"): the three globs
// under `include`, less test files and the planted-fault fixtures. A file of the set that no test
// imports is still counted. The worker count is fixed, because the build machine is shared.
export default defineConfig({
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
