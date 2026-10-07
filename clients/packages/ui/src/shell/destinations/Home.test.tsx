import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { Home } from './Home.tsx';

afterEach(cleanup);

function hexButton(key: string): HTMLElement {
  const hex = document.querySelector(`#artist-tile-${key} [data-artist-hex="1"]`);
  expect(hex).toBeTruthy();
  return hex as HTMLElement;
}

test('spotlight uses the first recently-added fixture with art tone and actions', () => {
  const library = demoLibrary();
  const onOpenAlbum = vi.fn();
  const onPlayAlbum = vi.fn();
  const onSeeAll = vi.fn();
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={onOpenAlbum}
      onPlayAlbum={onPlayAlbum}
      onSeeAll={onSeeAll}
    />,
  );

  const home = document.querySelector('#destination-home');
  expect(home?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(document.querySelector('#home-spotlight')).toBeTruthy();
  expect(document.querySelector('#cover-spotlight-demo-album-01')?.getAttribute('data-cover-art')).toStrictEqual('1');
  expect(document.querySelector('#home-spotlight [data-spotlight-eyebrow="1"]')?.textContent).toStrictEqual('Featured');
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).toBeTruthy();
  expect(document.querySelector('#home-spotlight [data-spotlight-artist="1"]')?.textContent).toStrictEqual('Mira Sol');
  expect(document.querySelector('#destination-headline')).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Home' })).toBeNull();
  expect([...document.querySelectorAll('#destination-home > [id^="home-"]')].map((node) => node.id)).toStrictEqual([
    'home-spotlight',
    'home-row-recent',
    'home-row-artists',
  ]);

  fireEvent.click(document.querySelector('#home-spotlight-play')!);
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(document.querySelector('#home-spotlight-play')!, { key: 'Enter' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(document.querySelector('#home-spotlight-play')!, { key: ' ' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(document.querySelector('#home-spotlight-play')!, { key: 'Tab' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);

  fireEvent.click(screen.getByRole('button', { name: 'Go to album' }));
  expect(onOpenAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to album' }), { key: 'Enter' });
  expect(onOpenAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to album' }), { key: ' ' });
  expect(onOpenAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to album' }), { key: 'Tab' });
  expect(onOpenAlbum).toHaveBeenCalledTimes(3);

  fireEvent.click(screen.getByRole('button', { name: 'See all' }));
  expect(onSeeAll).toHaveBeenCalledTimes(1);
  fireEvent.keyDown(screen.getByRole('button', { name: 'See all' }), { key: 'Enter' });
  expect(onSeeAll).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'See all' }), { key: ' ' });
  expect(onSeeAll).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'See all' }), { key: 'Tab' });
  expect(onSeeAll).toHaveBeenCalledTimes(3);
});

test('artists shelf lists browsable artists only and opens artist pages', () => {
  const library = demoLibrary();
  const onOpenArtist = vi.fn();
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onOpenArtist={onOpenArtist}
      onPlayAlbum={vi.fn()}
      onSeeAll={vi.fn()}
    />,
  );

  const shelf = document.querySelector('#home-row-artists [data-artist-shelf="1"]');
  expect(shelf).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Artists' }).getAttribute('data-home-title')).toStrictEqual('1');
  // Five demo artists have browsable releases; the hostile-only artist stays
  // off Home (Library owns the corpus with its label swap).
  const keys = [...document.querySelectorAll('#home-row-artists [data-artist-tile]')].map((node) =>
    node.getAttribute('data-artist-tile'),
  );
  expect(keys).toStrictEqual([
    'alex-reed-north',
    'alex-reed-south',
    'chris-zabriskie',
    'kai-engel',
    'keratin',
    'kevin-macleod',
    'mira-sol',
    'scott-buckley',
    'the-compound',
    'various-artists',
  ]);
  expect(document.querySelector('#artist-tile-hostile-artist')).toBeNull();
  // Artist art uses the same-origin generated avatars, tone behind.
  expect(document.querySelector('#artist-tile-mira-sol [data-artist-avatar="1"]')?.getAttribute('style')).toContain(
    'url("/media/artists/mira-sol.svg")',
  );
  expect(
    document.querySelector('#artist-tile-mira-sol [data-artist-avatar="1"]')?.getAttribute('data-cover-tone'),
  ).toStrictEqual('01');
  // Names are real text nodes, truncated visually only.
  expect(document.querySelector('#artist-tile-mira-sol [data-artist-tile-name="1"]')?.textContent).toStrictEqual(
    'Mira Sol',
  );

  fireEvent.click(hexButton('mira-sol'));
  expect(onOpenArtist).toHaveBeenCalledWith('mira-sol');
  fireEvent.keyDown(hexButton('mira-sol'), { key: 'Enter' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(hexButton('mira-sol'), { key: ' ' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(hexButton('mira-sol'), { key: 'Tab' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
  fireEvent.click(screen.getAllByRole('button', { name: 'Mira Sol' })[1]!);
  expect(onOpenArtist).toHaveBeenCalledTimes(4);
});

test('artists shelf falls back to tone plates and catalogue-safe labels', () => {
  const base = demoLibrary();
  const browsable = base.albums.filter((album) => album.id === 'demo-album-01');
  const hostile = base.albums.find((album) => album.hostile)!;
  const library: ShellLibrary = {
    albums: [...browsable, hostile],
    artists: [
      // Artist without an avatar image: tone plate + initial (never a letter poster).
      { key: 'mira-sol', name: 'Mira Sol', albumIds: ['demo-album-01'] },
      // Mixed artist: browsable release plus a hostile one → catalogue-safe label.
      { key: 'mixed', name: 'Corpus Mixed', albumIds: ['demo-album-01', hostile.id] },
    ],
  };
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onOpenArtist={vi.fn()}
      onPlayAlbum={vi.fn()}
      onSeeAll={vi.fn()}
    />,
  );
  expect(document.querySelector('#artist-tile-mira-sol')).toBeTruthy();
  expect(document.querySelector('#artist-tile-mixed')).toBeTruthy();
  // The hostile-adjacent artist renders the catalogue-safe label, not corpus text.
  expect(screen.getByText('Security corpus')).toBeTruthy();
  const avatar = document.querySelector('#artist-tile-mira-sol [data-artist-avatar="1"]');
  expect(avatar?.getAttribute('style')).toBeNull();
  expect(avatar?.getAttribute('data-cover-tone')).toStrictEqual('01');
  expect(document.querySelector('#artist-tile-mira-sol [data-artist-initial="1"]')?.textContent).toStrictEqual('M');
});

test('playingAlbumId marks one tile with the brass where-you-are state', () => {
  const library = demoLibrary();
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onSeeAll={vi.fn()}
      playingAlbumId="demo-album-02"
    />,
  );
  const playing = document.querySelector('[data-album-tile="demo-album-02"]');
  expect(playing?.getAttribute('data-tile-playing')).toStrictEqual('1');
  const resting = document.querySelector('[data-album-tile="demo-album-01"]');
  expect(resting?.getAttribute('data-tile-playing')).toStrictEqual('0');
  expect(document.querySelectorAll('[data-tile-playing="1"]')).toHaveLength(1);
});

test('home without browsable albums omits spotlight, art tone and shelves', () => {
  const base = demoLibrary();
  const hostile = base.albums.find((album) => album.hostile)!;
  const library: ShellLibrary = {
    albums: [hostile],
    artists: base.artists,
  };
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onSeeAll={vi.fn()}
    />,
  );
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toBeNull();
  expect(document.querySelector('#home-spotlight')).toBeNull();
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  expect(screen.queryByRole('button', { name: 'See all' })).toBeNull();
  expect(screen.getByText('No albums added yet')).toBeTruthy();
  // No shelf and no hostile-artist leak when nothing is browsable.
  expect(document.querySelector('#home-row-artists')).toBeNull();
  expect(screen.queryByText('Security corpus')).toBeNull();
});
