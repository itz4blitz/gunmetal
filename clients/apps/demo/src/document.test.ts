import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';

const DESCRIPTION =
  'Gunmetal is an open-source media server and player for music, movies and TV. The server sends the original file and gets out of the way.';

async function text(path: string): Promise<string> {
  return readFile(join(process.cwd(), path), 'utf8');
}

test('the document names Gunmetal once, in the head and for a browser with no script', async () => {
  const html = await text('apps/demo/index.html');
  expect(html.startsWith('<!doctype html>\n<html lang="en">')).toStrictEqual(true);
  expect(html.includes('<title>Gunmetal — music, movies and TV</title>')).toStrictEqual(true);
  expect(html.includes('name="description"')).toStrictEqual(true);
  expect(html.includes('property="og:description"')).toStrictEqual(true);
  expect(html.includes('name="twitter:description"')).toStrictEqual(true);
  expect(html.includes(DESCRIPTION)).toStrictEqual(true);
  expect(html.includes('<meta property="og:title" content="Gunmetal — music, movies and TV" />')).toStrictEqual(true);
  expect(html.includes('<meta property="og:type" content="website" />')).toStrictEqual(true);
  expect(html.includes('<meta property="og:image" content="/favicon.svg" />')).toStrictEqual(true);
  expect(html.includes('<meta property="og:locale" content="en" />')).toStrictEqual(true);
  expect(html.includes('<meta name="twitter:card" content="summary" />')).toStrictEqual(true);
  expect(html.includes('<meta name="twitter:title" content="Gunmetal — music, movies and TV" />')).toStrictEqual(true);
  expect(html.includes('<meta name="twitter:image" content="/favicon.svg" />')).toStrictEqual(true);
  expect(
    html.includes('<meta name="theme-color" content="#0f1317" media="(prefers-color-scheme: dark)" />'),
  ).toStrictEqual(true);
  expect(
    html.includes('<meta name="theme-color" content="#f3f5f7" media="(prefers-color-scheme: light)" />'),
  ).toStrictEqual(true);
  expect(html.includes('<meta name="color-scheme" content="dark light" />')).toStrictEqual(true);
  expect(
    html.includes('<meta name="robots" content="index, follow, max-image-preview:large, max-snippet:-1" />'),
  ).toStrictEqual(true);
  expect(html.includes('<meta name="application-name" content="Gunmetal" />')).toStrictEqual(true);
  expect(html.includes('<link rel="icon" href="/favicon.svg" type="image/svg+xml" />')).toStrictEqual(true);
  expect(html.includes('<link rel="apple-touch-icon" href="/apple-touch-icon.png" />')).toStrictEqual(true);
  expect(html.includes('<link rel="manifest" href="/site.webmanifest" />')).toStrictEqual(true);
  expect(html.includes('<link rel="license" href="https://www.gnu.org/licenses/agpl-3.0.html" />')).toStrictEqual(true);
  expect(html.includes('<noscript>')).toStrictEqual(true);
  expect(html.includes(`<h1>Gunmetal</h1>`)).toStrictEqual(true);
  const noscript = html.slice(html.indexOf('<noscript>'), html.indexOf('</noscript>')).replace(/\s+/g, ' ');
  expect(noscript.includes(DESCRIPTION)).toStrictEqual(true);
  // The library is not a place, so the document does not invent coordinates.
  expect(html.includes('geo.position')).toStrictEqual(false);
  expect(html.includes('ICBM')).toStrictEqual(false);

  const jsonLd = html.slice(
    html.indexOf('application/ld+json') + 'application/ld+json">'.length,
    html.indexOf('</script>', html.indexOf('application/ld+json')),
  );
  expect(JSON.parse(jsonLd)).toStrictEqual({
    '@context': 'https://schema.org',
    '@type': 'WebApplication',
    name: 'Gunmetal',
    applicationCategory: 'MultimediaApplication',
    operatingSystem: 'Web',
    description: DESCRIPTION,
    browserRequirements: 'Requires JavaScript.',
    featureList: [
      'Play music, movies and TV from a library you keep',
      'Send the original file from your server',
      'Come back to what you were playing',
    ],
    about: [
      { '@type': 'Thing', name: 'Music' },
      { '@type': 'Thing', name: 'Movies' },
      { '@type': 'Thing', name: 'Television' },
    ],
    license: 'https://www.gnu.org/licenses/agpl-3.0.html',
    image: '/favicon.svg',
  });
});

test('the tab icon is the steel nut and the brass play mark, with nothing behind the shape', async () => {
  const svg = await text('apps/demo/public/favicon.svg');
  expect(svg.includes('viewBox="0 0 1 1"')).toStrictEqual(false);
  expect(svg.includes('fill="#0f1317"')).toStrictEqual(false);
  expect(svg.includes('#d4952f')).toStrictEqual(true);
  expect(svg.includes('<polygon')).toStrictEqual(true);
  expect(svg.includes('<title>Gunmetal</title>')).toStrictEqual(true);
  const png = await readFile(join(process.cwd(), 'apps/demo/public/apple-touch-icon.png'));
  expect(png.subarray(0, 8)).toStrictEqual(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
  // IHDR colour type 6 is RGBA, so the corners outside the nut stay transparent.
  expect(png[25]).toStrictEqual(6);
});

test('crawlers may read the player and may not read the library files', async () => {
  const robots = await text('apps/demo/public/robots.txt');
  expect(robots).toStrictEqual(
    ['User-agent: *', 'Allow: /', 'Disallow: /media/', 'Disallow: /library.json', ''].join('\n'),
  );
  expect(JSON.parse(await text('apps/demo/public/site.webmanifest'))).toStrictEqual({
    name: 'Gunmetal',
    short_name: 'Gunmetal',
    description: DESCRIPTION,
    lang: 'en',
    start_url: '/',
    scope: '/',
    display: 'standalone',
    background_color: '#0f1317',
    theme_color: '#0f1317',
    icons: [
      { src: '/favicon.svg', sizes: 'any', type: 'image/svg+xml', purpose: 'any' },
      { src: '/apple-touch-icon.png', sizes: '180x180', type: 'image/png', purpose: 'any' },
    ],
  });
});
