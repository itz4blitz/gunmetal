import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { shellMessages } from '../messages/en/shell.ts';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';
import { emptySnapshot } from './test-playback.ts';
import { PlayerBar, defaultAudibleVolume, toggledVolume } from './PlayerBar.tsx';

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
  const onPlayPause = vi.fn();
  const { container } = render(
    <PlayerBar
      messages={shellMessages()}
      playback={emptySnapshot()}
      onOpenFull={onOpenFull}
      onPlayPause={onPlayPause}
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
  // Nothing is loaded, so there is nothing to play: the play control is a
  // disabled steel nut, out of the tab order, and pressing it does nothing.
  const play = screen.getByRole('button', { name: 'Play' });
  expect(play.id).toStrictEqual('shell-play');
  expect(play.getAttribute('data-disabled')).toStrictEqual('1');
  expect(play.getAttribute('data-idle')).toStrictEqual('1');
  expect(play.getAttribute('aria-disabled')).toStrictEqual('true');
  expect(play.getAttribute('tabindex')).toStrictEqual('-1');
  fireEvent.click(play);
  fireEvent.keyDown(play, { key: 'Enter' });
  fireEvent.keyDown(play, { key: ' ' });
  expect(onPlayPause).toHaveBeenCalledTimes(0);
  expect(onOpenFull).toHaveBeenCalledTimes(0);
  // The skips and the scrubber are out of reach too.
  expect(screen.getByRole('button', { name: 'Previous' }).getAttribute('data-disabled')).toStrictEqual('1');
  expect(screen.getByRole('button', { name: 'Next' }).getAttribute('data-disabled')).toStrictEqual('1');
  expect(container.querySelector('#player-scrubber')?.getAttribute('tabindex')).toStrictEqual('-1');
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

test('bar controls are drawn by the shared icon set, hidden from assistive tech', () => {
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={playingSnapshot()} volume={0.5} onVolume={vi.fn()} />,
  );
  const iconOf = (selector: string) => container.querySelector(`${selector} svg`)?.getAttribute('data-icon');
  expect(iconOf('#player-prev')).toStrictEqual('previous');
  expect(iconOf('#player-next')).toStrictEqual('next');
  expect(iconOf('#player-queue')).toStrictEqual('queue');
  expect(iconOf('#player-expand')).toStrictEqual('expand');
  expect(iconOf('#player-lyrics')).toStrictEqual('lyrics');
  expect(iconOf('#player-volume-icon')).toStrictEqual('volume');
  // Each control is named once, by its own label — the icon stays decorative.
  expect(
    [...container.querySelectorAll('#player-bar svg[data-icon]')].map((icon) => icon.getAttribute('aria-hidden')),
  ).toStrictEqual(['true', 'true', 'true', 'true', 'true', 'true', 'true']);
  // The artwork button carries a hidden expand hint that hover and focus reveal.
  const hint = container.querySelector('#player-art > [data-art-expand="1"]');
  expect(hint?.getAttribute('aria-hidden')).toStrictEqual('true');
  expect(hint?.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('expand');
  expect(screen.getAllByRole('button', { name: 'Open full player' }).map((node) => node.id)).toStrictEqual([
    'player-art',
    'player-meta',
    'player-expand',
  ]);
  expect(screen.getByRole('button', { name: 'Previous' }).id).toStrictEqual('player-prev');
  expect(screen.getByRole('button', { name: 'Next' }).id).toStrictEqual('player-next');
  // The nut draws its own glyph and keeps its text label for assistive tech.
  expect(container.querySelector('#shell-play svg')).toStrictEqual(null);
  expect(container.querySelector('#shell-play [data-control-label="1"]')?.textContent).toStrictEqual('Pause');
});

test('the mute rule silences an audible level and restores the remembered one', () => {
  expect(defaultAudibleVolume).toStrictEqual(0.8);
  expect(toggledVolume(0.4, 0.8)).toStrictEqual(0);
  expect(toggledVolume(1, 0.3)).toStrictEqual(0);
  expect(toggledVolume(0.01, 0.8)).toStrictEqual(0);
  expect(toggledVolume(0, 0.4)).toStrictEqual(0.4);
  expect(toggledVolume(0, 1)).toStrictEqual(1);
});

test('the speaker button mutes, then returns to the level it muted from', () => {
  const levels: number[] = [];
  const onVolume = (next: number) => {
    levels.push(next);
  };
  const bar = (volume: number) => (
    <PlayerBar messages={shellMessages()} playback={playingSnapshot()} volume={volume} onVolume={onVolume} />
  );
  const view = render(bar(0.4));
  const speaker = screen.getByRole('button', { name: 'Mute' });
  expect(speaker.id).toStrictEqual('player-volume-icon');
  expect(speaker.getAttribute('data-muted')).toStrictEqual('0');
  expect(speaker.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('volume');
  fireEvent.click(speaker);
  expect(levels).toStrictEqual([0]);

  // The composition root applies it; the button now offers the way back.
  view.rerender(bar(0));
  const silent = screen.getByRole('button', { name: 'Unmute' });
  expect(silent.id).toStrictEqual('player-volume-icon');
  expect(silent.getAttribute('data-muted')).toStrictEqual('1');
  expect(silent.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('mute');
  expect(screen.queryByRole('button', { name: 'Mute' })).toStrictEqual(null);
  fireEvent.click(silent);
  expect(levels).toStrictEqual([0, 0.4]);

  // Keyboard: Enter and Space activate, any other key does not.
  view.rerender(bar(0.65));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Mute' }), { key: 'Tab' });
  expect(levels).toStrictEqual([0, 0.4]);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Mute' }), { key: 'Enter' });
  expect(levels).toStrictEqual([0, 0.4, 0]);
  view.rerender(bar(0));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Unmute' }), { key: ' ' });
  expect(levels).toStrictEqual([0, 0.4, 0, 0.65]);
  // The handled keys are consumed (no page scroll on Space); others are not.
  expect(fireEvent.keyDown(screen.getByRole('button', { name: 'Unmute' }), { key: ' ' })).toStrictEqual(false);
  expect(fireEvent.keyDown(screen.getByRole('button', { name: 'Unmute' }), { key: 'Tab' })).toStrictEqual(true);
});

test('a bar that starts silent unmutes to the default level', () => {
  const levels: number[] = [];
  render(
    <PlayerBar
      messages={shellMessages()}
      playback={playingSnapshot()}
      volume={0}
      onVolume={(next) => {
        levels.push(next);
      }}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Unmute' }));
  expect(levels).toStrictEqual([0.8]);
});

test('with a track loaded the nut is live and the scrubber is a real slider', () => {
  const onPlayPause = vi.fn();
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={playingSnapshot()} onPlayPause={onPlayPause} />,
  );
  const nut = screen.getByRole('button', { name: 'Pause' });
  expect(nut.getAttribute('data-disabled')).toStrictEqual('0');
  expect(nut.getAttribute('data-idle')).toStrictEqual('0');
  expect(nut.getAttribute('tabindex')).toStrictEqual('0');
  fireEvent.click(nut);
  fireEvent.keyDown(nut, { key: 'Enter' });
  expect(onPlayPause).toHaveBeenCalledTimes(2);
  // 45s into a 3:00 track: the slider says so to assistive tech.
  const scrubber = container.querySelector('#player-scrubber');
  expect(scrubber?.getAttribute('role')).toStrictEqual('slider');
  expect(scrubber?.getAttribute('aria-label')).toStrictEqual('Progress');
  expect(scrubber?.getAttribute('aria-valuemin')).toStrictEqual('0');
  expect(scrubber?.getAttribute('aria-valuemax')).toStrictEqual('180000');
  expect(scrubber?.getAttribute('aria-valuenow')).toStrictEqual('45000');
  expect(scrubber?.getAttribute('aria-valuetext')).toStrictEqual('0:45 of 3:00');
  expect(scrubber?.getAttribute('tabindex')).toStrictEqual('0');
});
