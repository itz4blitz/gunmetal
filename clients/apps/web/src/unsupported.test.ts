import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';

test('the shell stylesheet uses the dark canvas and primary text token colours', async () => {
  const css = await readFile(join(process.cwd(), 'apps/web/public/shell.css'), 'utf8');
  expect(css.includes('background: #0f1317')).toStrictEqual(true);
  expect(css.includes('color: #e9eef2')).toStrictEqual(true);
  expect(css.includes('url(')).toStrictEqual(false);
});

// Verifies: SEC-API-052, CLI-002
test('the unsupported-browser page is static HTML with no script', async () => {
  const html = await readFile(join(process.cwd(), 'apps/web/public/unsupported.html'), 'utf8');
  expect(html.includes('<script')).toStrictEqual(false);
  expect(html.includes('WebAssembly')).toStrictEqual(true);
  expect(html.includes('secure page')).toStrictEqual(true);
});
