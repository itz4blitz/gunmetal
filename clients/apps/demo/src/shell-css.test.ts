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
  expect(css.includes('.png') || css.includes('.svg') || css.includes('.webp')).toStrictEqual(false);
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
  // The shelf never shows a scrollbar and never fades its last tile.
  expect(css.includes('scrollbar-width: none')).toStrictEqual(true);
  expect(css.includes('[data-album-shelf]::-webkit-scrollbar')).toStrictEqual(true);
  expect(css.includes('mask-image')).toStrictEqual(false);
});

test('home spotlight wash uses art-surface vars keyed by data-art-tone', async () => {
  const css = await demoShellCss();
  expect(css.includes('#destination-home[data-art-tone]')).toStrictEqual(true);
  expect(css.includes('--art-surface')).toStrictEqual(true);
  expect(css.includes('#home-spotlight')).toStrictEqual(true);
  const spotlight = css.slice(css.indexOf('#home-spotlight'));
  expect(spotlight.includes('min-height: 360px')).toStrictEqual(true);
  expect(css.includes('[data-spotlight-eyebrow]')).toStrictEqual(true);
  expect(css.includes("[data-type='display']") || css.includes('[data-type="display"]')).toStrictEqual(true);
});

test('queue sheet enters and exits over 200ms and respects reduced motion', async () => {
  const css = await demoShellCss();
  expect(css.includes('#queue-sheet[data-queue-open=')).toStrictEqual(true);
  expect(css.includes('200ms')).toStrictEqual(true);
  expect(css.includes('prefers-reduced-motion: reduce')).toStrictEqual(true);
  // A full-height side sheet that slides from the right, and a bottom sheet on
  // phones — not a floating popover card.
  const sheet = css.slice(css.indexOf('#queue-sheet {'), css.indexOf('#queue-scrim'));
  expect(sheet.includes('translateX(105%)')).toStrictEqual(true);
  expect(sheet.includes('border-radius: 0')).toStrictEqual(true);
  const compactSheet = css.slice(
    css.indexOf("#token-shell[data-width='compact'] #queue-sheet {"),
    css.indexOf("#token-shell[data-width='compact'] #queue-sheet[data-queue-open='1']"),
  );
  expect(compactSheet.includes('border-radius: var(--gm-radius-xl)')).toStrictEqual(true);
  expect(compactSheet.includes('translateY(105%)')).toStrictEqual(true);
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
  expect(css.includes('#player-bar::before') || css.includes('#player-bar:before')).toStrictEqual(true);
  expect(css.includes('#demo-label')).toStrictEqual(true);
  expect(css.includes("data-width='wide'] #content") || css.includes('data-width="wide"] #content')).toStrictEqual(
    true,
  );
  expect(css.includes('padding: 32px')).toStrictEqual(true);
  // The wide content column fills the space to the queue pane (no dead zone).
  expect(css.includes('max-width: 1120px')).toStrictEqual(false);
  // Opaque player bar: no translucency, no blur (design-language §2, §7).
  expect(css.includes('backdrop-filter: blur')).toStrictEqual(false);
  expect(css.includes('clip-path')).toStrictEqual(true);
  // Selected nav paints from data-selected; RN-web does not emit aria-selected.
  expect(css.includes("#nav-sidebar [data-selected='1']")).toStrictEqual(true);
  // Brass stays on play and where-you-are: the demo chip and empty marks go steel.
  const demoLabel = css.slice(css.indexOf('#demo-label'), css.indexOf('}', css.indexOf('#demo-label')));
  expect(demoLabel.includes('var(--gm-text-muted)')).toStrictEqual(true);
  expect(demoLabel.includes('--gm-accent')).toStrictEqual(false);
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
  expect(css.includes('clip-path: polygon(25% 6%, 75% 6%, 100% 50%, 75% 94%, 25% 94%, 0% 50%)')).toStrictEqual(true);
  const artistRow = css.slice(css.indexOf('[data-artist-row]'));
  expect(artistRow.includes('min-height: 56px') || artistRow.includes('height: 56px')).toStrictEqual(true);
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
  expect(css.includes("[data-settings-layout='side']") || css.includes('[data-settings-layout="side"]')).toStrictEqual(
    true,
  );
  const sideMarker = css.includes("[data-settings-layout='side']")
    ? "[data-settings-layout='side']"
    : '[data-settings-layout="side"]';
  const side = css.slice(css.indexOf(sideMarker));
  expect(side.includes('flex-direction: row')).toStrictEqual(true);
  expect(css.includes("[data-settings-badge='R2']") || css.includes('[data-settings-badge="R2"]')).toStrictEqual(true);
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
  expect(css.includes('scrollbar-width: thin') || css.includes('::-webkit-scrollbar')).toStrictEqual(true);
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
  expect(
    css.includes('[data-lyrics-line][data-current') || css.includes('[data-lyrics-line][data-current'),
  ).toStrictEqual(true);
});

test('album detail wash denser track rows and full player sheet are crafted', async () => {
  const css = await demoShellCss();
  expect(css.includes('#destination-album[data-art-tone]')).toStrictEqual(true);
  expect(css.includes('min-height: 48px')).toStrictEqual(true);
  expect(css.includes('[data-now-playing=') || css.includes('[data-now-playing=')).toStrictEqual(true);
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
  expect(css.includes("[data-size='full']") || css.includes('[data-size="full"]')).toStrictEqual(true);
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
  expect(css.includes('color-scheme: dark')).toStrictEqual(true);
  expect(css.includes('color-scheme: light')).toStrictEqual(true);
  expect(css.includes("html:has(#token-shell[data-theme='light'])")).toStrictEqual(true);
  expect(css.includes("html:has(#token-shell[data-theme='oled'])")).toStrictEqual(true);
  expect(css.includes("html:has(#token-shell[data-theme='high-contrast'])")).toStrictEqual(true);
  expect(css.includes("#token-shell[data-theme='light']")).toStrictEqual(true);
  const lightShell = css.slice(css.indexOf("#token-shell[data-theme='light']"));
  expect(lightShell.includes('color-scheme: light')).toStrictEqual(true);
});

test('wordmark and form controls use theme tokens and Inter, not hardcoded dark ink', async () => {
  const css = await demoShellCss();
  const face = css.slice(css.indexOf('@font-face'), css.indexOf('html,'));
  expect(face.includes("url('/fonts/InterVariable.woff2')")).toStrictEqual(true);
  expect(css.includes('fonts.googleapis.com') || css.includes('fonts.gstatic.com')).toStrictEqual(false);
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
  expect(css.includes("[data-slot-loaded='0']") || css.includes('[data-slot-loaded="0"]')).toStrictEqual(true);
  expect(css.includes('[data-album-license]')).toStrictEqual(true);
  expect(css.includes('#search-plugin-notice')).toStrictEqual(true);
  expect(css.includes('#plugin-host') || css.includes('[data-plugin-host]')).toStrictEqual(false);
});

test('2026 chrome uses icon transport, fills the column, and keeps a composed home', async () => {
  const css = await demoShellCss();
  expect(css.includes('#player-prev > *')).toStrictEqual(true);
  expect(css.includes('#player-next > *')).toStrictEqual(true);
  expect(css.includes('font-size: 0 !important')).toStrictEqual(true);
  expect(css.includes('#player-prev::after') || css.includes('#player-prev:after')).toStrictEqual(true);
  expect(css.includes('#player-next::after') || css.includes('#player-next:after')).toStrictEqual(true);
  // The queue icon is an inline SVG; the legacy glyph stays gated off.
  expect(css.includes('#player-queue:not(:has(svg))::after')).toStrictEqual(true);
  expect(css.includes('#player-full-prev::after') || css.includes('#player-full-prev:after')).toStrictEqual(true);
  expect(css.includes('gm-playing-bars')).toStrictEqual(true);
  expect(css.includes('#destination-home')).toStrictEqual(true);
  // Home is one composed column: hero, then shelves. No placeholder trio.
  const home = css.slice(css.indexOf('#destination-home'), css.indexOf('}', css.indexOf('#destination-home')));
  expect(home.includes('flex-direction: column')).toStrictEqual(true);
  expect(css.includes('repeat(3, minmax(0, 1fr))')).toStrictEqual(false);
  expect(css.includes('#home-row-continue')).toStrictEqual(false);
  const spotlight = css.slice(css.indexOf('#home-spotlight'));
  expect(spotlight.includes('min-height: 360px')).toStrictEqual(true);
  expect(css.includes('min(400px, 72vw)') || css.includes('min(400px,72vw)')).toStrictEqual(true);
  expect(css.includes('[data-cover-plate]::after') || css.includes('[data-cover-plate]:after')).toStrictEqual(true);
  expect(css.includes('#player-full:has([data-cover=') || css.includes('#player-full:has([data-cover')).toStrictEqual(
    true,
  );
  // Cover placeholders are machined plates, not letter posters: grid covers
  // carry no glyph and no sheen layer.
  const gridGlyph = css.indexOf("[data-size='grid'] [data-cover-label]");
  expect(gridGlyph).toStrictEqual(-1);
  expect(css.includes('[data-cover-sheen]')).toStrictEqual(false);
  // The queue sheet is a real sheet with a scrim behind it.
  expect(css.includes('#queue-scrim')).toStrictEqual(true);
  // Hex buttons take their focus ring on a square wrapper (clip-path eats outlines).
  // The hex plate's ring is keyboard-only: a mouse click never draws it.
  expect(css.includes("[data-hex-wrap='1']:has(:focus-visible)")).toStrictEqual(true);
  expect(css.includes("[data-hex-wrap='1']:focus-within")).toStrictEqual(false);
  expect(css.includes('drop-shadow(0 0 0')).toStrictEqual(false);
  // The bar is three zones with a capped, centred column; the empty state
  // sleeps the centre zone instead of unmounting it.
  expect(css.includes("grid-template-areas: 'left center right'")).toStrictEqual(true);
  expect(css.includes('#player-bar[data-bar-empty=')).toStrictEqual(true);
  expect(css.includes("#player-bar[data-bar-empty='1'] #player-center")).toStrictEqual(true);
  expect(css.includes('#player-art-empty')).toStrictEqual(true);
});

test('type scale tokens and artwork mix follow canvas, not a hardcoded dark plate', async () => {
  const css = await demoShellCss();
  expect(css.includes('--gm-type-display-size: 44px')).toStrictEqual(true);
  expect(css.includes('--gm-type-title1-size: 30px')).toStrictEqual(true);
  expect(css.includes('--gm-type-title2-size: 22px')).toStrictEqual(true);
  expect(css.includes('--gm-art-mix: 48%')).toStrictEqual(true);
  expect(css.includes('--gm-art-mix: 18%')).toStrictEqual(true);
  expect(css.includes('color-mix(in srgb, var(--gm-cover-01) var(--gm-art-mix), var(--gm-bg-canvas))')).toStrictEqual(
    true,
  );
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
  expect(css.includes('overflow-wrap: anywhere') || css.includes('overflow-wrap:anywhere')).toStrictEqual(true);
  const chip = css.slice(css.indexOf('[data-search-chip][data-pressed'));
  expect(chip.includes('background: transparent')).toStrictEqual(true);
  expect(chip.includes('border-radius: 999px') || chip.includes('pill')).toStrictEqual(false);
  expect(css.includes('#library-tabs')).toStrictEqual(true);
  const tabs = css.slice(css.indexOf('#library-tabs {'), css.indexOf('#library-tabs {') + 320);
  expect(tabs.includes('var(--gm-bg-canvas)')).toStrictEqual(true);
  expect(tabs.includes('backdrop-filter: blur')).toStrictEqual(false);
});

test('2026 elevation tokens layer inner strokes and keep soft shadows light-only', async () => {
  const css = await demoShellCss();
  // Dark: white inner strokes at 4-5%, and no soft shadow layers.
  expect(css.includes('--gm-stroke-soft: inset 0 0 0 1px color-mix(in srgb, #ffffff 4%, transparent)')).toStrictEqual(
    true,
  );
  expect(css.includes('--gm-stroke-hover: inset 0 0 0 1px color-mix(in srgb, #ffffff 10%, transparent)')).toStrictEqual(
    true,
  );
  expect(css.includes('--gm-stroke-edge: inset 0 0 0 1px color-mix(in srgb, #ffffff 5%, transparent)')).toStrictEqual(
    true,
  );
  expect(css.includes('--gm-shadow-float: 0 0 0 0 transparent')).toStrictEqual(true);
  expect(
    css.includes('--gm-elev-raised: var(--gm-raised-edge), var(--gm-stroke-edge), var(--gm-shadow-surface)'),
  ).toStrictEqual(true);
  expect(
    css.includes('--gm-elev-overlay: var(--gm-raised-edge), var(--gm-stroke-edge), var(--gm-shadow-float)'),
  ).toStrictEqual(true);
  // Light: stronger ink strokes, plus the only large soft shadows in the file.
  expect(css.includes('--gm-stroke-edge: inset 0 0 0 1px color-mix(in srgb, #16202a 8%, transparent)')).toStrictEqual(
    true,
  );
  expect(css.includes('--gm-shadow-float: 0 24px 56px color-mix(in srgb, #1a2228 18%, transparent)')).toStrictEqual(
    true,
  );
  // Menus, sheets and the pane player float on tokens; no hardcoded shadows.
  expect(css.includes('0 12px 32px')).toStrictEqual(false);
  expect(css.includes('0 18px 40px')).toStrictEqual(false);
  // The queue sheet is a raised surface of the overlay class, not a shadowless slab.
  const sheetStart = css.indexOf('#queue-sheet {', css.indexOf('Queue sheet: a real sheet'));
  const sheetBlock = css.slice(sheetStart, css.indexOf('}', sheetStart) + 1);
  expect(sheetBlock.includes('box-shadow: var(--gm-elev-overlay)')).toStrictEqual(true);
  // High contrast separates with borders: strokes and shadows switch off.
  expect(css.includes('--gm-stroke-edge: 0 0 0 0 transparent')).toStrictEqual(true);
});

test('2026 motion tokens: 160 base, 200 sheet, 180 lift, one standard curve', async () => {
  const css = await demoShellCss();
  expect(css.includes('--gm-motion: 160ms cubic-bezier(0.2, 0, 0, 1)')).toStrictEqual(true);
  expect(css.includes('--gm-motion-sheet: 200ms cubic-bezier(0.2, 0, 0, 1)')).toStrictEqual(true);
  expect(css.includes('--gm-motion-lift: 180ms cubic-bezier(0.2, 0, 0, 1)')).toStrictEqual(true);
  expect(css.includes('--gm-ease: cubic-bezier(0.2, 0, 0, 1)')).toStrictEqual(true);
  const reduced = css.slice(css.indexOf('prefers-reduced-motion: reduce'));
  expect(reduced.includes('--gm-motion-lift: 0ms')).toStrictEqual(true);
});

test('the now-playing bar carries an ambient artwork wash beneath its content', async () => {
  const css = await demoShellCss();
  expect(css.includes('#player-bar::after')).toStrictEqual(true);
  const wash = css.slice(css.indexOf('#player-bar::after'));
  expect(wash.includes('var(--gm-now-tone')).toStrictEqual(true);
  expect(wash.includes('var(--gm-bar-wash')).toStrictEqual(true);
  expect(wash.includes('z-index: 0')).toStrictEqual(true);
  expect(wash.includes('pointer-events: none')).toStrictEqual(true);
  // The bar's zones lift above the wash, and the tone lands on the bar itself
  // so both painted layers read it.
  expect(css.includes('#player-bar > *')).toStrictEqual(true);
  expect(css.includes("#token-shell[data-art-tone='01'] #player-bar {")).toStrictEqual(true);
  // High contrast takes no artwork tint at all: both painted layers switch off.
  const hc = css.indexOf("#token-shell[data-theme='high-contrast'] #player-bar::before");
  expect(hc).toBeGreaterThanOrEqual(0);
  const hcBlock = css.slice(hc, css.indexOf('}', hc) + 1);
  expect(hcBlock.includes('content: none')).toStrictEqual(true);
});

test('long bar titles marquee only when overflowing, pause on hover, and keep the ellipsis under reduced motion', async () => {
  const css = await demoShellCss();
  expect(css.includes('--gm-title-shift')).toStrictEqual(true);
  expect(css.includes('@keyframes gm-title-marquee')).toStrictEqual(true);
  const marqueeStart = css.indexOf("#player-title[data-marquee='1']");
  expect(marqueeStart).toBeGreaterThanOrEqual(0);
  const marquee = css.slice(marqueeStart);
  expect(marquee.includes('width: max-content')).toStrictEqual(true);
  expect(marquee.includes('gm-title-marquee')).toStrictEqual(true);
  expect(css.includes('#player-meta:hover #player-title[data-marquee')).toStrictEqual(true);
  expect(css.includes('animation-play-state: paused')).toStrictEqual(true);
  const reduced = css.slice(css.indexOf('prefers-reduced-motion: reduce'));
  expect(reduced.includes("data-marquee='1']")).toStrictEqual(true);
  expect(reduced.includes('text-overflow: ellipsis')).toStrictEqual(true);
});

test('queue lines reveal a play action that replaces the duration on hover and focus', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-queue-play]')).toStrictEqual(true);
  const playStart = css.indexOf('[data-queue-play]');
  const play = css.slice(playStart);
  expect(play.includes('grid-area: duration')).toStrictEqual(true);
  expect(play.includes('opacity: 0')).toStrictEqual(true);
  expect(css.includes('[data-queue-line]:hover [data-queue-play]')).toStrictEqual(true);
  expect(css.includes('[data-queue-line]:focus-within [data-queue-play]')).toStrictEqual(true);
  expect(css.includes("#token-shell[data-width='compact'] [data-queue-play]")).toStrictEqual(true);
  expect(css.includes('[data-queue-line]:hover [data-queue-duration]')).toStrictEqual(true);
});

test('tab underlines slide on transforms and focus rings animate in on focus-visible only', async () => {
  const css = await demoShellCss();
  expect(css.includes("#nav-tabs [id^='nav-item-']::after")).toStrictEqual(true);
  expect(css.includes('transform: scaleX(0)')).toStrictEqual(true);
  expect(css.includes('transform: scaleX(1)')).toStrictEqual(true);
  // The compact tab indicator is the sliding underline now, not a box shadow.
  expect(css.includes('inset 0 -3px 0 var(--gm-accent-indicator)')).toStrictEqual(false);
  expect(css.includes('@keyframes gm-underline-in')).toStrictEqual(true);
  // Base focus paints nothing; focus-visible draws and settles the ring.
  expect(css.includes('outline: 2px solid transparent')).toStrictEqual(true);
  expect(css.includes('outline-offset: 4px')).toStrictEqual(true);
  expect(css.includes('outline-color var(--gm-motion),')).toStrictEqual(true);
  expect(css.includes('outline-offset var(--gm-motion);')).toStrictEqual(true);
  expect(css.includes(':focus-visible')).toStrictEqual(true);
});

test('scrubber thumb grows with a brass halo on hover and while dragging', async () => {
  const css = await demoShellCss();
  expect(css.includes('#player-scrubber:active #player-progress-fill::after')).toStrictEqual(true);
  expect(css.includes('0 0 0 7px color-mix(in srgb, var(--gm-accent-fill) 18%, transparent)')).toStrictEqual(true);
});

test('the full player floats over an ambient artwork backdrop under a contrast veil', async () => {
  const css = await demoShellCss();
  expect(css.includes('#player-full-ambient')).toStrictEqual(true);
  const ambient = css.slice(css.indexOf('#player-full-ambient'));
  expect(ambient.includes('filter: blur(56px)')).toStrictEqual(true);
  expect(ambient.includes('z-index: 0')).toStrictEqual(true);
  expect(css.includes('[data-ambient-veil]')).toStrictEqual(true);
  const veil = css.slice(css.indexOf('[data-ambient-veil]'));
  expect(veil.includes('var(--gm-bg-canvas) 82%, transparent) 0%')).toStrictEqual(true);
  expect(veil.includes('var(--gm-bg-canvas) 93%, transparent) 100%')).toStrictEqual(true);
  expect(css.includes('#player-full > :not(#player-full-ambient)')).toStrictEqual(true);
  expect(css.includes("#token-shell[data-theme='high-contrast'] #player-full-ambient")).toStrictEqual(true);
});

test('interactive chrome presses at 0.97 with washes, strokes and machined art ring', async () => {
  const css = await demoShellCss();
  expect(css.includes('[data-queue-play]:active')).toStrictEqual(true);
  expect(css.includes('scale(0.97)')).toStrictEqual(true);
  // The bar's art well carries a machined ring that tints while playing.
  expect(css.includes("#player-art[data-playing='1'] [data-size='bar']")).toStrictEqual(true);
  expect(css.includes("#player-art [data-size='bar']") || css.includes('[data-size="bar"]')).toStrictEqual(true);
});
