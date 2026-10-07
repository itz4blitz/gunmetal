import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { shellMessages } from '../messages/en/shell.ts';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';
import { emptySnapshot } from './test-playback.ts';
import { createPositionClock } from './position-clock.ts';
import type { SyncedLine } from './synced-lyrics.ts';
import { PlayerFull, tabWrapTarget } from './PlayerFull.tsx';

afterEach(cleanup);

/** The fixture value a test names, or a loud failure — never an asserted maybe. */
function present<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) {
    throw new Error(`${what} is missing`);
  }
  return value;
}

const TIMED: readonly SyncedLine[] = [
  { atMs: 0, text: 'First line arrives at once' },
  { atMs: 30_000, text: 'the second at the half minute' },
];

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

test('full player stays unmounted until open and ignores empty playback', () => {
  const closed = render(
    <PlayerFull
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
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
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      messages={shellMessages()}
      playback={emptySnapshot()}
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
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
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
  expect(screen.getByRole('heading', { name: 'Now playing' }).id).toStrictEqual('player-full-heading');
  expect(screen.getByText('Pier at Dusk').id).toStrictEqual('player-full-title');
  expect(screen.getByText('Mira Sol').id).toStrictEqual('player-full-artist');
  expect(container.querySelector('#player-full-art [data-size="full"]')?.getAttribute('data-cover')).toStrictEqual(
    '01',
  );
  expect(container.querySelector('#player-full-progress')?.getAttribute('data-progress')).toStrictEqual('25');
  // One composed view: the artwork stage beside (or above) the console that
  // holds the title block, the scrubber, the transport and the secondary row.
  expect([...(container.querySelector('#player-full-body')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'player-full-chrome',
    'player-full-stage',
    'player-full-console',
  ]);
  expect(container.querySelector('#player-full-stage > #player-full-art [data-size="full"]')?.id).toStrictEqual(
    'cover-full-demo-track-01-01',
  );
  expect([...(container.querySelector('#player-full-console')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'player-full-info',
    'player-full-progress',
    'player-full-transport',
    'player-full-footer',
    'player-full-lyrics-unavailable',
  ]);
  expect([...(container.querySelector('#player-full-info')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'player-full-title',
    'player-full-artist',
  ]);
  // Controls are drawn by the shared icon set; the nut draws its own glyph,
  // keeps its text label and sits in the wrapper that carries its focus plate.
  const iconOf = (name: string) => screen.getByRole('button', { name }).querySelector('svg')?.getAttribute('data-icon');
  expect(iconOf('Close')).toStrictEqual('collapse');
  expect(iconOf('Previous')).toStrictEqual('previous');
  expect(iconOf('Next')).toStrictEqual('next');
  expect(iconOf('Lyrics')).toStrictEqual('lyrics');
  expect(iconOf('Queue')).toStrictEqual('queue');
  expect(iconOf('Shuffle')).toStrictEqual('shuffle');
  expect(iconOf('Repeat')).toStrictEqual('repeat');
  expect(
    [...container.querySelectorAll('#player-full svg[data-icon]')].map((icon) => icon.getAttribute('aria-hidden')),
  ).toStrictEqual(['true', 'true', 'true', 'true', 'true', 'true', 'true']);
  expect(screen.getByRole('button', { name: 'Pause' }).querySelector('svg')).toBeNull();
  expect(screen.getByRole('button', { name: 'Pause' }).textContent).toStrictEqual('Pause');
  expect(screen.getByRole('button', { name: 'Pause' }).getAttribute('data-playing')).toStrictEqual('1');
  expect(screen.getByRole('button', { name: 'Pause' }).parentElement?.getAttribute('data-hex-wrap')).toStrictEqual('1');
  expect(
    [...(container.querySelector('#player-full-transport')?.children ?? [])].map(
      (node) => node.id || node.firstElementChild?.id,
    ),
  ).toStrictEqual([
    'player-full-shuffle',
    'player-full-skip-back',
    'player-full-play',
    'player-full-skip-next',
    'player-full-repeat',
  ]);
  rerender(
    <PlayerFull
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), positionMs: 10, durationMs: 0 }}
      open
      onClose={onClose}
      onPlayPause={onPlayPause}
      onPrevious={onPrevious}
      onNext={onNext}
    />,
  );
  expect(container.querySelector('#player-full-progress')?.getAttribute('data-progress')).toStrictEqual('0');
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
  render(<PlayerFull messages={shellMessages()} playback={playingSnapshot()} open onClose={vi.fn()} />);
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
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
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
  fireEvent.click(document.querySelector('#player-full-scrim') as HTMLElement);
  expect(onClose).toHaveBeenCalledTimes(2);
});

test('full player lyrics toggle appears for plain and synced kinds and paints Text lines', () => {
  const onClose = vi.fn();
  const { rerender } = render(
    <PlayerFull
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), lyricsKind: 'plain', trackId: 'demo-track-01-03' }}
      open
      onClose={onClose}
    />,
  );
  expect(screen.getByRole('button', { name: 'Lyrics' }).getAttribute('aria-pressed')).toStrictEqual('false');
  expect(screen.getByRole('button', { name: 'Lyrics' }).hasAttribute('aria-disabled')).toStrictEqual(false);
  expect(document.querySelector('#player-full-stage')?.getAttribute('data-stage')).toStrictEqual('art');
  expect(document.querySelector('#player-full-lyrics-unavailable')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  expect(screen.getByRole('button', { name: 'Lyrics' }).getAttribute('aria-pressed')).toStrictEqual('true');
  expect(screen.getByRole('button', { name: 'Lyrics' }).getAttribute('data-lyrics-toggle')).toStrictEqual('1');
  // Open lyrics take the artwork's stage; the artwork stays mounted under them.
  expect(document.querySelector('#player-full-stage')?.getAttribute('data-stage')).toStrictEqual('lyrics');
  expect([...(document.querySelector('#player-full-stage')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'player-full-art',
    'player-full-lyrics',
  ]);
  expect(document.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual('0');
  expect(document.querySelector('#player-full-lyrics')?.getAttribute('data-empty')).toStrictEqual('0');
  expect(
    [...document.querySelectorAll('#player-full-lyrics [data-lyrics-line="1"]')].map((node) => node.textContent),
  ).toStrictEqual(['Hello, hello through the static', 'handshake in the noise', 'hold the line']);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Enter' });
  expect(document.querySelector('#player-full-lyrics')).toBeNull();
  expect(document.querySelector('#player-full-stage')?.getAttribute('data-stage')).toStrictEqual('art');
  expect(screen.getByRole('button', { name: 'Lyrics' }).getAttribute('data-lyrics-toggle')).toStrictEqual('0');
  rerender(
    <PlayerFull
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), lyricsKind: 'synced', trackId: 'demo-track-02-02' }}
      open
      onClose={onClose}
    />,
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: ' ' });
  expect(document.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual('1');
  expect(document.querySelector('#player-full-lyrics [data-current="1"]')?.textContent).toStrictEqual(
    'Hello, hello through the static',
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Tab' });
  rerender(<PlayerFull messages={shellMessages()} playback={playingSnapshot()} open onClose={onClose} />);
  expect(screen.getByRole('button', { name: 'Lyrics' }).getAttribute('data-lyrics-available')).toStrictEqual('0');
  expect(screen.getByRole('button', { name: 'Lyrics' }).getAttribute('aria-disabled')).toStrictEqual('true');
  expect(screen.getByRole('button', { name: 'Lyrics' }).getAttribute('aria-pressed')).toStrictEqual('false');
  expect(document.querySelector('#player-full-lyrics')).toBeNull();
  expect(document.querySelector('#player-full-stage')?.getAttribute('data-stage')).toStrictEqual('art');
  expect(document.querySelector('#player-full-lyrics-unavailable')?.textContent).toStrictEqual(
    'This file has no lyrics.',
  );
});

test('lyrics without a resolver fall back to the no-lyrics line in the stage', () => {
  render(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), lyricsKind: 'plain' }}
      open
      onClose={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  expect(
    [...document.querySelectorAll('#player-full-stage #player-full-lyrics [data-lyrics-line="1"]')].map(
      (node) => node.textContent,
    ),
  ).toStrictEqual(['This file has no lyrics.']);
  // The placeholder is the quiet empty state, not a one-line lyric sheet.
  expect(document.querySelector('#player-full-lyrics')?.getAttribute('data-empty')).toStrictEqual('1');
  expect(document.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual('0');
});

test('volume sits in the secondary row only when it is wired, and reports the new level', () => {
  const onVolume = vi.fn();
  const { container, rerender } = render(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      volume={0.4}
      onVolume={onVolume}
      onClose={vi.fn()}
    />,
  );
  expect([...(container.querySelector('#player-full-footer')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'player-full-volume',
    'player-full-footer-actions',
  ]);
  expect(container.querySelector('#player-full-volume')?.getAttribute('data-volume')).toStrictEqual('1');
  expect(container.querySelector('#player-full-volume-icon svg')?.getAttribute('data-icon')).toStrictEqual('volume');
  const range = screen.getByRole('slider', { name: 'Volume' });
  expect(range.id).toStrictEqual('player-full-volume-range');
  expect(range.getAttribute('value')).toStrictEqual('0.4');
  fireEvent.change(range, { target: { value: '0.9' } });
  expect(onVolume).toHaveBeenCalledTimes(1);
  expect(onVolume).toHaveBeenCalledWith(0.9);
  rerender(<PlayerFull messages={shellMessages()} playback={playingSnapshot()} open volume={0.4} onClose={vi.fn()} />);
  expect(container.querySelector('#player-full-volume')).toBeNull();
  rerender(
    <PlayerFull messages={shellMessages()} playback={playingSnapshot()} open onVolume={onVolume} onClose={vi.fn()} />,
  );
  expect(container.querySelector('#player-full-volume')).toBeNull();
  expect([...(container.querySelector('#player-full-footer')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'player-full-footer-actions',
  ]);
});

test('open full player paints playing-from, remaining time, up next and pane placement', () => {
  const onToggleQueue = vi.fn();
  const { rerender } = render(
    <PlayerFull
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
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
            coverUrl: '/media/covers/fixture.svg',
            mediaUrl: '/media/audio/fixtures.wav',
            durationMs: 180_000,
            lyricsKind: 'none',
          },
          {
            trackId: 'demo-track-01-02',
            albumId: 'demo-album-01',
            title: 'Salt Window',
            artistName: 'Mira Sol',
            coverTone: '01',
            coverUrl: '/media/covers/fixture.svg',
            mediaUrl: '/media/audio/fixtures.wav',
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
  expect(document.querySelector('#player-full')?.getAttribute('data-placement')).toStrictEqual('overlay');
  expect(document.querySelector('#player-full-scrim')?.getAttribute('data-open')).toStrictEqual('1');
  expect(document.querySelector('#player-full-from')?.textContent).toStrictEqual('Playing from Harbour Lights');
  expect(document.querySelector('#player-full-elapsed')?.textContent).toStrictEqual('0:45');
  expect(document.querySelector('#player-full-remaining')?.textContent).toStrictEqual('2:15');
  expect(document.querySelector('#player-full-up-next-title')?.textContent).toStrictEqual('Salt Window');
  expect(document.querySelector('#player-full-up-next-label')?.textContent).toStrictEqual('Up next');
  expect(document.querySelector('#player-full-up-next-artist')?.textContent).toStrictEqual('Mira Sol');
  expect(document.querySelector('#player-full-up-next-duration')?.textContent).toStrictEqual('3:20');
  expect(document.querySelector('#player-full-up-next [data-size="row"]')?.id).toStrictEqual(
    'cover-next-demo-track-01-02',
  );
  expect(
    document.querySelector('#player-full-up-next [data-size="row"]')?.getAttribute('data-cover-art'),
  ).toStrictEqual('1');
  expect([...(document.querySelector('#player-full-info')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'player-full-from',
    'player-full-title',
    'player-full-artist',
  ]);
  expect(document.querySelector('#player-full-console')?.lastElementChild?.id).toStrictEqual('player-full-up-next');
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  expect(onToggleQueue).toHaveBeenCalledTimes(1);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: 'Tab' });
  expect(onToggleQueue).toHaveBeenCalledTimes(3);
  rerender(
    <PlayerFull
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
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
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
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

test('open full player paints the ambient artwork backdrop under a contrast veil', () => {
  const { container, rerender } = render(
    <PlayerFull messages={shellMessages()} playback={playingSnapshot()} open onClose={vi.fn()} />,
  );
  const ambient = container.querySelector('#player-full-ambient');
  expect(ambient?.id).toStrictEqual('player-full-ambient');
  const art = ambient?.querySelector('img[data-ambient-art="1"]');
  expect(art?.getAttribute('src')).toStrictEqual('/media/covers/fixture.svg');
  expect(art?.getAttribute('alt')).toStrictEqual('');
  expect(art?.getAttribute('aria-hidden')).toStrictEqual('true');
  expect(ambient?.querySelector('[data-ambient-veil="1"]')?.getAttribute('data-ambient-veil')).toStrictEqual('1');
  rerender(
    <PlayerFull messages={shellMessages()} playback={{ ...playingSnapshot(), coverUrl: '' }} open onClose={vi.fn()} />,
  );
  expect(document.querySelector('#player-full-ambient img')).toBeNull();
  expect(
    document.querySelector('#player-full-ambient [data-ambient-veil="1"]')?.getAttribute('data-ambient-veil'),
  ).toStrictEqual('1');
});

test('the queue control brings up a sheet left open underneath before it toggles one', () => {
  const onToggleQueue = vi.fn();
  const view = (open: boolean, queueOpen: boolean) => (
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), queueOpen }}
      open={open}
      onClose={vi.fn()}
      onToggleQueue={onToggleQueue}
    />
  );
  const sheet = () => document.querySelector('#player-full')?.getAttribute('data-queue-sheet');
  // Play opened the queue on its own: it stays under the full player.
  const { rerender } = render(view(true, true));
  expect(sheet()).toStrictEqual('under');
  // Asking for the queue brings that sheet up; nothing is toggled shut.
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  expect(onToggleQueue).toHaveBeenCalledTimes(0);
  expect(sheet()).toStrictEqual('over');
  // Asking again closes the sheet that is up.
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  expect(onToggleQueue).toHaveBeenCalledTimes(1);
  expect(sheet()).toStrictEqual('over');
  rerender(view(true, false));
  expect(sheet()).toStrictEqual('under');
  // A queue that opens again on its own is underneath again.
  rerender(view(true, true));
  expect(sheet()).toStrictEqual('under');
  // A closed queue is opened by the control, over the player.
  rerender(view(true, false));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: 'Enter' });
  expect(onToggleQueue).toHaveBeenCalledTimes(2);
  rerender(view(true, true));
  expect(sheet()).toStrictEqual('over');
  // Putting the player away forgets the request.
  rerender(view(false, true));
  expect(document.querySelector('#player-full')).toBeNull();
  rerender(view(true, true));
  expect(sheet()).toStrictEqual('under');
});

test('the scrubber seeks by pointer and by arrow keys when it is wired', () => {
  const onSeek = vi.fn();
  const view = (playback: PlayerSnapshot, seek: ((positionMs: number) => void) | undefined) => (
    <PlayerFull messages={shellMessages()} playback={playback} open onSeek={seek} onClose={vi.fn()} />
  );
  const { rerender } = render(view(playingSnapshot(), onSeek));
  const scrubber = screen.getByRole('slider', { name: 'Progress' });
  expect(scrubber.id).toStrictEqual('player-full-scrubber');
  expect(scrubber.parentElement?.id).toStrictEqual('player-full-progress');
  expect([...scrubber.children].map((node) => node.id)).toStrictEqual(['player-full-progress-track']);
  expect(scrubber.getAttribute('aria-valuemin')).toStrictEqual('0');
  expect(scrubber.getAttribute('aria-valuemax')).toStrictEqual('180000');
  expect(scrubber.getAttribute('aria-valuenow')).toStrictEqual('45000');
  expect(scrubber.getAttribute('aria-valuetext')).toStrictEqual('0:45 of 3:00');
  expect(scrubber.getAttribute('tabindex')).toStrictEqual('0');
  expect(scrubber.getAttribute('data-seekable')).toStrictEqual('1');
  // Nothing is laid out yet (a zero-width track): a click seeks nowhere.
  fireEvent.click(scrubber, { clientX: 150 });
  expect(onSeek).toHaveBeenCalledTimes(0);
  Object.defineProperty(scrubber, 'getBoundingClientRect', {
    value: () => ({ left: 100, width: 200, top: 0, height: 20 }) as DOMRect,
  });
  fireEvent.click(scrubber, { clientX: 150 });
  expect(onSeek).toHaveBeenLastCalledWith(45_000); // a quarter of 3:00
  fireEvent.click(scrubber, { clientX: 250 });
  expect(onSeek).toHaveBeenLastCalledWith(135_000);
  // Outside the track clamps to its ends.
  fireEvent.click(scrubber, { clientX: 20 });
  expect(onSeek).toHaveBeenLastCalledWith(0);
  fireEvent.click(scrubber, { clientX: 900 });
  expect(onSeek).toHaveBeenLastCalledWith(180_000);
  expect(onSeek).toHaveBeenCalledTimes(4);
  // Arrow keys step five seconds from where playback is.
  fireEvent.keyDown(scrubber, { key: 'ArrowRight' });
  expect(onSeek).toHaveBeenLastCalledWith(50_000);
  fireEvent.keyDown(scrubber, { key: 'ArrowLeft' });
  expect(onSeek).toHaveBeenLastCalledWith(40_000);
  fireEvent.keyDown(scrubber, { key: 'Tab' });
  fireEvent.keyDown(scrubber, { key: 'Enter' });
  expect(onSeek).toHaveBeenCalledTimes(6);
  // The audio clock reports fractions of a millisecond; the slider's value stays whole.
  rerender(view({ ...playingSnapshot(), positionMs: 111_394.005 }, onSeek));
  expect(screen.getByRole('slider', { name: 'Progress' }).getAttribute('aria-valuenow')).toStrictEqual('111394');
  expect(screen.getByRole('slider', { name: 'Progress' }).getAttribute('aria-valuetext')).toStrictEqual('1:51 of 3:00');
  rerender(view({ ...playingSnapshot(), positionMs: 2_000 }, onSeek));
  fireEvent.keyDown(screen.getByRole('slider', { name: 'Progress' }), { key: 'ArrowLeft' });
  expect(onSeek).toHaveBeenLastCalledWith(0);
  rerender(view({ ...playingSnapshot(), positionMs: 178_000 }, onSeek));
  fireEvent.keyDown(screen.getByRole('slider', { name: 'Progress' }), { key: 'ArrowRight' });
  expect(onSeek).toHaveBeenLastCalledWith(180_000);
  expect(onSeek).toHaveBeenCalledTimes(8);

  // A track with no known length cannot be scrubbed: inert, out of the tab order.
  rerender(view({ ...playingSnapshot(), positionMs: 10, durationMs: 0 }, onSeek));
  const still = screen.getByRole('slider', { name: 'Progress' });
  expect(still.getAttribute('tabindex')).toStrictEqual('-1');
  expect(still.getAttribute('data-seekable')).toStrictEqual('0');
  expect(still.getAttribute('aria-valuemax')).toStrictEqual('0');
  expect(still.getAttribute('aria-valuetext')).toStrictEqual('0:00 of 0:00');
  fireEvent.click(still, { clientX: 150 });
  fireEvent.keyDown(still, { key: 'ArrowRight' });
  expect(onSeek).toHaveBeenCalledTimes(8);

  // Unwired: the same bar, showing progress only.
  rerender(view(playingSnapshot(), undefined));
  const shown = screen.getByRole('slider', { name: 'Progress' });
  expect(shown.getAttribute('tabindex')).toStrictEqual('-1');
  expect(shown.getAttribute('data-seekable')).toStrictEqual('0');
  expect(shown.getAttribute('aria-valuenow')).toStrictEqual('45000');
  fireEvent.click(shown, { clientX: 150 });
  fireEvent.keyDown(shown, { key: 'ArrowLeft' });
  expect(onSeek).toHaveBeenCalledTimes(8);
});

test('closing folds the view away over the sheet motion before it unmounts, where motion is allowed', () => {
  vi.useFakeTimers();
  const matchMedia = vi.fn((query: string) => ({ matches: false, media: query }));
  vi.stubGlobal('matchMedia', matchMedia);
  try {
    const view = (open: boolean) => (
      <PlayerFull messages={shellMessages()} playback={playingSnapshot()} open={open} onClose={vi.fn()} />
    );
    const state = () => [
      document.querySelector('#player-full')?.getAttribute('data-open'),
      document.querySelector('#player-full-scrim')?.getAttribute('data-open'),
    ];
    const { rerender } = render(view(true));
    expect(state()).toStrictEqual(['1', '1']);
    expect(matchMedia).toHaveBeenCalledTimes(0);
    // Closed: still mounted and marked as leaving for the 200ms sheet motion.
    rerender(view(false));
    expect(state()).toStrictEqual(['0', '0']);
    expect(matchMedia).toHaveBeenLastCalledWith('(prefers-reduced-motion: reduce)');
    act(() => {
      vi.advanceTimersByTime(199);
    });
    expect(state()).toStrictEqual(['0', '0']);
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(state()).toStrictEqual([undefined, undefined]);
    // Opening again while it leaves keeps it: the pending unmount is dropped.
    rerender(view(true));
    rerender(view(false));
    act(() => {
      vi.advanceTimersByTime(120);
    });
    rerender(view(true));
    expect(state()).toStrictEqual(['1', '1']);
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(state()).toStrictEqual(['1', '1']);
    // Reduced motion: no leave at all, the view is gone with the request.
    matchMedia.mockImplementation((query: string) => ({ matches: true, media: query }));
    rerender(view(false));
    expect(state()).toStrictEqual([undefined, undefined]);
  } finally {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  }
});

test('a paused track offers Play on the nut and reports the press', () => {
  const onPlayPause = vi.fn();
  render(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), playing: false }}
      open
      onClose={vi.fn()}
      onPlayPause={onPlayPause}
    />,
  );
  const play = screen.getByRole('button', { name: 'Play' });
  expect(play.id).toStrictEqual('player-full-play');
  expect(play.getAttribute('data-playing')).toStrictEqual('0');
  expect(play.getAttribute('data-player-control')).toStrictEqual('primary');
  expect(play.textContent).toStrictEqual('Play');
  expect(screen.queryByRole('button', { name: 'Pause' })).toBeNull();
  fireEvent.keyDown(play, { key: ' ' });
  expect(onPlayPause).toHaveBeenCalledTimes(1);
});

test('the dialog is modal: focus enters on open, is trapped, and returns to the opener', () => {
  const onClose = vi.fn();
  const view = (open: boolean) => (
    <>
      <button id="dialog-opener" type="button">
        Open the player
      </button>
      <PlayerFull
        messages={shellMessages()}
        playback={playingSnapshot()}
        open={open}
        onClose={onClose}
        onToggleQueue={vi.fn()}
      />
    </>
  );
  const { rerender, unmount } = render(view(false));
  const opener = document.querySelector('#dialog-opener') as HTMLElement;
  opener.focus();
  expect(document.activeElement?.id).toStrictEqual('dialog-opener');
  rerender(view(true));
  // The dialog is announced modal and takes the focus at its first control.
  expect(document.querySelector('#player-full')?.getAttribute('aria-modal')).toStrictEqual('true');
  expect(document.activeElement?.id).toStrictEqual('player-full-collapse');
  const controls = [
    ...(document.querySelector('#player-full') as HTMLElement).querySelectorAll<HTMLElement>('[tabindex="0"], input'),
  ].filter((node) => node.getAttribute('aria-disabled') !== 'true');
  expect(controls.length).toBeGreaterThanOrEqual(6);
  // Tab from the last control wraps to the first; Shift+Tab from the first
  // wraps to the last; from a control in the middle the order is natural.
  present(controls[controls.length - 1], 'last full-player control').focus();
  fireEvent.keyDown(document.querySelector('#player-full') as HTMLElement, { key: 'Tab' });
  expect(document.activeElement?.id).toStrictEqual('player-full-collapse');
  fireEvent.keyDown(document.querySelector('#player-full') as HTMLElement, { key: 'Tab', shiftKey: true });
  expect(document.activeElement?.id).toStrictEqual(
    present(controls[controls.length - 1], 'last full-player control').id,
  );
  const middle = controls[Math.floor(controls.length / 2)] as HTMLElement;
  middle.focus();
  fireEvent.keyDown(document.querySelector('#player-full') as HTMLElement, { key: 'Tab' });
  expect(document.activeElement?.id).toStrictEqual(middle.id);
  // Closing hands the focus back to where it came from.
  rerender(view(false));
  expect(document.activeElement?.id).toStrictEqual('dialog-opener');
  rerender(view(true));
  rerender(view(false));
  expect(document.activeElement?.id).toStrictEqual('dialog-opener');
  unmount();
});

test('the tab wrap is decided by the dialog node list alone', () => {
  const first = document.createElement('div');
  const middle = document.createElement('div');
  const last = document.createElement('div');
  const nodes = [first, middle, last];
  expect(tabWrapTarget([], document.body, false)).toStrictEqual(undefined);
  expect(tabWrapTarget(nodes, last, false)).toStrictEqual(first);
  expect(tabWrapTarget(nodes, first, true)).toStrictEqual(last);
  expect(tabWrapTarget(nodes, middle, false)).toStrictEqual(undefined);
  expect(tabWrapTarget(nodes, middle, true)).toStrictEqual(undefined);
  // A focus that escaped the dialog comes back to the natural ends.
  expect(tabWrapTarget(nodes, document.body, false)).toStrictEqual(first);
  expect(tabWrapTarget(nodes, document.body, true)).toStrictEqual(last);
});

test('shuffle and repeat ride in the full transport with their states', () => {
  const onToggleShuffle = vi.fn();
  const onCycleRepeat = vi.fn();
  const { container, rerender } = render(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), shuffleOn: true, repeatMode: 'all' }}
      open
      onClose={vi.fn()}
      onToggleShuffle={onToggleShuffle}
      onCycleRepeat={onCycleRepeat}
    />,
  );
  const shuffle = screen.getByRole('button', { name: 'Shuffle' });
  expect(shuffle.id).toStrictEqual('player-full-shuffle');
  expect(shuffle.getAttribute('aria-pressed')).toStrictEqual('true');
  const repeat = screen.getByRole('button', { name: 'Repeat all' });
  expect(repeat.id).toStrictEqual('player-full-repeat');
  expect(repeat.getAttribute('aria-pressed')).toStrictEqual('true');
  expect(container.querySelector('#player-full-repeat svg')?.getAttribute('data-icon')).toStrictEqual('repeat');
  fireEvent.click(shuffle);
  fireEvent.keyDown(repeat, { key: 'Enter' });
  expect(onToggleShuffle).toHaveBeenCalledTimes(1);
  expect(onCycleRepeat).toHaveBeenCalledTimes(1);
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), repeatMode: 'one' }}
      open
      onClose={vi.fn()}
      onToggleShuffle={onToggleShuffle}
      onCycleRepeat={onCycleRepeat}
    />,
  );
  expect(screen.getByRole('button', { name: 'Repeat one' }).getAttribute('aria-pressed')).toStrictEqual('true');
  expect(container.querySelector('#player-full-repeat svg')?.getAttribute('data-icon')).toStrictEqual('repeat-one');
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      onClose={vi.fn()}
      onToggleShuffle={onToggleShuffle}
      onCycleRepeat={onCycleRepeat}
    />,
  );
  expect(screen.getByRole('button', { name: 'Repeat' }).getAttribute('aria-pressed')).toStrictEqual('false');
  // Unwired, the toggles stay inert buttons.
  const idle = render(<PlayerFull messages={shellMessages()} playback={playingSnapshot()} open onClose={vi.fn()} />);
  fireEvent.click(idle.container.querySelector('#player-full-shuffle') as HTMLElement);
  fireEvent.keyDown(idle.container.querySelector('#player-full-repeat') as HTMLElement, { key: ' ' });
  expect(onToggleShuffle).toHaveBeenCalledTimes(1);
  expect(onCycleRepeat).toHaveBeenCalledTimes(1);
});

test('the console says the engine state once: its reason, or that it is buffering', () => {
  const { container, rerender } = render(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), buffering: true }}
      open
      onClose={vi.fn()}
    />,
  );
  const state = screen.getByRole('status');
  expect(state.id).toStrictEqual('player-full-state');
  expect(state.textContent).toStrictEqual('Buffering…');
  expect(state.getAttribute('data-player-state')).toStrictEqual('buffering');
  expect(container.querySelector('#player-full')?.getAttribute('data-buffering')).toStrictEqual('1');
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), playbackError: 'The file could not be decoded.' }}
      open
      onClose={vi.fn()}
    />,
  );
  expect(screen.getByRole('status').textContent).toStrictEqual('The file could not be decoded.');
  expect(container.querySelector('#player-full')?.getAttribute('data-errored')).toStrictEqual('1');
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), playbackError: '' }}
      open
      onClose={vi.fn()}
    />,
  );
  expect(screen.getByRole('status').textContent).toStrictEqual('This track could not be played.');
  rerender(<PlayerFull messages={shellMessages()} playback={playingSnapshot()} open onClose={vi.fn()} />);
  expect(screen.queryByRole('status')).toBeNull();
});

test('with a position clock the times and scrubber follow the frames', () => {
  let callback: ((time: number) => void) | undefined;
  const scheduler = {
    request(requested: (time: number) => void) {
      callback = requested;
      return 1;
    },
    cancel() {
      callback = undefined;
    },
  };
  const clock = createPositionClock(scheduler);
  clock.sync({ durationMs: 180_000, positionMs: 45_000, playing: true });
  const { container, unmount } = render(
    <PlayerFull messages={shellMessages()} playback={playingSnapshot()} open clock={clock} onClose={vi.fn()} />,
  );
  expect(container.querySelector('#player-full-elapsed')?.textContent).toStrictEqual('0:45');
  act(() => {
    callback?.(16);
  });
  act(() => {
    callback?.(2_016);
  });
  expect(container.querySelector('#player-full-elapsed')?.textContent).toStrictEqual('0:47');
  expect(container.querySelector('#player-full-remaining')?.textContent).toStrictEqual('2:13');
  expect(container.querySelector('#player-full-scrubber')?.getAttribute('aria-valuenow')).toStrictEqual('47000');
  clock.detach();
  unmount();
});

test('when the composition root owns the mute, the full player says so and can change it', () => {
  const changes: boolean[] = [];
  const view = (muted: boolean, volume: number) => (
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      volume={volume}
      onVolume={vi.fn()}
      muted={muted}
      onMuted={(next) => {
        changes.push(next);
      }}
      onClose={vi.fn()}
    />
  );
  const first = render(view(false, 0.4));
  const speaker = screen.getByRole('button', { name: 'Mute' });
  expect(speaker.id).toStrictEqual('player-full-volume-icon');
  expect(speaker.getAttribute('aria-pressed')).toStrictEqual('false');
  fireEvent.click(speaker);
  expect(changes).toStrictEqual([true]);
  first.rerender(view(true, 0.4));
  const silent = screen.getByRole('button', { name: 'Unmute' });
  expect(silent.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('mute');
  fireEvent.keyDown(silent, { key: 'Enter' });
  expect(changes).toStrictEqual([true, false]);
  // jsdom resolves id selectors against the whole document, so the other
  // variants render only once this one is gone.
  first.rerender(view(false, 0.4));
  first.unmount();
  // Without an owned mute the icon stays decorative, as before — and it
  // still says a mute it was merely told about.
  const decorative = render(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      volume={0.4}
      onVolume={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  expect(decorative.container.querySelector('#player-full-volume-icon')?.getAttribute('tabindex')).toStrictEqual('-1');
  expect(screen.queryByRole('button', { name: 'Mute' })).toBeNull();
  decorative.unmount();
  const told = render(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      volume={0.4}
      onVolume={vi.fn()}
      muted
      onClose={vi.fn()}
    />,
  );
  expect(told.container.querySelector('#player-full-volume-icon svg')?.getAttribute('data-icon')).toStrictEqual('mute');
  told.unmount();
  // An owned mute that has not been asked yet reads as unmuted, and its
  // first press asks for the mute.
  const fresh = render(
    <PlayerFull
      messages={shellMessages()}
      playback={playingSnapshot()}
      open
      volume={0.4}
      onVolume={vi.fn()}
      onMuted={(next) => {
        changes.push(next);
      }}
      onClose={vi.fn()}
    />,
  );
  expect(fresh.container.querySelector('#player-full-volume-icon')?.getAttribute('aria-pressed')).toStrictEqual(
    'false',
  );
  fireEvent.click(fresh.container.querySelector('#player-full-volume-icon') as HTMLElement);
  fireEvent.keyDown(fresh.container.querySelector('#player-full-volume-icon') as HTMLElement, { key: 'Enter' });
  expect(changes).toStrictEqual([true, false, true, true]);
  fresh.unmount();
});

test('a timed resolver lights the sounding line and the pane follows the position', () => {
  const timedLyricsFor = vi.fn((trackId: string) => (trackId === 'demo-track-01-01' ? TIMED : undefined));
  const view = (positionMs: number) => (
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), lyricsKind: 'synced' }}
      open
      timedLyricsFor={timedLyricsFor}
      positionMs={positionMs}
      onClose={vi.fn()}
    />
  );
  const { container, rerender } = render(view(0));
  expect(timedLyricsFor).toHaveBeenCalledWith('demo-track-01-01', 'synced');
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  const currents = () =>
    [...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.getAttribute('data-current'));
  expect(container.querySelector('#player-full-lyrics')?.getAttribute('data-synced')).toStrictEqual('1');
  expect([...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent)).toStrictEqual([
    'First line arrives at once',
    'the second at the half minute',
  ]);
  expect(currents()).toStrictEqual(['1', '0']);
  rerender(view(30_000));
  expect(currents()).toStrictEqual(['0', '1']);
  // A track the timed resolver has nothing for falls back to the plain one;
  // the sheet stays open across the track change.
  rerender(
    <PlayerFull
      messages={shellMessages()}
      playback={{ ...playingSnapshot(), lyricsKind: 'synced', trackId: 'demo-track-01-03' }}
      open
      timedLyricsFor={timedLyricsFor}
      onClose={vi.fn()}
    />,
  );
  expect([...container.querySelectorAll('[data-lyrics-line="1"]')].map((node) => node.textContent)).toStrictEqual([
    'This file has no lyrics.',
  ]);
  expect(container.querySelector('#player-full-lyrics')?.getAttribute('data-empty')).toStrictEqual('1');
});
