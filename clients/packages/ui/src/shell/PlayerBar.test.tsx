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

test('empty bar keeps the idle state in the left zone and sleeps the centre', () => {
  const onOpenFull = vi.fn();
  const onPlayFirst = vi.fn();
  const { container } = render(
    <PlayerBar
      messages={shellMessages()}
      playback={emptySnapshot()}
      onOpenFull={onOpenFull}
      onPlayFirst={onPlayFirst}
    />,
  );
  const bar = container.querySelector('#player-bar');
  expect(bar?.getAttribute('data-bar-empty')).toStrictEqual('1');
  const empty = screen.getByText('Nothing is playing');
  expect(empty.id).toStrictEqual('player-empty');
  expect(empty.parentElement?.id).toStrictEqual('player-meta');
  // The idle art placeholder anchors the left edge.
  expect(container.querySelector('#player-art-empty')).toBeTruthy();
  // The centre stays mounted but sleeps: no layout shift on first play.
  expect(bar?.querySelector('#player-transport')).toBeTruthy();
  expect(bar?.querySelector('#player-progress')).toBeTruthy();
  expect(bar?.getAttribute('data-bar-empty')).toStrictEqual('1');
  // No fake times: the scrubber reads 0:00 until a track exists.
  expect(container.querySelector('#player-time-elapsed')?.textContent).toStrictEqual('0:00');
  // The idle play control is enabled and starts the featured album.
  const play = screen.getByRole('button', { name: 'Play' });
  expect(play.getAttribute('data-disabled')).toStrictEqual('0');
  fireEvent.click(play);
  expect(onPlayFirst).toHaveBeenCalledTimes(1);
  expect(onOpenFull).toHaveBeenCalledTimes(0);
  // Extras sleep when empty; the queue toggle persists.
  expect(container.querySelector('#player-volume')).toBeNull();
  expect(container.querySelector('#player-lyrics')).toBeNull();
  expect(container.querySelector('#player-expand')).toBeNull();
  expect(bar?.querySelector('#player-actions')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Queue' })).toBeTruthy();
});

test('playing bar lays out left, centre and right zones with a stacked centre', () => {
  const onOpenFull = vi.fn();
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={playingSnapshot()} onOpenFull={onOpenFull} />,
  );
  const bar = container.querySelector('#player-bar');
  expect(bar).toBeTruthy();
  expect(bar?.getAttribute('data-bar-empty')).toStrictEqual('0');
  const zoneIds = [...(bar?.children ?? [])].map((child) => child.id);
  expect(zoneIds).toStrictEqual(['player-left', 'player-center', 'player-actions']);
  const centre = container.querySelector('#player-center');
  expect([...(centre?.children ?? [])].map((child) => child.id)).toStrictEqual(['player-transport', 'player-progress']);
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
  fireEvent.keyDown(openers[2]!, { key: 'Enter' });
  expect(onOpenFull).toHaveBeenCalledTimes(3);
});

test('playing bar shows the merged credit line, volume and a lyrics control', () => {
  const onOpenFull = vi.fn();
  const onVolume = vi.fn();
  const { container } = render(
    <PlayerBar
      messages={shellMessages()}
      playback={playingSnapshot()}
      albumTitle="Harbour Lights"
      volume={0.4}
      onVolume={onVolume}
      onOpenFull={onOpenFull}
    />,
  );
  // Artist and album share one secondary line: artist · album.
  expect(container.querySelector('#player-artist')?.textContent).toStrictEqual('Mira Sol · Harbour Lights');
  expect(container.querySelector('#player-volume')?.getAttribute('data-volume')).toStrictEqual('1');
  const range = container.querySelector('#player-volume-range') as HTMLInputElement;
  expect(range.value).toStrictEqual('0.4');
  fireEvent.change(range, { target: { value: '0.9' } });
  expect(onVolume).toHaveBeenCalledWith(0.9);
  expect(screen.getByRole('button', { name: 'Lyrics' }).id).toStrictEqual('player-lyrics');
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  expect(onOpenFull).toHaveBeenCalledTimes(1);
});

test('empty bar hides extras but keeps the queue toggle mounted', () => {
  const onOpenFull = vi.fn();
  const { container } = render(
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
  expect(container.querySelector('#player-volume')).toBeNull();
  expect(container.querySelector('#player-time-elapsed')?.textContent).toStrictEqual('0:00');
  expect(screen.getByRole('button', { name: 'Queue' })).toBeTruthy();
});

test('the scrubber seeks by click and by arrow keys', () => {
  const onSeek = vi.fn();
  const { container } = render(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} onSeek={onSeek} />);
  const scrubber = container.querySelector('#player-scrubber') as HTMLElement;
  Object.defineProperty(scrubber, 'getBoundingClientRect', {
    value: () => ({ left: 0, width: 200, top: 0, height: 20 }) as DOMRect,
  });
  fireEvent.click(scrubber, { clientX: 100 });
  expect(onSeek).toHaveBeenCalledWith(90_000); // 50% of 3:00
  fireEvent.keyDown(scrubber, { key: 'ArrowRight' });
  expect(onSeek).toHaveBeenLastCalledWith(50_000);
  fireEvent.keyDown(scrubber, { key: 'ArrowLeft' });
  expect(onSeek).toHaveBeenLastCalledWith(40_000);
});

test('compact bar drops the extras and keeps transport inline', () => {
  const onOpenFull = vi.fn();
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={playingSnapshot()} compact onOpenFull={onOpenFull} />,
  );
  expect(container.querySelector('#player-lyrics')).toBeNull();
  expect(container.querySelector('#player-volume')).toBeNull();
  expect(container.querySelector('#player-device')).toBeNull();
  expect(screen.getByRole('button', { name: 'Queue' })).toBeTruthy();
});
