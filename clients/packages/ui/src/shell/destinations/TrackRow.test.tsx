import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { hostileCorpus } from '../../../../fake-server/src/hostile.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellTrack } from '../library-types.ts';
import { TrackRow } from './TrackRow.tsx';

afterEach(cleanup);

const track: ShellTrack = {
  id: 'demo-track-01-01',
  albumId: 'demo-album-01',
  discIndex: 1,
  number: 1,
  title: 'Pier at Dusk',
  artistName: 'Mira Sol',
  durationMs: 214_000,
  flag: 'ok',
  lyricsKind: 'none',
  mediaUrl: '/media/audio/fixtures.wav',
};

test('rows are keyboard playable: Enter and Space play, other keys do nothing', () => {
  const onPlay = vi.fn();
  render(<TrackRow track={track} messages={destinationMessages()} onPlay={onPlay} />);
  const play = screen.getByRole('button', { name: 'Pier at Dusk' });
  fireEvent.keyDown(play, { key: 'Enter' });
  fireEvent.keyDown(play, { key: ' ' });
  expect(onPlay).toHaveBeenCalledTimes(2);
  expect(onPlay).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  fireEvent.keyDown(play, { key: 'ArrowDown' });
  fireEvent.keyDown(play, { key: 'Escape' });
  expect(onPlay).toHaveBeenCalledTimes(2);
});

test('the muted album column appears only when a title is passed (album pages omit it)', () => {
  const withAlbum = render(
    <TrackRow track={track} messages={destinationMessages()} albumTitle="Harbour Lights" onPlay={vi.fn()} />,
  );
  expect(withAlbum.container.querySelector('[data-track-album="1"]')?.textContent).toStrictEqual('Harbour Lights');
  expect(withAlbum.container.querySelector('[data-track-play]')?.getAttribute('data-with-album')).toStrictEqual('1');
  withAlbum.unmount();

  const withoutAlbum = render(<TrackRow track={track} messages={destinationMessages()} onPlay={vi.fn()} />);
  expect(withoutAlbum.container.querySelector('[data-track-album="1"]')).toBeNull();
  expect(withoutAlbum.container.querySelector('[data-track-play]')?.getAttribute('data-with-album')).toStrictEqual('0');
});

test('hostile rows replace corpus text with safe catalogue labels everywhere', () => {
  const hostileTrack: ShellTrack = {
    ...track,
    id: 'demo-track-08-01',
    title: hostileCorpus(),
    artistName: hostileCorpus(),
  };
  render(
    <TrackRow
      track={hostileTrack}
      messages={destinationMessages()}
      albumTitle={hostileCorpus()}
      hostile
      artistKey="hostile-artist"
      onOpenArtist={vi.fn()}
      onPlay={vi.fn()}
    />,
  );
  expect(document.querySelector('[data-track-row]')?.getAttribute('data-hostile')).toStrictEqual('1');
  expect(document.querySelector('[data-track-title="1"]')?.textContent).toStrictEqual('Hostile metadata (fixture)');
  expect(document.querySelector('[data-track-artist="1"]')?.textContent).toStrictEqual('Security corpus');
  expect(document.querySelector('[data-track-album="1"]')?.textContent).toStrictEqual('Hostile metadata (fixture)');
  expect(screen.getByRole('button', { name: 'Hostile metadata (fixture)' })).not.toBeNull();
  fireEvent.contextMenu(screen.getByRole('button', { name: 'Hostile metadata (fixture)' }));
  expect(screen.getByRole('menuitem', { name: 'Go to artist' })).not.toBeNull();
  // The corpus reaches neither the visible text nor the accessible tree.
  expect(document.body.textContent).not.toContain(hostileCorpus());
});

test('flags become text chips and mark the row; healthy rows stay unmarked', () => {
  const onPlay = vi.fn();
  const unplayable = render(
    <TrackRow track={{ ...track, flag: 'unplayable' }} messages={destinationMessages()} onPlay={onPlay} />,
  );
  expect(unplayable.container.querySelector('[data-track-row]')?.getAttribute('data-flagged')).toStrictEqual('1');
  expect(unplayable.container.querySelector('[data-track-flag]')?.textContent).toStrictEqual('Cannot play');
  expect(unplayable.container.querySelector('[data-track-flag]')?.getAttribute('data-track-flag')).toStrictEqual(
    'unplayable',
  );
  // A flagged row is not a play button: pressing it must not silently start
  // a different, playable track from the queue.
  const area = unplayable.container.querySelector('[data-track-play="1"]') as HTMLElement;
  expect(area.getAttribute('role')).toBeNull();
  expect(area.getAttribute('aria-disabled')).toStrictEqual('true');
  fireEvent.click(area);
  fireEvent.keyDown(area, { key: 'Enter' });
  expect(onPlay).not.toHaveBeenCalled();
  unplayable.unmount();

  const damaged = render(
    <TrackRow track={{ ...track, flag: 'damaged' }} messages={destinationMessages()} onPlay={vi.fn()} />,
  );
  expect(damaged.container.querySelector('[data-track-flag]')?.textContent).toStrictEqual('Damaged');
  damaged.unmount();

  const healthy = render(<TrackRow track={track} messages={destinationMessages()} onPlay={vi.fn()} />);
  expect(healthy.container.querySelector('[data-track-row]')?.getAttribute('data-flagged')).toStrictEqual('0');
  expect(healthy.container.querySelector('[data-track-flag]')).toBeNull();
});

test('the current row paints the now-playing state; other rows do not', () => {
  const current = render(<TrackRow track={track} messages={destinationMessages()} current onPlay={vi.fn()} />);
  expect(current.container.querySelector('[data-track-row]')?.getAttribute('data-current')).toStrictEqual('1');
  expect(current.container.querySelector('[data-now-playing="1"]')).not.toBeNull();
  current.unmount();

  const idle = render(<TrackRow track={track} messages={destinationMessages()} onPlay={vi.fn()} />);
  expect(idle.container.querySelector('[data-track-row]')?.getAttribute('data-current')).toStrictEqual('0');
  expect(idle.container.querySelector('[data-now-playing="1"]')).toBeNull();
});

test('durations render as tabular m:ss text', () => {
  const { container } = render(<TrackRow track={track} messages={destinationMessages()} onPlay={vi.fn()} />);
  expect(container.querySelector('[data-track-duration="1"]')?.textContent).toStrictEqual('3:34');
});

test('the stagger slot is the capped list position, whatever the table length', () => {
  const first = render(<TrackRow track={track} messages={destinationMessages()} staggerIndex={0} onPlay={vi.fn()} />);
  expect(first.container.querySelector('[data-track-row]')?.getAttribute('data-row-stagger')).toStrictEqual('0');
  first.unmount();

  const mid = render(<TrackRow track={track} messages={destinationMessages()} staggerIndex={4} onPlay={vi.fn()} />);
  expect(mid.container.querySelector('[data-track-row]')?.getAttribute('data-row-stagger')).toStrictEqual('4');
  mid.unmount();

  // Deep rows clamp to the last choreography slot.
  const deep = render(<TrackRow track={track} messages={destinationMessages()} staggerIndex={41} onPlay={vi.fn()} />);
  expect(deep.container.querySelector('[data-track-row]')?.getAttribute('data-row-stagger')).toStrictEqual('6');
  deep.unmount();

  // Default is slot 0 (search tables pass no index and stay still).
  const quiet = render(<TrackRow track={track} messages={destinationMessages()} onPlay={vi.fn()} />);
  expect(quiet.container.querySelector('[data-track-row]')?.getAttribute('data-row-stagger')).toStrictEqual('0');
});

test('go to artist context is only armed when both the key and the opener exist', () => {
  const messages = destinationMessages();
  const none = render(<TrackRow track={track} messages={messages} onPlay={vi.fn()} />);
  expect(none.container.querySelector('[data-item-more="1"]')).toBeNull();
  fireEvent.contextMenu(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(screen.queryByRole('menuitem', { name: 'Go to artist' })).toBeNull();
  none.unmount();

  const keyOnly = render(<TrackRow track={track} messages={messages} artistKey="mira-sol" onPlay={vi.fn()} />);
  expect(keyOnly.container.querySelector('[data-item-more="1"]')).toBeNull();
  keyOnly.unmount();

  const openerOnly = render(<TrackRow track={track} messages={messages} onPlay={vi.fn()} onOpenArtist={vi.fn()} />);
  expect(openerOnly.container.querySelector('[data-item-more="1"]')).toBeNull();
  openerOnly.unmount();
});

test('track context menu and more control open go to artist', () => {
  const onOpenArtist = vi.fn();
  const onPlay = vi.fn();
  render(
    <TrackRow
      track={track}
      messages={destinationMessages()}
      artistKey="mira-sol"
      onPlay={onPlay}
      onOpenArtist={onOpenArtist}
    />,
  );
  fireEvent.contextMenu(screen.getByRole('button', { name: 'Pier at Dusk' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(onOpenArtist).toHaveBeenCalledWith('mira-sol');
  expect(onPlay).not.toHaveBeenCalled();

  // The kebab is the "more" icon alone; its name is the aria-label.
  const more = screen.getByRole('button', { name: 'More' });
  expect(more.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('more');
  expect(more.textContent).toStrictEqual('');
  fireEvent.keyDown(screen.getByRole('button', { name: 'More' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: ' ' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  // The kebab is a toggle wired to its menu: Space opens, a key that is not
  // an activation key changes nothing, a second press closes.
  const kebab = screen.getByRole('button', { name: 'More' });
  expect(kebab.getAttribute('aria-haspopup')).toStrictEqual('menu');
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  fireEvent.keyDown(kebab, { key: ' ' });
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  expect(screen.getByRole('menu', { name: 'Actions' }).getAttribute('data-menu-id')).toStrictEqual(
    kebab.getAttribute('aria-controls'),
  );
  fireEvent.keyDown(kebab, { key: 'Tab' });
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.click(kebab);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  expect(screen.queryByRole('menu')).toStrictEqual(null);
  // Open again; a press on the kebab itself is left to the kebab, a press
  // anywhere else closes the menu.
  fireEvent.click(kebab);
  expect(screen.getAllByRole('menuitem').map((item) => item.getAttribute('aria-label'))).toStrictEqual([
    'Play',
    'Go to artist',
  ]);
  fireEvent.pointerDown(kebab);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.pointerDown(screen.getByRole('menuitem', { name: 'Play' }));
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.pointerDown(document.body);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  expect(screen.queryByRole('menu')).toStrictEqual(null);
});

test('catalogue menu plays queues and opens the album from the track row', () => {
  const onPlay = vi.fn();
  const onPlayNext = vi.fn();
  const onAddToQueue = vi.fn();
  const onGoToAlbum = vi.fn();
  const armed = render(
    <TrackRow
      track={track}
      messages={destinationMessages()}
      artistKey="mira-sol"
      onPlay={onPlay}
      onPlayNext={onPlayNext}
      onAddToQueue={onAddToQueue}
      onGoToAlbum={onGoToAlbum}
      onOpenArtist={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onPlay).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNext).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddToQueue).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onGoToAlbum).toHaveBeenCalledWith('demo-album-01');
  armed.unmount();

  const quiet = render(
    <TrackRow
      track={track}
      messages={destinationMessages()}
      artistKey="mira-sol"
      onPlay={onPlay}
      onOpenArtist={vi.fn()}
    />,
  );
  // Without the queue handlers the items are absent, not dead.
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  expect(screen.getAllByRole('menuitem').map((item) => item.getAttribute('aria-label'))).toStrictEqual([
    'Play',
    'Go to artist',
  ]);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onPlay).toHaveBeenCalledTimes(2);
  expect(onPlayNext).toHaveBeenCalledTimes(1);
  expect(onAddToQueue).toHaveBeenCalledTimes(1);
  expect(onGoToAlbum).toHaveBeenCalledTimes(1);
  quiet.unmount();
});

test('track context menu plays, queues without shuffle, and goes to the album', () => {
  const onPlay = vi.fn();
  const onPlayNext = vi.fn();
  const onAddToQueue = vi.fn();
  const onGoToAlbum = vi.fn();
  render(
    <TrackRow
      track={track}
      messages={destinationMessages()}
      artistKey="mira-sol"
      onPlay={onPlay}
      onPlayNext={onPlayNext}
      onAddToQueue={onAddToQueue}
      onGoToAlbum={onGoToAlbum}
      onOpenArtist={vi.fn()}
    />,
  );
  fireEvent.contextMenu(screen.getByRole('button', { name: 'Pier at Dusk' }));
  const labels = [...document.querySelectorAll('[data-menu-label="1"]')].map((node) => node.textContent);
  expect(labels).toStrictEqual(['Play', 'Play next', 'Add to queue', 'Go to album', 'Go to artist']);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onPlay).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNext).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddToQueue).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onGoToAlbum).toHaveBeenCalledWith('demo-album-01');
});
