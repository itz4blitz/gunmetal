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
    queue: [],
    queueOpen: false,
  };
}

test('full player stays closed in the DOM until open and ignores empty playback', () => {
  const closed = render(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open={false}
      onClose={vi.fn()}
    />,
  );
  expect(document.querySelector('#player-full')?.getAttribute('data-open')).toStrictEqual('0');
  expect(document.querySelector('#player-full-scrim')?.getAttribute('data-open')).toStrictEqual(
    '0',
  );
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
  const { container } = render(
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
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  expect(onClose).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
  fireEvent.click(screen.getByRole('button', { name: 'Previous' }));
  fireEvent.click(screen.getByRole('button', { name: 'Next' }));
  expect(onPlayPause).toHaveBeenCalledTimes(1);
  expect(onPrevious).toHaveBeenCalledTimes(1);
  expect(onNext).toHaveBeenCalledTimes(1);
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
