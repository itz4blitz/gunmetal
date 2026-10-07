import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';
import { htmlHeaders } from '../../web/src/headers.ts';

// Verifies: SEC-CLI-019
test('the demo dev configuration names the loopback address literally', async () => {
  const source = await readFile(join(process.cwd(), 'apps/demo/vite.config.ts'), 'utf8');
  const hosts = [...source.matchAll(/host: '([^']+)'/g)].map((match) => match[1]);
  expect(hosts).toStrictEqual(['127.0.0.1', '127.0.0.1']);
});

test('the demo dev configuration sends the production headers', async () => {
  const source = await readFile(join(process.cwd(), 'apps/demo/vite.config.ts'), 'utf8');
  expect(source.includes('htmlHeaders()')).toStrictEqual(true);
  expect(htmlHeaders()['X-Content-Type-Options']).toStrictEqual('nosniff');
});
