import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from 'vitest';

async function sheet(name: string): Promise<string> {
  return readFile(join(process.cwd(), 'apps/demo/public', name), 'utf8');
}

/** The declarations of the first rule whose selector list contains `selector`. */
function ruleBody(css: string, selector: string): string {
  const at = css.indexOf(selector);
  if (at < 0) {
    return '';
  }
  const open = css.indexOf('{', at);
  return css.slice(open + 1, css.indexOf('}', open));
}

test('the page loads the shared primitives last, after every area sheet', async () => {
  const html = await readFile(join(process.cwd(), 'apps/demo/index.html'), 'utf8');
  const sheets = [...html.matchAll(/<link rel="stylesheet" href="\/([a-z-]+\.css)" \/>/g)].map((match) => match[1]);
  expect(sheets).toStrictEqual([
    'shell.css',
    'area-home.css',
    'area-album.css',
    'area-library.css',
    'area-settings.css',
    'area-lyrics.css',
    'area-player.css',
    'area-video.css',
    'system.css',
  ]);
});

test('the nut and its glyphs are defined once, as tokens, and every play surface uses them', async () => {
  const css = await sheet('system.css');
  expect(css.split('--gm-nut: polygon(').length - 1).toStrictEqual(1);
  expect(css.split('--gm-glyph-play: polygon(').length - 1).toStrictEqual(1);
  expect(css.split('--gm-glyph-pause: polygon(').length - 1).toStrictEqual(1);
  // One rule shapes every play control; no surface redraws the hexagon.
  const nut = ruleBody(css, "#shell-play[data-player-control='primary'],");
  expect(nut.includes('clip-path: var(--gm-nut);')).toStrictEqual(true);
  expect(nut.includes('background: var(--gm-nut-face);')).toStrictEqual(true);
  for (const surface of [
    '#home-spotlight-play',
    '#album-play',
    '#artist-play',
    '[data-album-play]',
    '[data-artist-play]',
  ]) {
    expect(css.slice(0, css.indexOf('clip-path: var(--gm-nut);')).includes(surface)).toStrictEqual(true);
  }
  // The old sharp hexagon literal appears in none of the sheets that restyle it.
  const sharp = 'polygon(25% 6%, 75% 6%, 100% 50%, 75% 94%, 25% 94%, 0% 50%)';
  expect(css.includes(sharp)).toStrictEqual(false);
  expect((await sheet('area-album.css')).includes(sharp)).toStrictEqual(false);
  expect((await sheet('area-home.css')).includes(sharp)).toStrictEqual(false);
  expect((await sheet('area-player.css')).includes(sharp)).toStrictEqual(false);
  // People and cover marks take the same shape.
  const people = ruleBody(css, '#token-shell [data-artist-avatar],');
  expect(people.trim()).toStrictEqual('clip-path: var(--gm-nut);');
});

test('a mouse click can never flash a focus ring: controls rest with a transparent outline', async () => {
  const css = await sheet('system.css');
  // Zero specificity, so every deliberate ring still wins over it.
  const resting = ruleBody(css, ':where(#token-shell) :where([tabindex], input, button, select, textarea) {');
  expect(resting.includes('outline: 2px solid transparent;')).toStrictEqual(true);
  expect(resting.includes('outline-offset: 2px;')).toStrictEqual(true);
  // Keyboard focus always gets a ring, even on a control no sheet styled.
  const keyboard = ruleBody(
    css,
    ':where(#token-shell) :is([tabindex], input, button, select, textarea):focus-visible {',
  );
  expect(keyboard.includes('outline: 2px solid var(--gm-focus-ring);')).toStrictEqual(true);
  // The landmark panes ring inside themselves, not over the sidebar and bar.
  const landmarks = ruleBody(css, '#token-shell #content:focus-visible,');
  expect(landmarks.includes('outline-offset: -2px;')).toStrictEqual(true);
  // Nothing here animates an outline in on plain :focus.
  expect(css.includes('outline-color var(--gm-motion)')).toStrictEqual(false);
});

test('the frame reads both pane widths from tokens and the edges follow them', async () => {
  const css = await sheet('system.css');
  const tokens = ruleBody(css, '#token-shell {');
  expect(tokens.includes('--gm-sidebar-w: 256px;')).toStrictEqual(true);
  expect(tokens.includes('--gm-queue-w: 340px;')).toStrictEqual(true);
  expect(ruleBody(css, "#token-shell[data-width='expanded'] #shell-frame {").trim()).toStrictEqual(
    'grid-template-columns: var(--gm-sidebar-w) minmax(0, 1fr);',
  );
  expect(ruleBody(css, "#token-shell[data-width='wide'] #shell-frame {").trim()).toStrictEqual(
    'grid-template-columns: var(--gm-sidebar-w) minmax(0, 1fr) var(--gm-queue-w);',
  );
  expect(ruleBody(css, "[data-pane-resizer='sidebar'] {").trim()).toStrictEqual(
    'left: calc(var(--gm-sidebar-w) - 6px);',
  );
  const queueEdge = ruleBody(css, "[data-pane-resizer='queue'] {");
  expect(queueEdge.includes('right: calc(var(--gm-queue-w) - 5px);')).toStrictEqual(true);
  expect(queueEdge.includes('bottom: var(--gm-player-height);')).toStrictEqual(true);
  // The grab target reaches the 44px touch floor (design-language §10): an
  // invisible strip on the separator carries the pointer; the painted grip
  // stays a 2px hairline.
  const strip = ruleBody(css, '[data-pane-resizer]::before {');
  expect(strip.includes("content: '';")).toStrictEqual(true);
  expect(strip.includes('position: absolute;')).toStrictEqual(true);
  expect(strip.includes('width: 44px;')).toStrictEqual(true);
  expect(ruleBody(css, '[data-pane-resizer-grip] {').includes('width: 2px;')).toStrictEqual(true);
});

test('the phone bar keeps play and next, and truncates the title instead of scrolling it', async () => {
  const css = await sheet('system.css');
  // player.md, phone-width: play or pause and next on the right. Shuffle,
  // previous, repeat and the queue live in the full player.
  const hidden = ruleBody(
    css,
    "#token-shell[data-width='compact'] :is(#player-shuffle, #player-repeat, #player-queue) {",
  );
  expect(hidden.includes('display: none;')).toStrictEqual(true);
  // RN Web writes flex-shrink: 0 inline, so the meta has to win with
  // !important or the title paints across the transport.
  const meta = ruleBody(css, "#token-shell[data-width='compact'] #player-meta {");
  expect(meta.includes('flex-shrink: 1 !important;')).toStrictEqual(true);
  expect(meta.includes('overflow: hidden;')).toStrictEqual(true);
  const title = ruleBody(css, "#token-shell[data-width='compact'] #player-title[data-marquee='1'] {");
  expect(title.includes('animation: none;')).toStrictEqual(true);
  expect(title.includes('text-overflow: ellipsis;')).toStrictEqual(true);
  expect(title.includes('overflow: hidden;')).toStrictEqual(true);
  const artist = ruleBody(css, "#token-shell[data-width='compact'] #player-artist {");
  expect(artist.includes('text-overflow: ellipsis;')).toStrictEqual(true);
  expect(artist.includes('white-space: nowrap;')).toStrictEqual(true);
  // 320px reflow: the full-player transport gap shrinks so the nut and
  // the four plain controls stay inside the column.
  const transport = ruleBody(css, "#token-shell[data-width='compact'] #player-full-transport {");
  expect(transport.includes('gap: clamp(4px, calc((var(--pf-col) - 256px) / 4), 28px);')).toStrictEqual(true);
});

test('an open menu is lifted above its neighbours and the bottom edge is one line', async () => {
  const css = await sheet('system.css');
  expect(ruleBody(css, '#token-shell #content :has([data-context-menu]) {').trim()).toStrictEqual('z-index: 30;');
  const footer = ruleBody(
    css,
    "#token-shell:is([data-width='expanded'], [data-width='wide']) #nav-sidebar #nav-footer,",
  );
  expect(footer.includes('height: var(--gm-player-height);')).toStrictEqual(true);
  expect(footer.includes('border-top: 1px solid var(--gm-line-subtle);')).toStrictEqual(true);
});

test('the §7 space scale is defined once on the shell root and put to use', async () => {
  const css = await sheet('system.css');
  const tokens = ruleBody(css, '#token-shell {');
  const scale: readonly (readonly [string, string])[] = [
    ['--gm-space-0', '0px'],
    ['--gm-space-1', '2px'],
    ['--gm-space-2', '4px'],
    ['--gm-space-3', '8px'],
    ['--gm-space-4', '12px'],
    ['--gm-space-5', '16px'],
    ['--gm-space-6', '20px'],
    ['--gm-space-7', '24px'],
    ['--gm-space-8', '32px'],
    ['--gm-space-9', '40px'],
    ['--gm-space-10', '48px'],
    ['--gm-space-11', '64px'],
    ['--gm-space-12', '96px'],
  ];
  for (const [name, value] of scale) {
    // One definition, like the nut: never re-declared per rule.
    expect(css.split(`${name}:`).length - 1).toStrictEqual(1);
    expect(tokens.includes(`${name}: ${value};`)).toStrictEqual(true);
  }
  // The tokens exist to be used: the sheets replace raw px with them.
  expect(css.includes('var(--gm-space-')).toStrictEqual(true);
});

test('every motion in the shared sheet resolves to instant and forced colours stay navigable', async () => {
  const css = await sheet('system.css');
  const reduced = css.slice(css.indexOf('prefers-reduced-motion: reduce'));
  expect(reduced.includes('[data-pane-resizer-grip]')).toStrictEqual(true);
  expect(reduced.includes('transition: none;')).toStrictEqual(true);
  // Forced colours (Windows High Contrast, design-language §4): system
  // colours replace the palette, and the shell's own edges must survive.
  const forced = css.slice(css.indexOf('forced-colors: active'));
  expect(forced).not.toStrictEqual('');
  expect(forced.includes('ButtonText')).toStrictEqual(true);
  expect(forced.includes('Highlight')).toStrictEqual(true);
});

test('the shared sheet stays on tokens: no raw colours, pills, blur or image assets', async () => {
  const css = await sheet('system.css');
  expect(css.match(/#[0-9a-fA-F]{3,8}\b/g)).toStrictEqual(null);
  expect(css.includes('rgb(')).toStrictEqual(false);
  expect(css.includes('999px')).toStrictEqual(false);
  expect(css.includes('backdrop-filter')).toStrictEqual(false);
  expect(css.includes('url(')).toStrictEqual(false);
  // Status colours are theme-owned (shell.css theme blocks); the shared
  // sheet consumes tokens, it never declares theme values.
  expect(css.includes('--gm-status-')).toStrictEqual(false);
});
