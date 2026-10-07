import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { shellMessages } from '../messages/en/shell.ts';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';
import { emptySnapshot } from './test-playback.ts';
import { createPositionClock } from './position-clock.ts';
import { PlayerBar, defaultAudibleVolume, titleMarqueeShift, toggledVolume } from './PlayerBar.tsx';

afterEach(cleanup);

/** The fixture value a test names, or a loud failure — never an asserted maybe. */
function present<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) {
    throw new Error(`${what} is missing`);
  }
  return value;
}

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
  expect(container.querySelector('#player-art-empty')?.id).toStrictEqual('player-art-empty');
  // The centre stays mounted but sleeps: no layout shift on first play.
  expect(bar?.querySelector('#player-transport')?.id).toStrictEqual('player-transport');
  expect(bar?.querySelector('#player-progress')?.id).toStrictEqual('player-progress');
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
  expect(bar?.querySelector('#player-actions')?.id).toStrictEqual('player-actions');
  expect(screen.getByRole('button', { name: 'Queue' }).getAttribute('aria-label')).toStrictEqual('Queue');
});

test('playing bar lays out left, centre and right zones with a stacked centre', () => {
  const onOpenFull = vi.fn();
  const view = (playback: PlayerSnapshot) => (
    <PlayerBar messages={shellMessages()} playback={playback} onOpenFull={onOpenFull} />
  );
  const { container, rerender } = render(view(playingSnapshot()));
  const bar = container.querySelector('#player-bar');
  expect(bar?.getAttribute('data-bar-empty')).toStrictEqual('0');
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
  fireEvent.click(present(openers[0], 'full-player opener'));
  fireEvent.keyDown(present(openers[1], 'full-player opener'), { key: 'Enter' });
  fireEvent.keyDown(present(openers[2], 'full-player opener'), { key: 'Enter' });
  expect(onOpenFull).toHaveBeenCalledTimes(3);
  // Other keys leave the openers alone; both keys activate the art and the
  // credit line from the keyboard.
  fireEvent.keyDown(present(openers[0], 'full-player opener'), { key: 'Tab' });
  fireEvent.keyDown(present(openers[1], 'full-player opener'), { key: 'Tab' });
  fireEvent.keyDown(present(openers[2], 'full-player opener'), { key: 'Tab' });
  expect(onOpenFull).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(present(openers[0], 'full-player opener'), { key: 'Enter' });
  fireEvent.keyDown(present(openers[1], 'full-player opener'), { key: ' ' });
  expect(onOpenFull).toHaveBeenCalledTimes(5);
  // Paused playback marks the artwork honestly.
  expect(container.querySelector('#player-art')?.getAttribute('data-playing')).toStrictEqual('1');
  rerender(view({ ...playingSnapshot(), playing: false }));
  expect(container.querySelector('#player-art')?.getAttribute('data-playing')).toStrictEqual('0');
  rerender(view(playingSnapshot()));
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
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Tab' });
  expect(onOpenFull).toHaveBeenCalledTimes(3);
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
  expect(screen.getByRole('button', { name: 'Queue' }).getAttribute('aria-label')).toStrictEqual('Queue');
});

test('the scrubber seeks by click and by arrow keys', () => {
  const onSeek = vi.fn();
  const { container } = render(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} onSeek={onSeek} />);
  const scrubber = container.querySelector('#player-scrubber') as HTMLElement;
  // Nothing is laid out yet: a click seeks nowhere.
  fireEvent.click(scrubber, { clientX: 100 });
  expect(onSeek).toHaveBeenCalledTimes(0);
  Object.defineProperty(scrubber, 'getBoundingClientRect', {
    value: () => ({ left: 0, width: 200, top: 0, height: 20 }) as DOMRect,
  });
  fireEvent.click(scrubber, { clientX: 100 });
  expect(onSeek).toHaveBeenCalledWith(90_000); // 50% of 3:00
  fireEvent.keyDown(scrubber, { key: 'ArrowRight' });
  expect(onSeek).toHaveBeenLastCalledWith(50_000);
  fireEvent.keyDown(scrubber, { key: 'ArrowLeft' });
  expect(onSeek).toHaveBeenLastCalledWith(40_000);
  // A key that is not an arrow changes nothing.
  fireEvent.keyDown(scrubber, { key: 'Tab' });
  expect(onSeek).toHaveBeenCalledTimes(3);
  // Without a track length there is nothing to seek within.
  const idle = render(
    <PlayerBar messages={shellMessages()} playback={{ ...playingSnapshot(), durationMs: 0 }} onSeek={onSeek} />,
  );
  const dead = idle.container.querySelector('#player-scrubber') as HTMLElement;
  Object.defineProperty(dead, 'getBoundingClientRect', {
    value: () => ({ left: 0, width: 200, top: 0, height: 20 }) as DOMRect,
  });
  fireEvent.click(dead, { clientX: 100 });
  fireEvent.keyDown(dead, { key: 'ArrowRight' });
  expect(onSeek).toHaveBeenCalledTimes(3);
  // The empty bar's scrubber sleeps: out of the tab order and inert.
  const emptyView = render(<PlayerBar messages={shellMessages()} playback={emptySnapshot()} onSeek={onSeek} />);
  const sleeping = emptyView.container.querySelector('#player-scrubber') as HTMLElement;
  expect(sleeping.getAttribute('tabindex')).toStrictEqual('-1');
  fireEvent.click(sleeping, { clientX: 100 });
  fireEvent.keyDown(sleeping, { key: 'ArrowRight' });
  expect(onSeek).toHaveBeenCalledTimes(3);
  // Unwired, the same gestures are honest no-ops.
  const unwired = render(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} />);
  const inert = unwired.container.querySelector('#player-scrubber') as HTMLElement;
  Object.defineProperty(inert, 'getBoundingClientRect', {
    value: () => ({ left: 0, width: 200, top: 0, height: 20 }) as DOMRect,
  });
  fireEvent.click(inert, { clientX: 100 });
  fireEvent.keyDown(inert, { key: 'ArrowLeft' });
  expect(onSeek).toHaveBeenCalledTimes(3);
});

test('compact bar drops the extras and keeps transport inline', () => {
  const onOpenFull = vi.fn();
  const { container } = render(
    <PlayerBar messages={shellMessages()} playback={playingSnapshot()} compact onOpenFull={onOpenFull} />,
  );
  expect(container.querySelector('#player-lyrics')).toBeNull();
  expect(container.querySelector('#player-volume')).toBeNull();
  expect(container.querySelector('#player-device')).toBeNull();
  expect(screen.getByRole('button', { name: 'Queue' }).getAttribute('aria-label')).toStrictEqual('Queue');
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
  expect(iconOf('#player-shuffle')).toStrictEqual('shuffle');
  expect(iconOf('#player-repeat')).toStrictEqual('repeat');
  // Each control is named once, by its own label — the icon stays decorative.
  expect(
    [...container.querySelectorAll('#player-bar svg[data-icon]')].map((icon) => icon.getAttribute('aria-hidden')),
  ).toStrictEqual(['true', 'true', 'true', 'true', 'true', 'true', 'true', 'true', 'true']);
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

test('shuffle and repeat sit beside the transport and say their state', () => {
  const onToggleShuffle = vi.fn();
  const onCycleRepeat = vi.fn();
  const { container, rerender } = render(
    <PlayerBar
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), shuffleOn: true, repeatMode: 'all' }}
      onToggleShuffle={onToggleShuffle}
      onCycleRepeat={onCycleRepeat}
    />,
  );
  // The transport row walks shuffle, previous, the nut, next, repeat. The
  // nut sits in the square wrapper that carries its focus plate.
  expect(
    [...(container.querySelector('#player-transport')?.children ?? [])].map(
      (node) => node.id || node.firstElementChild?.id,
    ),
  ).toStrictEqual(['player-shuffle', 'player-prev', 'shell-play', 'player-next', 'player-repeat']);
  const shuffle = screen.getByRole('button', { name: 'Shuffle' });
  expect(shuffle.getAttribute('aria-pressed')).toStrictEqual('true');
  expect(shuffle.getAttribute('data-pressed')).toStrictEqual('1');
  expect(shuffle.getAttribute('tabindex')).toStrictEqual('0');
  const repeat = screen.getByRole('button', { name: 'Repeat all' });
  expect(repeat.getAttribute('aria-pressed')).toStrictEqual('true');
  expect(repeat.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('repeat');
  fireEvent.click(shuffle);
  fireEvent.keyDown(repeat, { key: 'Enter' });
  fireEvent.keyDown(repeat, { key: ' ' });
  fireEvent.keyDown(shuffle, { key: 'Tab' });
  expect(onToggleShuffle).toHaveBeenCalledTimes(1);
  expect(onCycleRepeat).toHaveBeenCalledTimes(2);
  // One announces the track itself comes round again; off says neither.
  rerender(
    <PlayerBar
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), repeatMode: 'one' }}
      onToggleShuffle={onToggleShuffle}
      onCycleRepeat={onCycleRepeat}
    />,
  );
  expect(screen.getByRole('button', { name: 'Repeat one' }).getAttribute('aria-pressed')).toStrictEqual('true');
  expect(container.querySelector('#player-repeat svg')?.getAttribute('data-icon')).toStrictEqual('repeat-one');
  rerender(
    <PlayerBar
      messages={shellMessages()}
      playback={playingSnapshot()}
      onToggleShuffle={onToggleShuffle}
      onCycleRepeat={onCycleRepeat}
    />,
  );
  expect(screen.getByRole('button', { name: 'Repeat' }).getAttribute('aria-pressed')).toStrictEqual('false');
  expect(container.querySelector('#player-repeat')?.getAttribute('data-pressed')).toStrictEqual('0');
  // Unwired, the controls are honest buttons that do nothing, not lies:
  // every transport key without its handler stays silent.
  const idle = render(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} />);
  for (const id of ['player-shuffle', 'player-prev', 'shell-play', 'player-next', 'player-repeat', 'player-queue']) {
    fireEvent.click(present(idle.container.querySelector(`#${id}`), id));
  }
  fireEvent.keyDown(present(idle.container.querySelector('#player-repeat'), 'idle repeat'), { key: 'Enter' });
  expect(onToggleShuffle).toHaveBeenCalledTimes(1);
  expect(onCycleRepeat).toHaveBeenCalledTimes(2);
  idle.unmount();
  idle.unmount();
  // Empty bar: the toggles sleep with the rest of the transport.
  const emptyBar = render(
    <PlayerBar messages={shellMessages()} playback={emptySnapshot()} onToggleShuffle={onToggleShuffle} />,
  );
  expect(emptyBar.container.querySelector('#player-shuffle')?.getAttribute('tabindex')).toStrictEqual('-1');
  fireEvent.click(emptyBar.container.querySelector('#player-shuffle') as HTMLElement);
  expect(onToggleShuffle).toHaveBeenCalledTimes(1);
});

test('the bar shows one honest engine state: its reason, or that it is buffering', () => {
  const { container, rerender } = render(
    <PlayerBar messages={shellMessages()} playback={{ ...playingSnapshot(), buffering: true }} />,
  );
  expect(container.querySelector('#player-bar')?.getAttribute('data-buffering')).toStrictEqual('1');
  expect(container.querySelector('#player-bar')?.getAttribute('data-errored')).toStrictEqual('0');
  const state = screen.getByRole('status');
  expect(state.id).toStrictEqual('player-state');
  expect(state.textContent).toStrictEqual('Buffering…');
  expect(state.getAttribute('data-player-state')).toStrictEqual('buffering');
  // A failure with a reason says the reason.
  rerender(
    <PlayerBar
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), buffering: false, playbackError: 'DEMUXER_ERROR_COULD_NOT_OPEN' }}
    />,
  );
  expect(container.querySelector('#player-bar')?.getAttribute('data-errored')).toStrictEqual('1');
  expect(screen.getByRole('status').textContent).toStrictEqual('DEMUXER_ERROR_COULD_NOT_OPEN');
  expect(screen.getByRole('status').getAttribute('data-player-state')).toStrictEqual('error');
  // A failure without a reason says what it can, still once.
  rerender(<PlayerBar messages={shellMessages()} playback={{ ...playingSnapshot(), playbackError: '' }} />);
  expect(screen.getByRole('status').textContent).toStrictEqual('This track could not be played.');
  // Healthy playback shows no state line at all.
  rerender(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} />);
  expect(screen.queryByRole('status')).toBeNull();
  expect(container.querySelector('#player-state')).toBeNull();
});

test('the marquee shift is measured slack, or nothing when the title fits', () => {
  expect(titleMarqueeShift(400, 176)).toStrictEqual(226);
  expect(titleMarqueeShift(178, 176)).toStrictEqual(null);
  expect(titleMarqueeShift(0, 0)).toStrictEqual(null);
});

test('the title marquee is measured on a real clip and forgotten when the title fits', () => {
  const { container, rerender } = render(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} />);
  const title = container.querySelector('#player-title') as HTMLElement;
  const clip = title.parentElement as HTMLElement;
  Object.defineProperty(clip, 'clientWidth', { configurable: true, value: 176 });
  Object.defineProperty(title, 'scrollWidth', { configurable: true, value: 400 });
  vi.stubGlobal('getComputedStyle', () => ({ paddingLeft: '10px', paddingRight: '10px' }));
  try {
    const setProperty = vi.spyOn(title.style, 'setProperty');
    const removeProperty = vi.spyOn(title.style, 'removeProperty');
    // A new title re-measures: 400px in a 156px clip shifts by the slack.
    rerender(
      <PlayerBar messages={shellMessages()} playback={{ ...playingSnapshot(), title: 'A Much Longer Title' }} />,
    );
    expect(setProperty).toHaveBeenCalledWith('--gm-title-shift', '246px');
    // A title that fits removes the shift instead.
    Object.defineProperty(title, 'scrollWidth', { configurable: true, value: 100 });
    rerender(
      <PlayerBar
        messages={shellMessages()}
        playback={{ ...playingSnapshot(), title: 'A Much Longer Title That Fits' }}
      />,
    );
    expect(removeProperty).toHaveBeenCalledWith('--gm-title-shift');
  } finally {
    vi.unstubAllGlobals();
  }
});

test('a bar rendered off the document measures nothing and does not crash', () => {
  const detached = document.createElement('div');
  const { container } = render(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} />, {
    container: detached,
  });
  expect(container.querySelector('#player-title')).not.toBeNull();
  expect(document.getElementById('player-title')).toBeNull();
});

test('with a position clock the readouts follow the frames, not the snapshot', () => {
  const scheduler = manualFrameScheduler();
  const clock = createPositionClock(scheduler);
  clock.sync({ durationMs: 180_000, positionMs: 45_000, playing: true });
  const { container } = render(<PlayerBar messages={shellMessages()} playback={playingSnapshot()} clock={clock} />);
  expect(container.querySelector('#player-time-elapsed')?.textContent).toStrictEqual('0:45');
  act(() => {
    scheduler.flush(16);
  });
  // The first frame only learns the baseline: the sync is the truth.
  expect(container.querySelector('#player-time-elapsed')?.textContent).toStrictEqual('0:45');
  act(() => {
    scheduler.flush(2_016); // 2000ms after the baseline frame
  });
  expect(container.querySelector('#player-time-elapsed')?.textContent).toStrictEqual('0:47');
  expect(container.querySelector('#player-scrubber')?.getAttribute('aria-valuenow')).toStrictEqual('47000');
  expect(container.querySelector('#player-progress-fill')?.getAttribute('data-fill')).toStrictEqual('26');
  clock.detach();
});

function manualFrameScheduler() {
  let callback: ((time: number) => void) | undefined;
  return {
    request(requested: (time: number) => void) {
      callback = requested;
      return 1;
    },
    cancel() {
      callback = undefined;
    },
    flush(time: number) {
      const run = callback;
      callback = undefined;
      run?.(time);
    },
  };
}

test('when the composition root owns the mute, the speaker button says the truth', () => {
  const changes: boolean[] = [];
  const levels: number[] = [];
  const view = (muted: boolean, volume: number) => (
    <PlayerBar
      messages={shellMessages()}
      playback={playingSnapshot()}
      volume={volume}
      onVolume={(next) => {
        levels.push(next);
      }}
      muted={muted}
      onMuted={(next) => {
        changes.push(next);
      }}
    />
  );
  const { rerender } = render(view(false, 0.4));
  const speaker = screen.getByRole('button', { name: 'Mute' });
  expect(speaker.getAttribute('aria-pressed')).toStrictEqual('false');
  fireEvent.click(speaker);
  expect(changes).toStrictEqual([true]);
  // The composition root applies it; the control reflects its state.
  rerender(view(true, 0.4));
  const silent = screen.getByRole('button', { name: 'Unmute' });
  expect(silent.getAttribute('aria-pressed')).toStrictEqual('true');
  expect(silent.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('mute');
  // Dragging the level while muted unmutes: the state stays honest.
  const range = screen.getByRole('slider', { name: 'Volume' }) as HTMLInputElement;
  fireEvent.change(range, { target: { value: '0.7' } });
  expect(levels).toStrictEqual([0.7]);
  expect(changes).toStrictEqual([true, false]);
  rerender(view(false, 0.7));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Mute' }), { key: 'Enter' });
  expect(changes).toStrictEqual([true, false, true]);
  rerender(view(true, 0.7));
  // The range at zero is not a mute of its own any more: the state is.
  rerender(view(true, 0));
  expect(screen.getByRole('button', { name: 'Unmute' }).getAttribute('aria-pressed')).toStrictEqual('true');
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
