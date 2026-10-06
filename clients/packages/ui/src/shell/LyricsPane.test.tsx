import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { LyricsPane } from './LyricsPane.tsx';

afterEach(cleanup);

test('lyrics pane stays unmounted until open and paints lines as Text', () => {
  const closed = render(
    <LyricsPane
      id="lyrics-pane"
      label="Lyrics"
      lines={['First', 'Second']}
      synced={false}
      open={false}
    />,
  );
  expect(document.querySelector('#lyrics-pane')).toBeNull();
  closed.unmount();

  const { container } = render(
    <LyricsPane
      id="lyrics-pane"
      label="Lyrics"
      lines={['This file has no lyrics.']}
      synced={false}
      open
    />,
  );
  expect(screen.getByRole('region', { name: 'Lyrics' }).id).toStrictEqual('lyrics-pane');
  expect(container.querySelector('#lyrics-pane')?.getAttribute('data-synced')).toStrictEqual('0');
  expect(
    [...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent),
  ).toStrictEqual(['This file has no lyrics.']);
  expect(container.querySelector('[data-current="1"]')).toBeNull();
});

test('synced lyrics highlight only the first line as a static demo', () => {
  const { container } = render(
    <LyricsPane
      id="player-full-lyrics"
      label="Lyrics"
      lines={['First line stays lit', 'the rest wait their turn', 'no clock in the demo']}
      synced
      open
    />,
  );
  const lines = [...container.querySelectorAll('[data-lyrics-line="1"]')];
  expect(container.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual(
    '1',
  );
  expect(lines.map((node) => node.textContent)).toStrictEqual([
    'First line stays lit',
    'the rest wait their turn',
    'no clock in the demo',
  ]);
  expect(lines.map((node) => node.getAttribute('data-current'))).toStrictEqual(['1', '0', '0']);
});
