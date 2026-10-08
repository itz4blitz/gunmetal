import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import { htmlHeaders } from './src/headers.ts';

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
    // Dev only. The built client asks for these paths on its own origin.
    proxy: {
      '/library.json': { target: 'http://127.0.0.1:4875', changeOrigin: true },
      '/media/library': { target: 'http://127.0.0.1:4875', changeOrigin: true },
    },
  },
  preview: {
    host: '127.0.0.1',
    port: 4173,
    strictPort: true,
    headers,
  },
  build: {
    outDir: '../../../target/clients/web',
    emptyOutDir: true,
    assetsDir: 'assets',
    sourcemap: false,
    modulePreload: { polyfill: false },
  },
});
