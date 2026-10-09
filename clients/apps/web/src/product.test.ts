import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';

const DESCRIPTION =
  'Gunmetal is an open-source media server and player for music, movies and TV. The server sends the original file and gets out of the way.';

async function text(path: string): Promise<string> {
  return readFile(join(process.cwd(), path), 'utf8');
}

test('the web document is the player you open, with same-origin sheets and no native shell', async () => {
  const html = await text('apps/web/index.html');
  expect(html.startsWith('<!doctype html>\n<html lang="en">')).toStrictEqual(true);
  expect(html.includes('<title>Gunmetal — music, movies and TV</title>')).toStrictEqual(true);
  expect(html.includes('name="description"')).toStrictEqual(true);
  expect(html.includes(DESCRIPTION)).toStrictEqual(true);
  expect(html.includes('<link rel="manifest" href="/site.webmanifest" />')).toStrictEqual(true);
  expect(html.includes('<link rel="apple-touch-icon" href="/apple-touch-icon.png" />')).toStrictEqual(true);
  expect(html.includes('<script type="module" src="/entry.tsx"></script>')).toStrictEqual(true);
  expect(html.includes('electron')).toStrictEqual(false);
  expect(html.includes('tauri')).toStrictEqual(false);
  expect(html.includes('192.168.1.120')).toStrictEqual(false);
  const sheets = [...html.matchAll(/<link rel="stylesheet" href="\/([a-z-]+\.css)" \/>/g)].map((match) => match[1]);
  expect(sheets).toStrictEqual([
    'shell.css',
    'area-home.css',
    'area-album.css',
    'area-library.css',
    'area-settings.css',
    'area-lyrics.css',
    'area-player.css',
    'system.css',
  ]);
  const entry = await text('apps/web/entry.tsx');
  expect(entry.includes('openPlayer')).toStrictEqual(true);
  expect(entry.includes('start(')).toStrictEqual(true);
  expect(entry.includes('192.168.1.120')).toStrictEqual(false);
  expect(entry.includes('electron')).toStrictEqual(false);
  expect(entry.includes('tauri')).toStrictEqual(false);
  const config = await text('apps/web/vite.config.ts');
  expect(config.includes("target: 'http://127.0.0.1:4875'")).toStrictEqual(true);
  expect(config.includes("'/library.json'")).toStrictEqual(true);
  expect(config.includes("'/media/library'")).toStrictEqual(true);
  expect(config.includes('192.168.1.120')).toStrictEqual(false);
  expect(config.includes('electron')).toStrictEqual(false);
  expect(config.includes('tauri')).toStrictEqual(false);
});

test('the player sheets, font, fixture audio and document icons resolve on the web origin', async () => {
  const shell = await text('apps/web/public/shell.css');
  expect(shell.includes('background: #0f1317')).toStrictEqual(true);
  expect(shell.includes('color: #e9eef2')).toStrictEqual(true);
  expect(shell.includes('--gm-accent-fill: #d4952f')).toStrictEqual(true);
  expect(shell.includes("url('/fonts/InterVariable.woff2')")).toStrictEqual(true);
  expect(shell.includes('http://')).toStrictEqual(false);
  expect(shell.includes('https://')).toStrictEqual(false);
  const font = await readFile(join(process.cwd(), 'apps/web/public/fonts/InterVariable.woff2'));
  expect(font.subarray(0, 4)).toStrictEqual(Buffer.from([0x77, 0x4f, 0x46, 0x32]));
  const wav = await readFile(join(process.cwd(), 'apps/web/public/media/audio/demo-album-01.wav'));
  expect(wav.subarray(0, 4)).toStrictEqual(Buffer.from('RIFF'));
  const png = await readFile(join(process.cwd(), 'apps/web/public/apple-touch-icon.png'));
  expect(png.subarray(0, 8)).toStrictEqual(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
  expect(await text('apps/web/public/robots.txt')).toStrictEqual(
    ['User-agent: *', 'Allow: /', 'Disallow: /media/', 'Disallow: /library.json', ''].join('\n'),
  );
});
