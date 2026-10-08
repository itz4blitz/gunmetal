import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
// Deprecated. Open clients/apps/web. This config remains so the old entry still binds to loopback.
import { htmlHeaders } from '../web/src/headers.ts';

const headers = htmlHeaders();
const root = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root,
  publicDir: 'public',
  resolve: {
    alias: {
      'react-native': 'react-native-web',
    },
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    headers,
    // Deprecated. Open clients/apps/web. A down host fails the fetch and the app
    // keeps the fixture catalogue. Dev only: the page asks for same-origin paths.
    proxy: {
      '/library.json': { target: 'http://127.0.0.1:4875', changeOrigin: true },
      '/media/library': { target: 'http://127.0.0.1:4875', changeOrigin: true },
    },
    // Tailnet-only access (tailscale serve fronts this loopback port with
    // HTTPS); the exact host is allow-listed, nothing wild-carded.
    // The node was renamed to gunmetal — the dev server answers to that host only.
    allowedHosts: ['gunmetal.taild1bbf.ts.net'],
  },
  preview: {
    host: '127.0.0.1',
    port: 4173,
    strictPort: true,
    headers,
  },
  build: {
    outDir: '../../../target/clients/demo',
    emptyOutDir: true,
    assetsDir: 'assets',
    sourcemap: false,
    modulePreload: { polyfill: false },
  },
});
