import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';

async function demoShellCss(): Promise<string> {
  return readFile(join(process.cwd(), 'apps/demo/public/shell.css'), 'utf8');
}

test('empty cards use secondary title, muted body, and a quiet CSS mark', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-empty-title]')).toStrictEqual(true);
  expect(css.includes('[data-empty-mark]')).toStrictEqual(true);
  expect(css.includes('var(--gm-text-secondary)')).toStrictEqual(true);
  expect(css.includes('var(--gm-text-muted)')).toStrictEqual(true);
  expect(css.includes('empty-mark')).toStrictEqual(true);
  expect(css.includes('.png') || css.includes('.svg') || css.includes('.webp')).toStrictEqual(
    false,
  );
});

test('artist subtitles use text.secondary while captions stay muted', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-album-artist]')).toStrictEqual(true);
  const artistBlock = css.slice(css.indexOf('[data-album-artist]'));
  expect(artistBlock.includes('var(--gm-text-secondary)')).toStrictEqual(true);
  expect(css.includes('#theme-label')).toStrictEqual(true);
  expect(css.includes('--gm-text-secondary: #b0bbc5')).toStrictEqual(true);
});

test('search field strengthens focus with line.control and muted placeholder', async () => {
  const css = await demoShellCss();
  expect(css.includes('#search-field:focus-visible')).toStrictEqual(true);
  expect(css.includes('var(--gm-line-control)')).toStrictEqual(true);
  expect(css.includes('#search-field::placeholder')).toStrictEqual(true);
});

test('album tile hover scales cover art and fades the brass play control', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-album-tile]:hover [data-size]')).toStrictEqual(true);
  expect(css.includes('scale(1.03)')).toStrictEqual(true);
  expect(css.includes('[data-album-play]')).toStrictEqual(true);
  expect(css.includes('opacity')).toStrictEqual(true);
});

test('queue sheet enters and exits over 200ms and respects reduced motion', async () => {
  const css = await demoShellCss();
  expect(css.includes('#queue-sheet[data-queue-open=')).toStrictEqual(true);
  expect(css.includes('200ms')).toStrictEqual(true);
  expect(css.includes('prefers-reduced-motion: reduce')).toStrictEqual(true);
});
