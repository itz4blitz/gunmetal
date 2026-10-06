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
} as DestinationMessages;

const album: ShellAlbum = {
  id: 'demo-album-1',
  title: 'Harbour Lights',
  artistName: 'Keratin',
  artistKey: 'keratin',
  year: 2024,
  coverTone: '02',
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
    },
  ],
};

test('album tile shows title artist hierarchy and opens on activate', () => {
  const onOpen = vi.fn();
  render(<AlbumTile album={album} messages={messages} onOpen={onOpen} />);
  expect(screen.getByText('Harbour Lights')).toBeTruthy();
  expect(screen.getByText('Keratin')).toBeTruthy();
  expect(document.querySelector('[data-album-title="1"]')?.textContent).toStrictEqual(
    'Harbour Lights',
  );
  expect(document.querySelector('[data-album-artist="1"]')?.textContent).toStrictEqual('Keratin');
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: 'Enter' });
  expect(onOpen).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: ' ' });
  expect(onOpen).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: 'Tab' });
  expect(onOpen).toHaveBeenCalledTimes(3);
});

test('play affordance calls onPlay when provided otherwise onOpen', () => {
  const onOpen = vi.fn();
  const onPlay = vi.fn();
  const withPlay = render(
    <AlbumTile album={album} messages={messages} onOpen={onOpen} onPlay={onPlay} />,
  );
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
  expect(document.querySelector('[data-cover-label]')?.textContent).toStrictEqual('H');
});
