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
  expect(spotlight.includes('min-height: 360px')).toStrictEqual(true);
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

test('every CLI-141 theme sets color-scheme and paints html through :has', async () => {
  const css = await demoShellCss();
  expect(css.includes("color-scheme: dark")).toStrictEqual(true);
  expect(css.includes("color-scheme: light")).toStrictEqual(true);
  expect(css.includes("html:has(#token-shell[data-theme='light'])")).toStrictEqual(true);
  expect(css.includes("html:has(#token-shell[data-theme='oled'])")).toStrictEqual(true);
  expect(css.includes("html:has(#token-shell[data-theme='high-contrast'])")).toStrictEqual(true);
  expect(css.includes('#token-shell[data-theme=\'light\']')).toStrictEqual(true);
  const lightShell = css.slice(css.indexOf("#token-shell[data-theme='light']"));
  expect(lightShell.includes('color-scheme: light')).toStrictEqual(true);
});

test('wordmark and form controls use theme tokens and Inter, not hardcoded dark ink', async () => {
  const css = await demoShellCss();
  const face = css.slice(css.indexOf('@font-face'), css.indexOf('html,'));
  expect(face.includes("url('/fonts/InterVariable.woff2')")).toStrictEqual(true);
  expect(css.includes('fonts.googleapis.com') || css.includes('fonts.gstatic.com')).toStrictEqual(
    false,
  );
  const wordmark = css.slice(css.indexOf('#shell-wordmark'), css.indexOf('#nav-sidebar #shell-wordmark'));
  expect(wordmark.includes('var(--gm-text-primary)')).toStrictEqual(true);
  expect(wordmark.includes('#e9eef2')).toStrictEqual(false);
  expect(css.includes('#search-field')).toStrictEqual(true);
  expect(css.includes('font-family: Inter, system-ui, sans-serif !important')).toStrictEqual(true);
  expect(css.includes('--gm-text: var(--gm-text-primary)')).toStrictEqual(true);
  expect(css.includes('--gm-text: var(--gm-text-secondary)')).toStrictEqual(true);
  expect(css.includes('--gm-text: var(--gm-text-muted)')).toStrictEqual(true);
});

test('plugin slots and album license chrome use muted tokens, not a host control', async () => {
  const css = await demoShellCss();
  expect(css.includes('#settings-plugin-slots')).toStrictEqual(true);
  expect(css.includes('[data-plugin-slot]')).toStrictEqual(true);
  expect(css.includes("[data-slot-loaded='0']") || css.includes('[data-slot-loaded="0"]')).toStrictEqual(
    true,
  );
  expect(css.includes('[data-album-license]')).toStrictEqual(true);
  expect(css.includes('#search-plugin-notice')).toStrictEqual(true);
  expect(css.includes('#plugin-host') || css.includes('[data-plugin-host]')).toStrictEqual(false);
});

test('2026 chrome uses icon transport, a compact empty rail, a 360 hero and playing bars', async () => {
  const css = await demoShellCss();
  expect(css.includes('#player-prev > *')).toStrictEqual(true);
  expect(css.includes('#player-next > *')).toStrictEqual(true);
  expect(css.includes('font-size: 0 !important')).toStrictEqual(true);
  expect(css.includes('#player-prev::after') || css.includes('#player-prev:after')).toStrictEqual(
    true,
  );
  expect(css.includes('#player-next::after') || css.includes('#player-next:after')).toStrictEqual(
    true,
  );
  expect(css.includes('#player-queue::after') || css.includes('#player-queue:after')).toStrictEqual(
    true,
  );
  expect(css.includes('#player-full-prev::after') || css.includes('#player-full-prev:after')).toStrictEqual(
    true,
  );
  expect(css.includes('gm-playing-bars')).toStrictEqual(true);
  expect(css.includes('#destination-home')).toStrictEqual(true);
  expect(css.includes('repeat(3, minmax(0, 1fr))')).toStrictEqual(true);
  const continueRow = css.slice(css.indexOf('#home-row-continue'));
  expect(continueRow.includes('min-height: 64px') || continueRow.includes('min-height:64px')).toStrictEqual(
    true,
  );
  const spotlight = css.slice(css.indexOf('#home-spotlight'));
  expect(spotlight.includes('min-height: 360px')).toStrictEqual(true);
  expect(css.includes('min(400px, 72vw)') || css.includes('min(400px,72vw)')).toStrictEqual(true);
  expect(css.includes('[data-cover-plate]::after') || css.includes('[data-cover-plate]:after')).toStrictEqual(
    true,
  );
  expect(css.includes('#player-full:has([data-cover=') || css.includes('#player-full:has([data-cover')).toStrictEqual(
    true,
  );
});

test('type scale tokens and artwork mix follow canvas, not a hardcoded dark plate', async () => {
  const css = await demoShellCss();
  expect(css.includes('--gm-type-display-size: 44px')).toStrictEqual(true);
  expect(css.includes('--gm-type-title1-size: 30px')).toStrictEqual(true);
  expect(css.includes('--gm-type-title2-size: 22px')).toStrictEqual(true);
  expect(css.includes('--gm-art-mix: 48%')).toStrictEqual(true);
  expect(css.includes('--gm-art-mix: 18%')).toStrictEqual(true);
  expect(
    css.includes('color-mix(in srgb, var(--gm-cover-01) var(--gm-art-mix), var(--gm-bg-canvas))'),
  ).toStrictEqual(true);
  expect(css.includes('color-mix(in srgb, var(--gm-cover-01) 48%, #0f1317)')).toStrictEqual(false);
  expect(css.includes('::selection')).toStrictEqual(true);
});

test('full player stays inside the shell on medium and wider and overlays only on compact', async () => {
  const css = await demoShellCss();
  const compactMarker = "#token-shell[data-width='compact'] #player-full";
  const wideMarker = "#token-shell[data-width='wide'] #player-full";
  const mediumMarker = "#token-shell[data-width='medium'] #player-full";
  const expandedMarker = "#token-shell[data-width='expanded'] #player-full";
  expect(css.includes(compactMarker)).toStrictEqual(true);
  expect(css.includes(wideMarker)).toStrictEqual(true);
  expect(css.includes(mediumMarker)).toStrictEqual(true);
  expect(css.includes(expandedMarker)).toStrictEqual(true);
  const compactBlock = css.slice(css.indexOf(compactMarker), css.indexOf(compactMarker) + 280);
  expect(compactBlock.includes('position: fixed')).toStrictEqual(true);
  expect(compactBlock.includes('grid-area: content')).toStrictEqual(false);
  const wideBlock = css.slice(css.indexOf(wideMarker), css.indexOf(wideMarker) + 420);
  expect(wideBlock.includes('grid-area: content')).toStrictEqual(true);
  expect(wideBlock.includes('position: fixed')).toStrictEqual(false);
  expect(wideBlock.includes('inset: 0')).toStrictEqual(false);
  expect(wideBlock.includes('var(--gm-radius-l)')).toStrictEqual(true);
  expect(css.includes("#token-shell[data-width='wide'] #player-full-scrim")).toStrictEqual(true);
  const wideScrim = css.slice(
    css.indexOf("#token-shell[data-width='wide'] #player-full-scrim"),
    css.indexOf("#token-shell[data-width='wide'] #player-full-scrim") + 160,
  );
  expect(wideScrim.includes('display: none')).toStrictEqual(true);
});

test('home medium stack, track rows, search chips and album chrome keep their size', async () => {
  const css = await demoShellCss();
  expect(css.includes("#token-shell[data-width='medium'] #destination-home")).toStrictEqual(true);
  const mediumHome = css.slice(css.indexOf("#token-shell[data-width='medium'] #destination-home"));
  expect(mediumHome.includes('flex-direction: column')).toStrictEqual(true);
  expect(css.includes("#token-shell[data-width='medium'] #home-spotlight")).toStrictEqual(true);
  expect(css.includes("#token-shell[data-width='compact'] #home-spotlight")).toStrictEqual(true);
  const compactSpot = css.slice(css.indexOf("#token-shell[data-width='compact'] #home-spotlight"));
  expect(compactSpot.includes('min-height: 0')).toStrictEqual(true);
  expect(css.includes('height: auto')).toStrictEqual(true);
  expect(css.includes("#token-shell[data-width='compact'] [data-track-row]")).toStrictEqual(true);
  expect(css.includes('#album-lyrics-toggle')).toStrictEqual(true);
  expect(css.includes('overflow-wrap: anywhere') || css.includes('overflow-wrap:anywhere')).toStrictEqual(
    true,
  );
  const chip = css.slice(css.indexOf('[data-search-chip][data-pressed'));
  expect(chip.includes('background: transparent')).toStrictEqual(true);
  expect(chip.includes('border-radius: 999px') || chip.includes('pill')).toStrictEqual(false);
  expect(css.includes('#library-tabs')).toStrictEqual(true);
  const tabs = css.slice(css.indexOf('#library-tabs {'), css.indexOf('#library-tabs {') + 320);
  expect(tabs.includes('var(--gm-bg-canvas)')).toStrictEqual(true);
  expect(tabs.includes('backdrop-filter: blur')).toStrictEqual(false);
});
