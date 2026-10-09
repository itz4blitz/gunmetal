import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { artistTileData, Home } from './Home.tsx';

/** A lookup that must land: the test names what it could not find. */
function required<T extends Element>(node: T | null | undefined, what: string): T {
  if (node === null || node === undefined) {
    throw new Error(`${what} missing`);
  }
  return node;
}

afterEach(cleanup);

function hexButton(key: string): HTMLElement {
  const hex = document.querySelector(`#artist-tile-${key} [data-artist-hex="1"]`);
  expect(hex).not.toBeNull();
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
  expect(document.querySelector('#home-spotlight')).not.toBeNull();
  expect(document.querySelector('#cover-spotlight-demo-album-01')?.getAttribute('data-cover-art')).toStrictEqual('1');
  expect(document.querySelector('#home-spotlight [data-spotlight-eyebrow="1"]')?.textContent).toStrictEqual('Featured');
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).not.toBeNull();
  expect(document.querySelector('#home-spotlight [data-spotlight-artist="1"]')?.textContent).toStrictEqual('Mira Sol');
  expect(document.querySelector('#destination-headline')).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Home' })).toBeNull();
  expect([...document.querySelectorAll('#destination-home > [id^="home-"]')].map((node) => node.id)).toStrictEqual([
    'home-spotlight',
    'home-row-recent',
    'home-row-artists',
  ]);

  fireEvent.click(required(document.querySelector('#home-spotlight-play'), '#home-spotlight-play'));
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(required(document.querySelector('#home-spotlight-play'), '#home-spotlight-play'), { key: 'Enter' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(required(document.querySelector('#home-spotlight-play'), '#home-spotlight-play'), { key: ' ' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(required(document.querySelector('#home-spotlight-play'), '#home-spotlight-play'), { key: 'Tab' });
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
  expect(shelf).not.toBeNull();
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
  const secondOpen = screen.getAllByRole('button', { name: 'Mira Sol' })[1];
  if (secondOpen === undefined) {
    throw new Error('second Mira Sol button missing');
  }
  fireEvent.click(secondOpen);
  expect(onOpenArtist).toHaveBeenCalledTimes(4);
});

test('artists shelf falls back to tone plates and catalogue-safe labels', () => {
  const base = demoLibrary();
  const browsable = base.albums.filter((album) => album.id === 'demo-album-01');
  const hostile = base.albums.find((album) => album.hostile);
  if (hostile === undefined) {
    throw new Error('fixture hostile album missing');
  }
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
  expect(document.querySelector('#artist-tile-mira-sol')).not.toBeNull();
  expect(document.querySelector('#artist-tile-mixed')).not.toBeNull();
  // The hostile-adjacent artist renders the catalogue-safe label, not corpus text.
  expect(screen.getByText('Security corpus')).not.toBeNull();
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
  // Without onOpenArtist the artist tile still renders its open control; it
  // is honest about having nowhere to go — pressing it does nothing.
  fireEvent.click(required(document.querySelector('#artist-tile-mira-sol [data-artist-open="1"]'), 'artist open'));
  expect(document.querySelector('#destination-artist')).toBeNull();
});

test('home without browsable albums omits spotlight, art tone and shelves', () => {
  const base = demoLibrary();
  const hostile = base.albums.find((album) => album.hostile);
  if (hostile === undefined) {
    throw new Error('fixture hostile album missing');
  }
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
  expect(screen.getByText('No albums added yet')).not.toBeNull();
  // No browsable release → no spotlight at all, so no bloom and no kebab.
  expect(document.querySelector('[data-hero-bloom="1"]')).toBeNull();
  expect(document.querySelector('[data-spotlight-more="1"]')).toBeNull();
  // No shelf and no hostile-artist leak when nothing is browsable.
  expect(document.querySelector('#home-row-artists')).toBeNull();
  expect(screen.queryByText('Security corpus')).toBeNull();
});

test('spotlight carries the ambient bloom, meta line and a quiet album menu', () => {
  const library = demoLibrary();
  const onOpenAlbum = vi.fn();
  const onPlayNextAlbum = vi.fn();
  const onAddAlbumToQueue = vi.fn();
  const onOpenArtist = vi.fn();
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={onOpenAlbum}
      onOpenArtist={onOpenArtist}
      onPlayAlbum={vi.fn()}
      onPlayNextAlbum={onPlayNextAlbum}
      onAddAlbumToQueue={onAddAlbumToQueue}
      onSeeAll={vi.fn()}
    />,
  );

  // Ambient bloom: a decorative second copy of the spotlight cover — no
  // control role, painted from the same same-origin URL.
  const bloom = document.querySelector('#home-spotlight [data-hero-bloom="1"]');
  expect(bloom).not.toBeNull();
  expect(bloom?.getAttribute('role')).toStrictEqual(null);
  expect(bloom?.getAttribute('style')).toContain('url("/media/covers/demo-album-01.svg")');

  // Meta line under the artist: artist, year and track count from the catalogue,
  // drawn as a real text node (untrusted-text rule, design-language §6).
  expect(document.querySelector('#home-spotlight [data-spotlight-meta="1"]')?.textContent).toStrictEqual(
    'Mira Sol · 2021 · 4 tracks',
  );

  // The quiet kebab opens the same catalogue menu the tiles use. Shelf tiles
  // also carry a More button, so the hero one is addressed by its hook.
  const more = document.querySelector('[data-spotlight-more="1"]');
  expect(more).not.toBeNull();
  expect(screen.queryByRole('menuitem', { name: 'Play next' })).toBeNull();
  fireEvent.click(required(more, '[data-spotlight-more="1"]'));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNextAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.click(required(document.querySelector('[data-spotlight-more="1"]'), '[data-spotlight-more="1"]'));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddAlbumToQueue).toHaveBeenCalledWith('demo-album-01');
  // Escape closes it without firing the focused item.
  fireEvent.click(required(document.querySelector('[data-spotlight-more="1"]'), '[data-spotlight-more="1"]'));
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: 'Escape' });
  expect(screen.queryByRole('menuitem', { name: 'Go to album' })).toBeNull();
  fireEvent.click(required(document.querySelector('[data-spotlight-more="1"]'), '[data-spotlight-more="1"]'));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(onOpenArtist).toHaveBeenCalledWith('mira-sol');
  // Opening and using the menu never opens the album itself.
  expect(onOpenAlbum).not.toHaveBeenCalled();

  // The kebab is the shared "more" icon, a toggle wired to its menu: a second
  // press closes it, Enter opens it, and a press anywhere else closes it.
  const kebab = document.querySelector('[data-spotlight-more="1"]') as HTMLElement;
  expect(kebab.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('more');
  expect(kebab.textContent).toStrictEqual('');
  expect(kebab.getAttribute('aria-haspopup')).toStrictEqual('menu');
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  fireEvent.click(kebab);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  expect(screen.getByRole('menu', { name: 'Actions' }).getAttribute('data-menu-id')).toStrictEqual(
    kebab.getAttribute('aria-controls'),
  );
  fireEvent.click(kebab);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  expect(screen.queryByRole('menu')).toStrictEqual(null);
  fireEvent.keyDown(kebab, { key: 'Enter' });
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.keyDown(kebab, { key: 'Tab' });
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.pointerDown(document.querySelector('#home-spotlight') as HTMLElement);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  expect(screen.queryByRole('menu')).toStrictEqual(null);
  fireEvent.keyDown(kebab, { key: ' ' });
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.keyDown(kebab, { key: ' ' });
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
});

test('shelf headers count what they show, in muted tabular chrome', () => {
  const library = demoLibrary();
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onSeeAll={vi.fn()}
    />,
  );
  const recentCount = document.querySelector('#home-row-recent [data-shelf-count="1"]');
  expect(recentCount?.textContent).toStrictEqual('14');
  // Count chrome carries no role: the accessible header stays the bare title
  // (RN-web strips aria-hidden, so the bare number is the pinned contract).
  expect(recentCount?.getAttribute('role')).toStrictEqual(null);
  const artistCount = document.querySelector('#home-row-artists [data-shelf-count="1"]');
  expect(artistCount?.textContent).toStrictEqual('10');
  expect(artistCount?.getAttribute('role')).toStrictEqual(null);
});

test('the playing release rings its artist hex with the where-you-are state', () => {
  const library = demoLibrary();
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onSeeAll={vi.fn()}
      playingAlbumId="demo-album-01"
    />,
  );
  expect(document.querySelector('#artist-tile-mira-sol')?.getAttribute('data-artist-playing')).toStrictEqual('1');
  expect(document.querySelector('#artist-tile-keratin')?.getAttribute('data-artist-playing')).toStrictEqual('0');
  expect(document.querySelectorAll('[data-artist-playing="1"]')).toHaveLength(1);
});

test('area-home.css keeps the shelves on the render discipline', async () => {
  const { readFile } = await import('node:fs/promises');
  const { dirname, join } = await import('node:path');
  const { fileURLToPath } = await import('node:url');
  const here = dirname(fileURLToPath(import.meta.url));
  const css = await readFile(join(here, '../../../../../apps/demo/public/area-home.css'), 'utf8');
  expect(css.includes('content-visibility: auto')).toStrictEqual(true);
  expect(css.includes('[data-album-shelf] [data-album-tile]')).toStrictEqual(true);
  expect(css.includes('[data-artist-shelf] [data-artist-tile]')).toStrictEqual(true);
  expect(css.includes('contain-intrinsic-size: auto 180px auto 280px;')).toStrictEqual(true);
});

test('a partly resolvable artist tones from its first catalogue release and opens from the keyboard', () => {
  const library: ShellLibrary = {
    albums: demoLibrary().albums.slice(0, 1),
    artists: [{ key: 'ghost', name: 'Ghost', albumIds: ['demo-album-missing', 'demo-album-01'] }],
  };
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
  // The tone comes from the release the catalogue can actually resolve.
  const avatar = document.querySelector('#artist-tile-ghost [data-artist-avatar="1"]');
  expect(avatar?.getAttribute('data-cover-tone')).toStrictEqual('01');
  // The hex and the name both open the artist from the keyboard.
  fireEvent.keyDown(hexButton('ghost'), { key: 'Enter' });
  expect(onOpenArtist).toHaveBeenCalledWith('ghost');
  const nameButton = document.querySelector('#artist-tile-ghost [data-artist-open="1"]');
  expect(nameButton).not.toBeNull();
  fireEvent.keyDown(required(nameButton, 'artist open button'), { key: ' ' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(required(nameButton, 'artist open button'), { key: 'Tab' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
});

test('the spotlight kebab plays and opens the album directly', () => {
  const onPlayAlbum = vi.fn();
  const onOpenAlbum = vi.fn();
  render(
    <Home
      messages={destinationMessages()}
      library={demoLibrary()}
      onOpenAlbum={onOpenAlbum}
      onOpenArtist={vi.fn()}
      onPlayAlbum={onPlayAlbum}
      onPlayNextAlbum={vi.fn()}
      onAddAlbumToQueue={vi.fn()}
      onSeeAll={vi.fn()}
    />,
  );
  fireEvent.click(required(document.querySelector('[data-spotlight-more="1"]'), '[data-spotlight-more="1"]'));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.click(required(document.querySelector('[data-spotlight-more="1"]'), '[data-spotlight-more="1"]'));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onOpenAlbum).toHaveBeenCalledWith('demo-album-01');
});

test('artistTileData claims exactly the tone the catalogue resolved', () => {
  expect(artistTileData('01')).toStrictEqual({ artistAvatar: '1', coverTone: '01' });
  expect(artistTileData(undefined)).toStrictEqual({ artistAvatar: '1' });
});

test('a folder library spotlights the first covered album and shelves each cover', () => {
  const base = demoLibrary();
  const first = base.albums[0];
  const second = base.albums[1];
  if (first === undefined || second === undefined) {
    throw new Error('fixture albums missing');
  }
  const library: ShellLibrary = {
    albums: [
      {
        ...first,
        id: 'bare-album',
        title: 'Untitled Folder',
        artistName: 'No Cover',
        artistKey: 'no-cover',
        year: 0,
        coverUrl: '',
        tracks: first.tracks.slice(0, 1),
      },
      {
        ...second,
        id: 'covered-album',
        title: 'St. Elsewhere',
        artistName: 'Gnarls Barkley',
        artistKey: 'gnarls',
        year: 2006,
        coverUrl: '/media/library/covers/covered.jpg',
        tracks: second.tracks.slice(0, 2),
      },
    ],
    artists: [],
  };
  const onPlayAlbum = vi.fn();
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={onPlayAlbum}
      onSeeAll={vi.fn()}
    />,
  );

  expect(screen.getByRole('heading', { name: 'St. Elsewhere' })).not.toBeNull();
  expect(screen.queryByRole('heading', { name: 'Untitled Folder' })).toBeNull();
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toStrictEqual('02');
  expect(document.querySelector('#home-spotlight [data-spotlight-meta="1"]')?.textContent).toStrictEqual(
    'Gnarls Barkley · 2006 · 2 tracks',
  );
  expect(document.querySelector('#home-spotlight [data-hero-bloom="1"]')?.getAttribute('style')).toContain(
    'url("/media/library/covers/covered.jpg")',
  );
  expect(document.querySelector('[data-empty-row="1"]')).toBeNull();
  expect(screen.queryByText('No albums added yet')).toBeNull();
  expect(document.querySelector('#album-tile-bare-album')).not.toBeNull();
  expect(document.querySelector('#album-tile-covered-album')).not.toBeNull();
  expect(document.querySelector('#cover-grid-bare-album')?.getAttribute('data-cover-art')).toStrictEqual('0');
  expect(document.querySelector('#cover-grid-covered-album')?.getAttribute('data-cover-art')).toStrictEqual('1');
  expect(document.querySelector('#cover-grid-covered-album')?.getAttribute('style')).toContain(
    'url("/media/library/covers/covered.jpg")',
  );
  expect(document.querySelector('#cover-spotlight-covered-album')?.getAttribute('style')).toContain(
    'url("/media/library/covers/covered.jpg")',
  );
  fireEvent.click(required(document.querySelector('#home-spotlight-play'), '#home-spotlight-play'));
  expect(onPlayAlbum).toHaveBeenCalledWith('covered-album');
});

test('a spotlight without a year or artwork claims neither', () => {
  const base = demoLibrary();
  const first = base.albums[0];
  if (first === undefined) {
    throw new Error('fixture album missing');
  }
  const quiet: ShellLibrary = {
    albums: [
      { ...first, year: 0, coverUrl: '' },
      ...base.albums.slice(1, 3).map((album) => ({ ...album, coverUrl: '' })),
    ],
    artists: base.artists,
  };
  render(
    <Home
      messages={destinationMessages()}
      library={quiet}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onSeeAll={vi.fn()}
    />,
  );
  // No covered release: the first album is the spotlight, and a missing year is omitted.
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).not.toBeNull();
  expect(document.querySelector('[data-spotlight-meta="1"]')?.textContent).toStrictEqual('Mira Sol · 4 tracks');
  // No artwork URL: the hero renders without its ambient bloom.
  expect(document.querySelector('[data-hero-bloom="1"]')).toBeNull();
  // And without art the spotlight carries no art-tone wash either.
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toStrictEqual('01');
});
