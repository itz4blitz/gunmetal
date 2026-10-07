import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { shellMessages } from '../messages/en/shell.ts';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';
import { emptySnapshot } from './test-playback.ts';
import { PlayerBar } from './PlayerBar.tsx';

afterEach(cleanup);

function playingSnapshot(): PlayerSnapshot {
  return {
    trackId: 'demo-track-01-01',
    albumId: 'demo-album-01',
    title: 'Pier at Dusk',
    artistName: 'Mira Sol',
    coverTone: '01',
    coverUrl: '/media/covers/fixture.svg',
    mediaUrl: '/media/audio/fixtures.wav',
    playing: true,
    positionMs: 45_000,
    durationMs: 180_000,
    lyricsKind: 'none',
    queue: [],
    queueOpen: false,
  };
}

test('empty player bar keeps Nothing is playing left-aligned in the now slot', () => {
  const onOpenFull = vi.fn();
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={emptySnapshot()} onOpenFull={onOpenFull} />,
  );
  const bar = container.querySelector('#player-bar');
  const empty = screen.getByText('Nothing is playing');
  expect(empty.id).toStrictEqual('player-empty');
  expect(empty.parentElement?.id).toStrictEqual('player-now');
  expect(empty.parentElement?.getAttribute('data-empty')).toStrictEqual('1');
  expect(bar?.querySelector('#player-art')).toBeNull();
  expect(bar?.querySelector('#player-meta')).toBeNull();
  expect(bar?.querySelector('#player-transport')).toBeTruthy();
  expect(bar?.querySelector('#player-progress')).toBeTruthy();
  // An empty bar never pretends to play: no fake times, no dashed slots.
  expect(bar?.querySelector('#player-time')).toBeNull();
  expect(screen.queryByText('0:00 / 0:00')).toBeNull();
  expect(bar?.querySelector('#player-device')).toBeNull();
  expect(bar?.querySelector('#player-actions')).toBeTruthy();
  expect(onOpenFull).toHaveBeenCalledTimes(0);
});

test('playing bar lays out art, meta, transport, scrubber and actions', () => {
  const onOpenFull = vi.fn();
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={playingSnapshot()} onOpenFull={onOpenFull} />,
  );
  const bar = container.querySelector('#player-bar');
  expect(bar).toBeTruthy();
  const zoneIds = [...(bar?.children ?? [])].map((child) => child.id);
  expect(zoneIds).toStrictEqual(['player-art', 'player-meta', 'player-transport', 'player-progress', 'player-actions']);
  expect(screen.getByText('Pier at Dusk').id).toStrictEqual('player-title');
  expect(screen.getByText('Mira Sol').id).toStrictEqual('player-artist');
  expect(container.querySelector('#player-album')).toBeNull();
  expect(screen.getByRole('button', { name: 'Pause' }).id).toStrictEqual('shell-play');
  expect(screen.getByRole('button', { name: 'Pause' }).getAttribute('data-player-control')).toStrictEqual('primary');
  expect(container.querySelector('#player-progress-fill')?.getAttribute('data-fill')).toStrictEqual('25');
  expect(container.querySelector('#player-expand')?.getAttribute('data-expand')).toStrictEqual('1');
  const openers = screen.getAllByRole('button', { name: 'Open full player' });
  expect(openers.map((node) => node.id)).toStrictEqual(['player-art', 'player-meta', 'player-expand']);
  fireEvent.click(openers[0]!);
  fireEvent.keyDown(openers[1]!, { key: 'Enter' });
  fireEvent.keyDown(openers[0]!, { key: ' ' });
  fireEvent.keyDown(openers[1]!, { key: ' ' });
  fireEvent.click(openers[2]!);
  fireEvent.keyDown(openers[2]!, { key: 'Enter' });
  fireEvent.keyDown(openers[2]!, { key: ' ' });
  fireEvent.keyDown(openers[0]!, { key: 'Tab' });
  fireEvent.keyDown(openers[1]!, { key: 'Tab' });
  fireEvent.keyDown(openers[2]!, { key: 'Tab' });
  expect(onOpenFull).toHaveBeenCalledTimes(7);
});

test('playing bar shows album, empty device slot and a lyrics control that opens the full player', () => {
  const onOpenFull = vi.fn();
  const { container, rerender } = render(
    <PlayerBar
      messages={shellMessages()}
      playback={playingSnapshot()}
      albumTitle="Harbour Lights"
      onOpenFull={onOpenFull}
    />,
  );
  expect(container.querySelector('#player-album')?.textContent).toStrictEqual('Harbour Lights');
  expect(container.querySelector('#player-device')?.getAttribute('data-device-slot')).toStrictEqual('empty');
  expect(screen.getByRole('button', { name: 'Lyrics' }).id).toStrictEqual('player-lyrics');
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Tab' });
  expect(onOpenFull).toHaveBeenCalledTimes(3);
  rerender(
    <PlayerBar
      messages={shellMessages()}
      playback={emptySnapshot()}
      albumTitle="Harbour Lights"
      onOpenFull={onOpenFull}
    />,
  );
  expect(container.querySelector('#player-album')).toBeNull();
  expect(container.querySelector('#player-lyrics')).toBeNull();
  expect(container.querySelector('#player-device')).toBeNull();
  expect(container.querySelector('#player-time')).toBeNull();
  rerender(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} albumTitle="" onOpenFull={onOpenFull} />);
  expect(container.querySelector('#player-album')).toBeNull();
  rerender(
    <PlayerBar
      messages={shellMessages()}
      playback={playingSnapshot()}
      albumTitle="Harbour Lights"
      compact
      onOpenFull={onOpenFull}
    />,
  );
  expect(container.querySelector('#player-album')?.textContent).toStrictEqual('Harbour Lights');
  expect(container.querySelector('#player-lyrics')).toBeNull();
  expect(container.querySelector('#player-device')).toBeNull();
});
