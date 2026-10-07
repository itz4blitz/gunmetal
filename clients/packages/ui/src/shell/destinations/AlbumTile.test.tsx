import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';

afterEach(cleanup);

const messages = {
  hostileAlbumLabel: 'Hostile metadata (fixture)',
  hostileArtistLabel: 'Hostile artist (fixture)',
  playAlbum: 'Play album',
  play: 'Play',
  playNext: 'Play next',
  addToQueue: 'Add to queue',
  goToAlbum: 'Go to album',
  goToArtist: 'Go to artist',
  moreActions: 'More',
  contextMenu: 'Actions',
} as DestinationMessages;

const album: ShellAlbum = {
  id: 'demo-album-1',
  title: 'Harbour Lights',
  artistName: 'Keratin',
  artistKey: 'keratin',
  year: 2024,
  coverTone: '02',
  coverUrl: '/media/covers/fixture.svg',
  hostile: false,
  discs: [{ index: 1, title: '' }],
  tracks: [
    {
      id: 't1',
      albumId: 'demo-album-1',
      discIndex: 1,
      number: 1,
      title: 'Pier at Dusk',
      artistName: 'Keratin',
      durationMs: 180_000,
      flag: 'ok',
      lyricsKind: 'none',
      mediaUrl: '/media/audio/fixtures.wav',
    },
  ],
};

test('album tile shows title artist hierarchy and opens on activate', () => {
  const onOpen = vi.fn();
  render(<AlbumTile album={album} messages={messages} onOpen={onOpen} />);
  expect(screen.getByText('Harbour Lights')).toBeTruthy();
  expect(screen.getByText('Keratin')).toBeTruthy();
  expect(document.querySelector('[data-album-title="1"]')?.textContent).toStrictEqual('Harbour Lights');
  expect(document.querySelector('[data-album-artist="1"]')?.textContent).toStrictEqual('Keratin');
  fireEvent.click(document.querySelector('[data-album-art="1"]')!);
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(onOpen).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: 'Enter' });
  expect(onOpen).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: ' ' });
  expect(onOpen).toHaveBeenCalledTimes(4);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: 'Tab' });
  expect(onOpen).toHaveBeenCalledTimes(4);
});

test('play control is a sibling of open — never a nested button', () => {
  const { container } = render(<AlbumTile album={album} messages={messages} onOpen={vi.fn()} onPlay={vi.fn()} />);
  expect(container.querySelectorAll('button button')).toHaveLength(0);
  const open = screen.getByRole('button', { name: 'Harbour Lights' });
  const play = screen.getByRole('button', { name: 'Play album' });
  expect(open.contains(play)).toStrictEqual(false);
  expect(play.contains(open)).toStrictEqual(false);
  expect(play.closest('[data-album-art="1"]')).toBeTruthy();
  expect(open.getAttribute('data-album-open')).toStrictEqual('1');
});

test('play affordance calls onPlay when provided otherwise onOpen', () => {
  const onOpen = vi.fn();
  const onPlay = vi.fn();
  const withPlay = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} onPlay={onPlay} />);
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(onPlay).toHaveBeenCalledWith('demo-album-1');
  expect(onOpen).not.toHaveBeenCalled();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Enter' });
  expect(onPlay).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: ' ' });
  expect(onPlay).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Tab' });
  expect(onPlay).toHaveBeenCalledTimes(3);
  withPlay.unmount();

  const openOnly = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} />);
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  openOnly.unmount();
});

test('playing prop marks the tile with the brass where-you-are state', () => {
  const onOpen = vi.fn();
  const playing = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} playing />);
  const tile = document.querySelector('#album-tile-demo-album-1');
  expect(tile?.getAttribute('data-tile-playing')).toStrictEqual('1');
  // The state is presentation only: open still works from title and art.
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  playing.unmount();

  const resting = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} />);
  expect(document.querySelector('#album-tile-demo-album-1')?.getAttribute('data-tile-playing')).toStrictEqual('0');
  resting.unmount();
});

test('hostile album uses catalogue labels on the tile', () => {
  const onOpen = vi.fn();
  render(
    <AlbumTile
      album={{ ...album, hostile: true, title: '<script>', artistName: 'x' }}
      messages={messages}
      onOpen={onOpen}
    />,
  );
  expect(screen.getByRole('button', { name: 'Hostile metadata (fixture)' })).toBeTruthy();
  expect(screen.getByText('Hostile artist (fixture)')).toBeTruthy();
  // Grid covers carry no letter — the hostile label lives in the tile text.
  expect(document.querySelector('[data-cover-label]')).toBeNull();
});

test('stagger slots and go to artist open from context and more', () => {
  const onOpenArtist = vi.fn();
  const first = render(<AlbumTile album={album} messages={messages} onOpen={vi.fn()} staggerIndex={3} />);
  expect(document.querySelector('[data-tile-stagger="3"]')).toBeTruthy();
  expect(first.container.querySelector('[data-item-more="1"]')).toBeNull();
  fireEvent.contextMenu(document.querySelector('#album-tile-demo-album-1')!);
  expect(screen.queryByRole('menuitem', { name: 'Go to artist' })).toBeNull();
  first.unmount();

  const capped = render(
    <AlbumTile album={album} messages={messages} onOpen={vi.fn()} onOpenArtist={onOpenArtist} staggerIndex={12} />,
  );
  expect(document.querySelector('[data-tile-stagger="6"]')).toBeTruthy();
  fireEvent.contextMenu(document.querySelector('#album-tile-demo-album-1')!);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(onOpenArtist).toHaveBeenCalledWith('keratin');
  fireEvent.keyDown(screen.getByRole('button', { name: 'More' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: 'Escape' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'More' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'More' }), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  expect(screen.getByRole('menuitem', { name: 'Go to artist' })).toBeTruthy();
  capped.unmount();
});

test('catalogue menu plays queues and opens the album from the tile', () => {
  const onOpen = vi.fn();
  const onPlay = vi.fn();
  const onPlayNext = vi.fn();
  const onAddToQueue = vi.fn();
  const armed = render(
    <AlbumTile
      album={album}
      messages={messages}
      onOpen={onOpen}
      onPlay={onPlay}
      onPlayNext={onPlayNext}
      onAddToQueue={onAddToQueue}
      onOpenArtist={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onPlay).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNext).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddToQueue).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  armed.unmount();

  const quiet = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} onOpenArtist={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onPlay).toHaveBeenCalledTimes(1);
  expect(onPlayNext).toHaveBeenCalledTimes(1);
  expect(onAddToQueue).toHaveBeenCalledTimes(1);
  quiet.unmount();
});

test('album context menu plays, queues without shuffle, and opens the album', () => {
  const onOpen = vi.fn();
  const onPlay = vi.fn();
  const onPlayNext = vi.fn();
  const onAddToQueue = vi.fn();
  render(
    <AlbumTile
      album={album}
      messages={messages}
      onOpen={onOpen}
      onPlay={onPlay}
      onPlayNext={onPlayNext}
      onAddToQueue={onAddToQueue}
      onOpenArtist={vi.fn()}
    />,
  );
  fireEvent.contextMenu(document.querySelector('#album-tile-demo-album-1')!);
  const labels = [...document.querySelectorAll('[data-menu-label="1"]')].map((node) => node.textContent);
  expect(labels).toStrictEqual(['Play', 'Play next', 'Add to queue', 'Go to album', 'Go to artist']);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onPlay).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNext).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddToQueue).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
});
