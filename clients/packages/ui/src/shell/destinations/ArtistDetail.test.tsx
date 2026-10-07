import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { ArtistDetail } from './ArtistDetail.tsx';

afterEach(cleanup);

test('artist detail shows hero image name album grid all songs and plays the first album', () => {
  const library = demoLibrary();
  const artist = library.artists.find((row) => row.key === 'mira-sol');
  const onPlayAlbum = vi.fn();
  const onOpenAlbum = vi.fn();
  const onPlayTrack = vi.fn();
  const { container } = render(
    <ArtistDetail
      artist={artist}
      library={library}
      messages={destinationMessages()}
      currentTrackId="demo-track-02-02"
      onBack={vi.fn()}
      onOpenAlbum={onOpenAlbum}
      onPlayAlbum={onPlayAlbum}
      onPlayTrack={onPlayTrack}
    />,
  );
  const root = container.querySelector('#destination-artist');
  expect(root?.getAttribute('data-artist-key')).toStrictEqual('mira-sol');
  expect(root?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(container.querySelector('[data-artist-hero="1"]')).toBeTruthy();
  // The generated artist image fills the nut-shaped hero; the initial stays as fallback.
  expect(container.querySelector('[data-artist-avatar-nut="1"]')).toBeTruthy();
  const hero = container.querySelector('[data-artist-avatar="hero"]') as HTMLElement;
  expect(hero.getAttribute('data-artist-image')).toStrictEqual('1');
  expect(hero.style.backgroundImage).toContain('/media/artists/mira-sol.svg');
  expect(hero.textContent).toStrictEqual('M');
  expect(screen.getByRole('heading', { name: 'Mira Sol' }).id).toStrictEqual('destination-headline');
  expect(container.querySelector('#artist-disambiguation')).toBeNull();
  expect(container.querySelector('[data-artist-album-count="1"]')?.textContent).toStrictEqual('2 albums');
  expect(container.querySelector('#artist-play[data-brass-hex="1"]')).toBeTruthy();
  expect(container.querySelector('#artist-album-grid')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Harbour Lights' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Night Shift' })).toBeTruthy();
  fireEvent.click(document.querySelector('#artist-play')!);
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: 'Enter' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: ' ' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: 'Tab' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.click(screen.getByRole('button', { name: 'Night Shift' }));
  expect(onOpenAlbum).toHaveBeenCalledWith('demo-album-02');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });

  // All songs reads across the artist's own albums (4 + 3 tracks) and plays a row.
  const songs = container.querySelector('#artist-all-songs');
  expect(songs).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'All songs' })).toBeTruthy();
  expect(container.querySelectorAll('[data-artist-song-list="1"] [data-track-row]')).toHaveLength(7);
  expect(
    [...container.querySelectorAll('[data-artist-song-list="1"] [data-track-album]')].map((node) => node.textContent),
  ).toStrictEqual([
    'Harbour Lights',
    'Harbour Lights',
    'Harbour Lights',
    'Harbour Lights',
    'Night Shift',
    'Night Shift',
    'Night Shift',
  ]);
  const currentRow = container.querySelector('#track-row-demo-track-02-02');
  expect(currentRow?.getAttribute('data-current')).toStrictEqual('1');
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
});

test('same-name artists carry the catalogue-key disambiguation line; unique names do not', () => {
  const library = demoLibrary();
  const alex = library.artists.find((row) => row.key === 'alex-reed-north');
  const { unmount } = render(
    <ArtistDetail
      artist={alex}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Alex Reed' })).toBeTruthy();
  expect(document.querySelector('#artist-disambiguation')?.textContent).toStrictEqual('alex-reed-north');
  unmount();

  const mira = library.artists.find((row) => row.key === 'mira-sol');
  render(
    <ArtistDetail
      artist={mira}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
    />,
  );
  expect(document.querySelector('#artist-disambiguation')).toBeNull();
});

test('missing artist and an artist with no albums stay on chrome without a wash', () => {
  const library = demoLibrary();
  const onBack = vi.fn();
  const onPlayAlbum = vi.fn();
  const missing = render(
    <ArtistDetail
      artist={undefined}
      library={library}
      messages={destinationMessages()}
      onBack={onBack}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={onPlayAlbum}
    />,
  );
  expect(screen.getByRole('heading', { name: 'That artist is not in the demo library' }).id).toStrictEqual(
    'destination-headline',
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: ' ' });
  expect(onBack).toHaveBeenCalledTimes(1);
  missing.unmount();

  const emptyLibrary: ShellLibrary = {
    albums: library.albums,
    artists: [{ key: 'lonely', name: '   ', albumIds: ['missing-album'] }],
  };
  const empty = render(
    <ArtistDetail
      artist={emptyLibrary.artists[0]}
      library={emptyLibrary}
      messages={destinationMessages()}
      onBack={onBack}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={onPlayAlbum}
    />,
  );
  expect(empty.container.querySelector('#destination-artist')?.getAttribute('data-art-tone')).toBeNull();
  const hero = empty.container.querySelector('[data-artist-avatar="hero"]') as HTMLElement;
  expect(hero.getAttribute('data-artist-image')).toStrictEqual('0');
  expect(hero.textContent).toStrictEqual('?');
  expect(document.querySelector('#destination-headline')?.textContent).toStrictEqual('   ');
  fireEvent.click(document.querySelector('#artist-play')!);
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: 'Enter' });
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: ' ' });
  expect(onPlayAlbum).not.toHaveBeenCalled();
  expect(document.querySelector('#artist-all-songs')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(onBack).toHaveBeenCalledTimes(2);
});

test('hostile artist chrome uses the catalogue label and its albums never list songs', () => {
  const library = demoLibrary();
  const artist = library.artists.find((row) => row.key === 'hostile-artist');
  const onPlayTrack = vi.fn();
  const { container } = render(
    <ArtistDetail
      artist={artist}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={onPlayTrack}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Security corpus' }).id).toStrictEqual('destination-headline');
  const hero = container.querySelector('[data-artist-avatar="hero"]') as HTMLElement;
  expect(hero.getAttribute('data-artist-image')).toStrictEqual('0');
  expect(hero.textContent).toStrictEqual('S');
  expect(container.querySelector('#artist-all-songs')).toBeNull();
  expect(container.textContent).not.toContain('onerror');
  expect(container.textContent).not.toContain('__gm_xss');
});
