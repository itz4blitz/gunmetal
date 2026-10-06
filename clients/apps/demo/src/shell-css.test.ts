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
  const spotlight = css.slice(css.indexOf('#home-spotlight'));
  expect(spotlight.includes('min-height: 280px')).toStrictEqual(true);
  expect(css.includes('[data-spotlight-eyebrow]')).toStrictEqual(true);
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
  expect(css.includes('var(--gm-accent-indicator)')).toStrictEqual(true);
  expect(css.includes("#library-tabs [data-selected='1']::after")).toStrictEqual(true);
  expect(css.includes('var(--gm-radius-m)')).toStrictEqual(true);
  const marker = "#library-tabs [data-selected='1']";
  const start = css.indexOf(marker);
  const block = css.slice(start, css.indexOf('}', start) + 1);
  expect(block.includes('border-radius: var(--gm-radius-m)')).toStrictEqual(true);
  expect(block.includes('border-radius: 999px') || block.includes('pill')).toStrictEqual(false);
  expect(block.includes('background: transparent')).toStrictEqual(true);
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

test('settings app uses a steel side list on wide and brass R2 badges', async () => {
  const css = await demoShellCss();
  expect(css.includes('#settings-nav')).toStrictEqual(true);
  expect(
    css.includes("[data-settings-layout='side']") || css.includes('[data-settings-layout="side"]'),
  ).toStrictEqual(true);
  const sideMarker = css.includes("[data-settings-layout='side']")
    ? "[data-settings-layout='side']"
    : '[data-settings-layout="side"]';
  const side = css.slice(css.indexOf(sideMarker));
  expect(side.includes('flex-direction: row')).toStrictEqual(true);
  expect(
    css.includes("[data-settings-badge='R2']") || css.includes('[data-settings-badge="R2"]'),
  ).toStrictEqual(true);
  const badgeMarker = css.includes("[data-settings-badge='R2']")
    ? "[data-settings-badge='R2']"
    : '[data-settings-badge="R2"]';
  const badge = css.slice(css.indexOf(badgeMarker));
  expect(badge.includes('var(--gm-accent-text)')).toStrictEqual(true);
  expect(css.includes('[data-settings-fact]')).toStrictEqual(true);
  expect(css.includes('#settings-nav [data-selected=') || css.includes('#settings-nav [aria-selected=')).toStrictEqual(
    true,
  );
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

test('2026 motion staggers tiles, fades heroes without parallax and presses at 0.98', async () => {
  const css = await demoShellCss();
  expect(css.includes('gm-tile-enter')).toStrictEqual(true);
  expect(css.includes('animation-delay: 20ms')).toStrictEqual(true);
  expect(css.includes('animation-delay: 120ms')).toStrictEqual(true);
  expect(css.includes('gm-hero-enter')).toStrictEqual(true);
  expect(css.includes('parallax')).toStrictEqual(false);
  expect(css.includes('gm-page-enter')).toStrictEqual(true);
  expect(css.includes('160ms')).toStrictEqual(true);
  expect(css.includes('scale(0.98)')).toStrictEqual(true);
  expect(css.includes('#destination-artist[data-art-tone]')).toStrictEqual(true);
  expect(css.includes('[data-artist-hero]')).toStrictEqual(true);
  const reduced = css.slice(css.indexOf('prefers-reduced-motion: reduce'));
  expect(reduced.includes('[data-album-tile]')).toStrictEqual(true);
  expect(reduced.includes('animation: none')).toStrictEqual(true);
});

test('context menus sit on overlay at radius.l with brass focus and 160ms motion', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-context-menu]') || css.includes('[data-item-menu]')).toStrictEqual(true);
  const menuMarker = css.includes('[data-context-menu]') ? '[data-context-menu]' : '[data-item-menu]';
  const menu = css.slice(css.indexOf(menuMarker));
  expect(menu.includes('var(--gm-radius-l)')).toStrictEqual(true);
  expect(menu.includes('var(--gm-bg-overlay)')).toStrictEqual(true);
  expect(css.includes('var(--gm-focus-ring)')).toStrictEqual(true);
  expect(css.includes('160ms')).toStrictEqual(true);
  expect(css.includes('prefers-reduced-motion: reduce')).toStrictEqual(true);
  expect(css.includes('[data-lyrics-line]')).toStrictEqual(true);
  expect(css.includes('[data-lyrics-line][data-current') || css.includes("[data-lyrics-line][data-current")).toStrictEqual(
    true,
  );
});

test('album detail wash denser track rows and full player sheet are crafted', async () => {
  const css = await demoShellCss();
  expect(css.includes('#destination-album[data-art-tone]')).toStrictEqual(true);
  expect(css.includes('min-height: 48px')).toStrictEqual(true);
  expect(css.includes('[data-now-playing=') || css.includes('[data-now-playing=')).toStrictEqual(
    true,
  );
  expect(css.includes('#player-full')).toStrictEqual(true);
  expect(css.includes('#player-full-scrim')).toStrictEqual(true);
  const fullOpenMarker = css.includes("#player-full[data-open='1']")
    ? "#player-full[data-open='1']"
    : '#player-full[data-open="1"]';
  const fullOpenStart = css.indexOf(fullOpenMarker);
  expect(fullOpenStart).toBeGreaterThanOrEqual(0);
  const fullOpenBlock = css.slice(fullOpenStart, css.indexOf('}', fullOpenStart) + 1);
  expect(fullOpenBlock.includes('display: flex')).toStrictEqual(true);
  expect(fullOpenBlock.includes('display: none')).toStrictEqual(false);
  expect(css.includes('#player-expand')).toStrictEqual(true);
  expect(css.includes('#player-art') && css.includes('cursor: pointer')).toStrictEqual(true);
  expect(css.includes("[data-size='full']") || css.includes('[data-size="full"]')).toStrictEqual(
    true,
  );
});

test('settings side list, artist wash, menus and lyrics pane are crafted', async () => {
  const css = await demoShellCss();
  expect(css.includes("#destination-settings[data-settings-layout='side']")).toStrictEqual(true);
  expect(css.includes('#settings-nav')).toStrictEqual(true);
  expect(css.includes('#destination-artist[data-art-tone]')).toStrictEqual(true);
  expect(css.includes('[data-item-menu]') || css.includes('[data-context-menu]')).toStrictEqual(true);
  expect(css.includes('#player-full-lyrics')).toStrictEqual(true);
  expect(css.includes('[data-lyrics-line]')).toStrictEqual(true);
});
