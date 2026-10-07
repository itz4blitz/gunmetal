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

test('the shared sheet stays on tokens: no raw colours, pills, blur or image assets', async () => {
  const css = await sheet('system.css');
  expect(css.match(/#[0-9a-fA-F]{3,8}\b/g)).toStrictEqual(null);
  expect(css.includes('rgb(')).toStrictEqual(false);
  expect(css.includes('999px')).toStrictEqual(false);
  expect(css.includes('backdrop-filter')).toStrictEqual(false);
  expect(css.includes('url(')).toStrictEqual(false);
});
