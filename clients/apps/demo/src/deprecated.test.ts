import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';

test('the demo app is deprecated and tells a person to open the web client', async () => {
  const manifest = JSON.parse(await readFile(join(process.cwd(), 'apps/demo/package.json'), 'utf8')) as {
    description: string;
  };
  const readme = await readFile(join(process.cwd(), 'apps/demo/README.md'), 'utf8');
  const html = await readFile(join(process.cwd(), 'apps/demo/index.html'), 'utf8');
  const config = await readFile(join(process.cwd(), 'apps/demo/vite.config.ts'), 'utf8');
  expect(manifest.description).toStrictEqual(
    'Deprecated. Open clients/apps/web. This package remains so its tests can prove the player the web client imports.',
  );
  expect(readme).toStrictEqual(
    [
      '# Deprecated',
      '',
      'Open `clients/apps/web`. That is the client. This app remains so its tests can prove the player modules the web client imports. New features do not land here.',
      '',
    ].join('\n'),
  );
  expect(html.includes('Deprecated. Open clients/apps/web.')).toStrictEqual(true);
  expect(config.includes('Deprecated. Open clients/apps/web.')).toStrictEqual(true);
  expect(config.includes('192.168.1.120')).toStrictEqual(false);
  expect(config.includes("target: 'http://127.0.0.1:4875'")).toStrictEqual(true);
});
