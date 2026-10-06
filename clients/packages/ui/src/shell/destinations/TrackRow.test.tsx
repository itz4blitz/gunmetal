import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
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
};

test('go to artist context is only armed when both the key and the opener exist', () => {
  const messages = destinationMessages();
  const none = render(<TrackRow track={track} messages={messages} onPlay={vi.fn()} />);
  expect(none.container.querySelector('[data-item-more="1"]')).toBeNull();
  fireEvent.contextMenu(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(screen.queryByRole('menuitem', { name: 'Go to artist' })).toBeNull();
  none.unmount();

  const keyOnly = render(
    <TrackRow track={track} messages={messages} artistKey="mira-sol" onPlay={vi.fn()} />,
  );
  expect(keyOnly.container.querySelector('[data-item-more="1"]')).toBeNull();
  keyOnly.unmount();

  const openerOnly = render(
    <TrackRow track={track} messages={messages} onPlay={vi.fn()} onOpenArtist={vi.fn()} />,
  );
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

  fireEvent.keyDown(screen.getByRole('button', { name: 'More' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: ' ' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'More' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'More' }), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  expect(screen.getByRole('menuitem', { name: 'Go to artist' })).toBeTruthy();
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
