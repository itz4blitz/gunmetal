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

test('album tile hover lifts cover art and fades the brass play control', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-album-tile]:hover [data-size]')).toStrictEqual(true);
  expect(css.includes('translateY(-4px) scale(1.02)')).toStrictEqual(true);
  expect(css.includes('[data-album-play]')).toStrictEqual(true);
  expect(css.includes('opacity')).toStrictEqual(true);
  expect(css.includes('flex: 0 0 180px')).toStrictEqual(true);
  expect(css.includes('scroll-snap-type: x mandatory')).toStrictEqual(true);
});

test('home spotlight wash uses art-surface vars keyed by data-art-tone', async () => {
  const css = await demoShellCss();
  expect(css.includes('#destination-home[data-art-tone]')).toStrictEqual(true);
  expect(css.includes('--art-surface')).toStrictEqual(true);
  expect(css.includes('#home-spotlight')).toStrictEqual(true);
  expect(css.includes('[data-type=\'display\']') || css.includes('[data-type="display"]')).toStrictEqual(
    true,
  );
});

test('queue sheet enters and exits over 200ms and respects reduced motion', async () => {
  const css = await demoShellCss();
  expect(css.includes('#queue-sheet[data-queue-open=')).toStrictEqual(true);
  expect(css.includes('200ms')).toStrictEqual(true);
  expect(css.includes('prefers-reduced-motion: reduce')).toStrictEqual(true);
});

test('shell chrome elevates brand rule, nav glyphs, vignette, art-tint and wide breath', async () => {
  const css = await demoShellCss();
  expect(css.includes('#shell-brand-rule')).toStrictEqual(true);
  expect(css.includes('var(--gm-accent-fill)')).toStrictEqual(true);
  expect(css.includes('[data-nav-glyph=')).toStrictEqual(true);
  expect(css.includes("content: ''") || css.includes('content:""')).toStrictEqual(true);
  expect(css.includes('Canvas depth: subtle top vignette')).toStrictEqual(true);
  expect(css.includes('linear-gradient')).toStrictEqual(true);
  expect(css.includes('[data-art-tone=')).toStrictEqual(true);
  expect(css.includes('#player-bar::before') || css.includes('#player-bar:before')).toStrictEqual(
    true,
  );
  expect(css.includes('#demo-label')).toStrictEqual(true);
  expect(css.includes("data-width='wide'] #content") || css.includes('data-width="wide"] #content')).toStrictEqual(
    true,
  );
  expect(css.includes('padding: 32px')).toStrictEqual(true);
  expect(css.includes('backdrop-filter')).toStrictEqual(true);
  expect(css.includes('clip-path')).toStrictEqual(true);
});

test('library tabs stick with a brass underline indicator at radius.m, not pills', async () => {
  const css = await demoShellCss();
  expect(css.includes('#library-tabs')).toStrictEqual(true);
  expect(css.includes('position: sticky')).toStrictEqual(true);
  expect(css.includes('inset 0 -2px 0 var(--gm-accent-indicator)')).toStrictEqual(true);
  expect(css.includes('var(--gm-radius-m)')).toStrictEqual(true);
  const tabSelected = css.slice(css.indexOf('#library-tabs [aria-selected=\'true\']'));
  expect(tabSelected.includes('border-radius: 999px') || tabSelected.includes('pill')).toStrictEqual(
    false,
  );
});

test('library album grid uses minmax 160px and a 24px gap', async () => {
  const css = await demoShellCss();
  expect(css.includes('minmax(160px, 1fr)')).toStrictEqual(true);
  const grid = css.slice(css.indexOf('#library-album-grid'));
  expect(grid.includes('gap: 24px')).toStrictEqual(true);
});

test('artist rows are 56px with a hex avatar and tracks raise on hover with tabular duration', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-artist-avatar]')).toStrictEqual(true);
  expect(css.includes('clip-path: polygon(25% 6%, 75% 6%, 100% 50%, 75% 94%, 25% 94%, 0% 50%)')).toStrictEqual(
    true,
  );
  const artistRow = css.slice(css.indexOf('[data-artist-row]'));
  expect(artistRow.includes('min-height: 56px') || artistRow.includes('height: 56px')).toStrictEqual(
    true,
  );
  expect(css.includes('[data-track-duration]')).toStrictEqual(true);
  expect(css.includes('font-variant-numeric: tabular-nums')).toStrictEqual(true);
  expect(css.includes('var(--gm-raised-edge)')).toStrictEqual(true);
});

test('settings panels sit on raised machined surfaces and theme is segmented', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-settings-panel]')).toStrictEqual(true);
  expect(css.includes('#settings-appearance #theme-switcher')).toStrictEqual(true);
  const segmented = css.slice(css.indexOf('#settings-appearance #theme-switcher'));
  expect(segmented.includes('flex-direction: row')).toStrictEqual(true);
  expect(segmented.includes('var(--gm-bg-inset)')).toStrictEqual(true);
});

test('page enter fades over 160ms and scrollbars are thin muted chrome', async () => {
  const css = await demoShellCss();
  expect(css.includes('gm-page-enter') || css.includes('@keyframes')).toStrictEqual(true);
  expect(css.includes('160ms')).toStrictEqual(true);
  expect(css.includes('prefers-reduced-motion: reduce')).toStrictEqual(true);
  expect(css.includes('scrollbar-width: thin') || css.includes('::-webkit-scrollbar')).toStrictEqual(
    true,
  );
});
