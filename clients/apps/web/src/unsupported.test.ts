import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';

test('the shell stylesheet is the player sheet: token colours and a same-origin font', async () => {
  const css = await readFile(join(process.cwd(), 'apps/web/public/shell.css'), 'utf8');
  expect(css.includes('background: #0f1317')).toStrictEqual(true);
  expect(css.includes('color: #e9eef2')).toStrictEqual(true);
  expect(css.includes('--gm-accent-fill: #d4952f')).toStrictEqual(true);
  expect(css.includes("url('/fonts/InterVariable.woff2')")).toStrictEqual(true);
  expect(css.includes('http://')).toStrictEqual(false);
  expect(css.includes('https://')).toStrictEqual(false);
});

// Verifies: SEC-API-052, CLI-002
test('the unsupported-browser page is static HTML with no script', async () => {
  const html = await readFile(join(process.cwd(), 'apps/web/public/unsupported.html'), 'utf8');
  expect(html.includes('<script')).toStrictEqual(false);
  expect(html.includes('WebAssembly')).toStrictEqual(true);
  expect(html.includes('secure page')).toStrictEqual(true);
});
