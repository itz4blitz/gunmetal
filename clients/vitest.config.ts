import { defineConfig } from 'vitest/config';

// Stub: enough to find the tests. configuration.test.ts comes first and must fail on its assertions.
export default defineConfig({
  test: {
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
