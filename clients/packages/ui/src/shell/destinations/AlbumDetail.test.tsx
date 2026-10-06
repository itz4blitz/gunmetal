import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum } from '../library-types.ts';
import { AlbumDetail } from './AlbumDetail.tsx';

afterEach(cleanup);

const album: ShellAlbum = {
  id: 'demo-album-01',
  title: 'Harbour Lights',
  artistName: 'Mira Sol',
  artistKey: 'mira-sol',
  year: 2021,
  coverTone: '01',
  hostile: false,
  discs: [{ index: 1, title: '' }],
  tracks: [
    {
      id: 'demo-track-01-01',
      albumId: 'demo-album-01',
      discIndex: 1,
      number: 1,
      title: 'Pier at Dusk',
      artistName: 'Mira Sol',
      durationMs: 214_000,
      flag: 'ok',
      lyricsKind: 'none',
    },
    {
      id: 'demo-track-01-02',
      albumId: 'demo-album-01',
      discIndex: 1,
      number: 2,
      title: 'Salt Window',
      artistName: 'Mira Sol',
      durationMs: 198_000,
      flag: 'ok',
      lyricsKind: 'none',
    },
  ],
};

test('album detail paints a full-bleed cover-tone header with year track count and brass play', () => {
  const onPlayAlbum = vi.fn();
  const { container } = render(
    <AlbumDetail
      album={album}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={onPlayAlbum}
      onPlayTrack={vi.fn()}
    />,
  );
  const root = container.querySelector('#destination-album');
  expect(root?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(container.querySelector('[data-album-header-large="1"]')).toBeTruthy();
  expect(container.querySelector('[data-album-header-bleed="1"]')).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual(
    'destination-headline',
  );
  expect(container.querySelector('[data-album-artist]')?.textContent).toStrictEqual('Mira Sol');
  expect(screen.getByText('Year 2021')).toBeTruthy();
  expect(screen.getByText('2 tracks').id).toStrictEqual('album-track-count');
  expect(container.querySelector('#album-play[data-brass-hex="1"]')).toBeTruthy();
  fireEvent.click(container.querySelector('[data-album-artist]')!);
  fireEvent.keyDown(container.querySelector('[data-album-artist]')!, { key: 'Enter' });
  fireEvent.keyDown(container.querySelector('[data-album-artist]')!, { key: ' ' });
  fireEvent.keyDown(container.querySelector('[data-album-artist]')!, { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
});

test('album artist control goes to the artist when the opener is provided', () => {
  const onOpenArtist = vi.fn();
  render(
    <AlbumDetail
      album={album}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
      onOpenArtist={onOpenArtist}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Go to artist' }));
  expect(onOpenArtist).toHaveBeenCalledWith('mira-sol');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to artist' }), { key: 'Enter' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to artist' }), { key: ' ' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to artist' }), { key: 'Tab' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
});

test('track rows mark the current track as now playing', () => {
  const onPlayTrack = vi.fn();
  const { container, rerender } = render(
    <AlbumDetail
      album={album}
      messages={destinationMessages()}
      currentTrackId="demo-track-01-02"
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={onPlayTrack}
    />,
  );
  const current = container.querySelector('#track-row-demo-track-01-02');
  const other = container.querySelector('#track-row-demo-track-01-01');
  expect(current?.getAttribute('data-current')).toStrictEqual('1');
  expect(current?.querySelector('[data-now-playing="1"]')).toBeTruthy();
  expect(other?.getAttribute('data-current')).toStrictEqual('0');
  expect(other?.querySelector('[data-now-playing="1"]')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  rerender(
    <AlbumDetail
      album={album}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={onPlayTrack}
    />,
  );
  expect(container.querySelector('#track-row-demo-track-01-02')?.getAttribute('data-current')).toStrictEqual(
    '0',
  );
});

test('album lyrics toggle paints fixture lines as Text and highlights synced first line', () => {
  const withLyrics = {
    ...album,
    tracks: [
      album.tracks[0]!,
      {
        ...album.tracks[1]!,
        id: 'demo-track-02-02',
        title: 'Freight Elevator',
        lyricsKind: 'synced' as const,
      },
    ],
  };
  const { container } = render(
    <AlbumDetail
      album={withLyrics}
      messages={destinationMessages()}
      currentTrackId="demo-track-02-02"
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  expect(screen.getByRole('button', { name: 'Lyrics' }).id).toStrictEqual('album-lyrics-toggle');
  expect(document.querySelector('#album-lyrics')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  expect(container.querySelector('#album-lyrics')?.getAttribute('data-synced')).toStrictEqual('1');
  expect(
    [...container.querySelectorAll('#album-lyrics [data-lyrics-line="1"]')].map((node) => node.textContent),
  ).toStrictEqual([
    'Floors count themselves in the dark',
    'steel doors, a held breath',
    'then the motor starts',
  ]);
  expect(container.querySelector('#album-lyrics [data-current="1"]')?.textContent).toStrictEqual(
    'Floors count themselves in the dark',
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Enter' });
  expect(document.querySelector('#album-lyrics')).toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: ' ' });
  expect(document.querySelector('#album-lyrics')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Tab' });
  expect(document.querySelector('#album-lyrics')).toBeTruthy();
});

test('album lyrics fall back to the first plain or synced track when the current row has none', () => {
  const withLyrics = {
    ...album,
    tracks: [
      album.tracks[0]!,
      {
        ...album.tracks[1]!,
        id: 'demo-track-01-03',
        title: 'Letter Under Glass',
        lyricsKind: 'plain' as const,
      },
    ],
  };
  const { container, rerender } = render(
    <AlbumDetail
      album={withLyrics}
      messages={destinationMessages()}
      currentTrackId="demo-track-01-01"
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  expect(container.querySelector('#album-lyrics')?.getAttribute('data-synced')).toStrictEqual('0');
  expect(
    [...container.querySelectorAll('#album-lyrics [data-lyrics-line="1"]')].map((node) => node.textContent),
  ).toStrictEqual([
    'The harbour keeps the letter',
    'folded under glass',
    'until the tide comes back',
  ]);
  expect(container.querySelector('#album-lyrics [data-current="1"]')).toBeNull();
  rerender(
    <AlbumDetail
      album={withLyrics}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  expect(document.querySelector('#album-lyrics')).toBeTruthy();
  expect(
    [...container.querySelectorAll('#album-lyrics [data-lyrics-line="1"]')].map((node) => node.textContent)[0],
  ).toStrictEqual('The harbour keeps the letter');
});
