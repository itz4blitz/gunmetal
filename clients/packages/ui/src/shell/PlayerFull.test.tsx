import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { shellMessages } from '../messages/en/shell.ts';
import { emptyPlayback, type PlaybackSnapshot } from './playback.ts';
import { PlayerFull } from './PlayerFull.tsx';

afterEach(cleanup);

function playingSnapshot(): PlaybackSnapshot {
  return {
    trackId: 'demo-track-01-01',
    albumId: 'demo-album-01',
    title: 'Pier at Dusk',
    artistName: 'Mira Sol',
    coverTone: '01',
    playing: true,
    positionMs: 45_000,
    durationMs: 180_000,
    lyricsKind: 'none',
    queue: [],
    queueOpen: false,
  };
}

test('full player stays unmounted until open and ignores empty playback', () => {
  const closed = render(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open={false}
      onClose={vi.fn()}
    />,
  );
  expect(document.querySelector('#player-full')).toBeNull();
  expect(document.querySelector('#player-full-scrim')).toBeNull();
  closed.unmount();

  const empty = render(
    <PlayerFull
      messages={shellMessages()}
      playback={emptyPlayback()}
      open
      onClose={vi.fn()}
    />,
  );
  expect(document.querySelector('#player-full')).toBeNull();
  empty.unmount();
});

test('open full player shows cover title artist scrubber transport and close', () => {
  const onClose = vi.fn();
  const onPlayPause = vi.fn();
  const onPrevious = vi.fn();
  const onNext = vi.fn();
  const { container, rerender } = render(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      onClose={onClose}
      onPlayPause={onPlayPause}
      onPrevious={onPrevious}
      onNext={onNext}
    />,
  );
  const sheet = container.querySelector('#player-full');
  expect(sheet?.getAttribute('data-open')).toStrictEqual('1');
  expect(sheet?.getAttribute('aria-label')).toStrictEqual('Full player');
  expect(screen.getByText('Pier at Dusk').id).toStrictEqual('player-full-title');
  expect(screen.getByText('Mira Sol').id).toStrictEqual('player-full-artist');
  expect(container.querySelector('#player-full-art [data-size="full"]')).toBeTruthy();
  expect(container.querySelector('#player-full-progress')?.getAttribute('data-progress')).toStrictEqual(
    '25',
  );
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), positionMs: 10, durationMs: 0 }}
      open
      onClose={onClose}
      onPlayPause={onPlayPause}
      onPrevious={onPrevious}
      onNext={onNext}
    />,
  );
  expect(container.querySelector('#player-full-progress')?.getAttribute('data-progress')).toStrictEqual(
    '0',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  expect(onClose).toHaveBeenCalledTimes(1);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close' }), { key: 'Enter' });
  expect(onClose).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close' }), { key: ' ' });
  expect(onClose).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close' }), { key: 'Tab' });
  expect(onClose).toHaveBeenCalledTimes(3);
  fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Previous' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Next' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Pause' }), { key: 'Tab' });
  expect(onPlayPause).toHaveBeenCalledTimes(1);
  expect(onPrevious).toHaveBeenCalledTimes(1);
  expect(onNext).toHaveBeenCalledTimes(1);
});

test('full player transport tolerates missing optional handlers', () => {
  render(
    <PlayerFull messages={shellMessages()} playback={playingSnapshot()} open onClose={vi.fn()} />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
  fireEvent.click(screen.getByRole('button', { name: 'Previous' }));
  fireEvent.click(screen.getByRole('button', { name: 'Next' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Pause' }), { key: 'Enter' });
  expect(screen.getByRole('button', { name: 'Pause' }).id).toStrictEqual('player-full-play');
});

test('Escape and scrim dismiss the full player while other keys do not', () => {
  const onClose = vi.fn();
  render(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      onClose={onClose}
    />,
  );
  fireEvent.keyDown(window, { key: 'Tab' });
  expect(onClose).toHaveBeenCalledTimes(0);
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(onClose).toHaveBeenCalledTimes(1);
  fireEvent.click(document.querySelector('#player-full-scrim')!);
  expect(onClose).toHaveBeenCalledTimes(2);
});

test('full player lyrics toggle appears for plain and synced kinds and paints Text lines', () => {
  const onClose = vi.fn();
  const { rerender } = render(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), lyricsKind: 'plain', trackId: 'demo-track-01-03' }}
      open
      onClose={onClose}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  expect(document.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual('0');
  expect(
    [...document.querySelectorAll('#player-full-lyrics [data-lyrics-line="1"]')].map((node) => node.textContent),
  ).toStrictEqual([
    'The harbour keeps the letter',
    'folded under glass',
    'until the tide comes back',
  ]);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Enter' });
  expect(document.querySelector('#player-full-lyrics')).toBeNull();
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), lyricsKind: 'synced', trackId: 'demo-track-02-02' }}
      open
      onClose={onClose}
    />,
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: ' ' });
  expect(document.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual('1');
  expect(document.querySelector('#player-full-lyrics [data-current="1"]')?.textContent).toStrictEqual(
    'Floors count themselves in the dark',
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Tab' });
  rerender(
    <PlayerFull messages={shellMessages()} playback={playingSnapshot()} open onClose={onClose} />,
  );
  expect(screen.getByRole('button', { name: 'Lyrics' }).getAttribute('data-lyrics-available')).toStrictEqual(
    '0',
  );
  expect(document.querySelector('#player-full-lyrics')).toBeNull();
  expect(document.querySelector('#player-full-lyrics-unavailable')?.textContent).toStrictEqual(
    'This file has no lyrics.',
  );
});

test('open full player paints playing-from, remaining time, up next and pane placement', () => {
  const onToggleQueue = vi.fn();
  const { rerender } = render(
    <PlayerFull
      messages={shellMessages()}
      playback={{
        ...playingSnapshot(),
        queue: [
          {
            trackId: 'demo-track-01-01',
            albumId: 'demo-album-01',
            title: 'Pier at Dusk',
            artistName: 'Mira Sol',
            coverTone: '01',
            durationMs: 180_000,
            lyricsKind: 'none',
          },
          {
            trackId: 'demo-track-01-02',
            albumId: 'demo-album-01',
            title: 'Salt Window',
            artistName: 'Mira Sol',
            coverTone: '01',
            durationMs: 200_000,
            lyricsKind: 'none',
          },
        ],
      }}
      open
      albumTitle="Harbour Lights"
      onClose={vi.fn()}
      onToggleQueue={onToggleQueue}
    />,
  );
  expect(document.querySelector('#player-full')?.getAttribute('data-placement')).toStrictEqual(
    'overlay',
  );
  expect(document.querySelector('#player-full-scrim')).toBeTruthy();
  expect(document.querySelector('#player-full-from')?.textContent).toStrictEqual(
    'Playing from Harbour Lights',
  );
  expect(document.querySelector('#player-full-elapsed')?.textContent).toStrictEqual('0:45');
  expect(document.querySelector('#player-full-remaining')?.textContent).toStrictEqual('2:15');
  expect(document.querySelector('#player-full-up-next-title')?.textContent).toStrictEqual('Salt Window');
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  expect(onToggleQueue).toHaveBeenCalledTimes(1);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: 'Tab' });
  expect(onToggleQueue).toHaveBeenCalledTimes(3);
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      placement="pane"
      onClose={vi.fn()}
    />,
  );
  expect(document.querySelector('#player-full')?.getAttribute('data-placement')).toStrictEqual('pane');
  expect(document.querySelector('#player-full-scrim')).toBeNull();
  expect(document.querySelector('#player-full-from')).toBeNull();
  expect(document.querySelector('#player-full-up-next')).toBeNull();
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      placement="pane"
      albumTitle=""
      onClose={vi.fn()}
    />,
  );
  expect(document.querySelector('#player-full-from')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Enter' });
  expect(document.querySelector('#player-full-lyrics')).toBeNull();
});
