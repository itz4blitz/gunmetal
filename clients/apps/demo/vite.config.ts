import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
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
    port: 5174,
    strictPort: true,
    headers,
  },
  preview: {
    host: '127.0.0.1',
    port: 4174,
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
