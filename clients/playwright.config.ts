export default {
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: true,
  retries: 0,
  workers: 1,
  use: {
    baseURL: 'http://127.0.0.1:5173',
  },
  webServer: {
    command: 'pnpm exec vite --config apps/web/vite.config.ts',
    url: 'http://127.0.0.1:5173',
    reuseExistingServer: true,
  },
};
