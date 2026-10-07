import { cleanup, render, screen } from '@testing-library/react';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { afterEach, expect, test } from 'vitest';
import { LyricsPane } from './LyricsPane.tsx';

afterEach(cleanup);

const VERSE = ['First line stays lit', 'the rest wait their turn', 'no clock in the demo'];

async function areaLyricsCss(): Promise<string> {
  return readFile(join(process.cwd(), 'apps/demo/public/area-lyrics.css'), 'utf8');
}

test('lyrics pane stays unmounted until open and paints lines as Text', () => {
  const closed = render(
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={['First', 'Second']} synced={false} open={false} />,
  );
  expect(document.querySelector('#lyrics-pane')).toBeNull();
  closed.unmount();

  const { container } = render(
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={['This file has no lyrics.']} synced={false} open />,
  );
  expect(screen.getByRole('region', { name: 'Lyrics' }).id).toStrictEqual('lyrics-pane');
  expect(container.querySelector('#lyrics-pane')?.getAttribute('data-synced')).toStrictEqual('0');
  expect([...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent)).toStrictEqual([
    'This file has no lyrics.',
  ]);
  expect(container.querySelector('[data-current="1"]')).toBeNull();
});

test('synced lyrics highlight only the first line as a static demo', () => {
  const { container } = render(<LyricsPane id="player-full-lyrics" label="Lyrics" lines={VERSE} synced open />);
  const lines = [...container.querySelectorAll('[data-lyrics-line="1"]')];
  expect(container.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual('1');
  expect(lines.map((node) => node.textContent)).toStrictEqual([...VERSE]);
  expect(lines.map((node) => node.getAttribute('data-current'))).toStrictEqual(['1', '0', '0']);
});

test('synced lines carry a bounded distance from the current line', () => {
  const { container } = render(
    <LyricsPane
      id="lyrics-pane"
      label="Lyrics"
      lines={['one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight']}
      synced
      open
      currentLine={2}
    />,
  );
  const lines = [...container.querySelectorAll('[data-lyrics-line="1"]')];
  expect(lines.map((node) => node.getAttribute('data-current'))).toStrictEqual([
    '0',
    '0',
    '1',
    '0',
    '0',
    '0',
    '0',
    '0',
  ]);
  expect(lines.map((node) => node.getAttribute('data-distance'))).toStrictEqual([
    '2',
    '1',
    '0',
    '1',
    '2',
    '3',
    '4',
    '4',
  ]);
});

test('currentLine defaults to the first line; an out-of-range line lights nothing', () => {
  const { container } = render(<LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced open />);
  expect(
    [...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-distance')),
  ).toStrictEqual(['0', '1', '2']);

  const dimmed = render(<LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced open currentLine={9} />);
  expect(dimmed.container.querySelectorAll('[data-current="1"]')).toHaveLength(0);
  expect(
    [...dimmed.container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-distance')),
  ).toStrictEqual(['4', '4', '4']);
});

test('plain lyrics carry no distance stepping', () => {
  const { container } = render(<LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced={false} open />);
  expect(
    [...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-distance')),
  ).toStrictEqual(['0', '0', '0']);
});

test('the no-lyrics placeholder renders as quiet chrome, still plain text', () => {
  const { container } = render(
    <LyricsPane id="lyrics-pane" label="Lyrics" lines={['This file has no lyrics.']} synced={false} open empty />,
  );
  const pane = container.querySelector('#lyrics-pane');
  expect(pane?.getAttribute('data-empty')).toStrictEqual('1');
  expect(pane?.getAttribute('data-synced')).toStrictEqual('0');
  expect([...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent)).toStrictEqual([
    'This file has no lyrics.',
  ]);
  expect(container.querySelector('[data-current="1"]')).toBeNull();
});

test('empty defaults to the reading state', () => {
  const { container } = render(<LyricsPane id="lyrics-pane" label="Lyrics" lines={VERSE} synced={false} open />);
  expect(container.querySelector('#lyrics-pane')?.getAttribute('data-empty')).toStrictEqual('0');
});

test('area-lyrics.css draws the pane from type and colour tokens only', async () => {
  const css = await areaLyricsCss();
  expect(css.includes("[data-lyrics-pane='1']")).toStrictEqual(true);
  expect(css.includes('[data-lyrics-line')).toStrictEqual(true);
  expect(css.includes('var(--gm-type-title3-size)')).toStrictEqual(true);
  expect(css.includes('var(--gm-type-title3-line)')).toStrictEqual(true);
  expect(css.includes('var(--gm-text-primary)')).toStrictEqual(true);
  expect(css.includes('var(--gm-text-secondary)')).toStrictEqual(true);
  expect(css.includes('var(--gm-text-muted)')).toStrictEqual(true);
  // 2026 pass: plain lyrics read one ramp step above the synced base.
  expect(css.includes('calc(var(--gm-type-title3-size) + 1px)')).toStrictEqual(true);
  // The synced current line carries a brass where-you-are bar (§3: where you are).
  expect(css.includes('var(--gm-accent-indicator)')).toStrictEqual(true);
  // The column fades faintly at its ends to suggest continuation — a mask,
  // drawn from token colours, never an image.
  expect(css.includes('mask-image')).toStrictEqual(true);
  // No raw palette values, no assets, no emoji: tokens and text only.
  expect(/#[0-9a-fA-F]{3,8}\b/.test(css)).toStrictEqual(false);
  expect(css.includes('url(')).toStrictEqual(false);
  expect(/\p{Extended_Pictographic}/u.test(css)).toStrictEqual(false);
});

test('the active-line change runs on the motion token and stops under reduced motion', async () => {
  const css = await areaLyricsCss();
  expect(css.includes('var(--gm-motion)')).toStrictEqual(true);
  expect(css.includes('@media (prefers-reduced-motion: reduce)')).toStrictEqual(true);
  expect(css.includes('transition: none')).toStrictEqual(true);
  // The where-you-are bar fades on the same token and dies with it.
  expect(css.includes("[data-lyrics-line='1']::before")).toStrictEqual(true);
  expect(css.includes("[data-lyrics-line='1']::before {\n    transition: none")).toStrictEqual(true);
});
