import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { shellMessages } from '../messages/en/shell.ts';
import { emptyPlayback, type PlaybackSnapshot } from './playback.ts';
import { PlayerBar } from './PlayerBar.tsx';

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

test('empty player bar keeps Nothing is playing left-aligned in the now slot', () => {
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={emptyPlayback()} />,
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
  expect(bar?.querySelector('#player-actions')).toBeTruthy();
});

test('playing bar lays out art, meta, transport, scrubber and actions', () => {
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={playingSnapshot()} />,
  );
  const bar = container.querySelector('#player-bar');
  expect(bar).toBeTruthy();
  const zoneIds = [...(bar?.children ?? [])].map((child) => child.id);
  expect(zoneIds).toStrictEqual([
    'player-art',
    'player-meta',
    'player-transport',
    'player-progress',
    'player-actions',
  ]);
  expect(screen.getByText('Pier at Dusk').id).toStrictEqual('player-title');
  expect(screen.getByText('Mira Sol').id).toStrictEqual('player-artist');
  expect(screen.getByRole('button', { name: 'Pause' }).id).toStrictEqual('shell-play');
  expect(screen.getByRole('button', { name: 'Pause' }).getAttribute('data-player-control')).toStrictEqual(
    'primary',
  );
  expect(container.querySelector('#player-progress-fill')?.getAttribute('data-fill')).toStrictEqual(
    '25',
  );
});
