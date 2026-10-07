import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';

const fontsDir = join(process.cwd(), 'apps/demo/public/fonts');

// Verifies: SEC-SUP-029, SEC-CLI-012
test('the demo ships Inter as a self-hosted OFL WOFF2 and no third-party font URL', async () => {
  const font = await readFile(join(fontsDir, 'InterVariable.woff2'));
  expect(font.subarray(0, 4).toString('ascii')).toStrictEqual('wOF2');
  expect(font.byteLength).toBeGreaterThan(100_000);
  const licence = await readFile(join(fontsDir, 'LICENSE.txt'), 'utf8');
  expect(licence.includes('SIL OPEN FONT LICENSE')).toStrictEqual(true);
  expect(licence.includes('The Inter Project Authors')).toStrictEqual(true);
  const css = await readFile(join(process.cwd(), 'apps/demo/public/shell.css'), 'utf8');
  expect(css.includes("url('/fonts/InterVariable.woff2')")).toStrictEqual(true);
  expect(css.includes('https://')).toStrictEqual(false);
});
