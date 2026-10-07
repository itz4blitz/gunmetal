import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { hostileCorpus } from '../../../../fake-server/src/hostile.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { Library } from './Library.tsx';

afterEach(cleanup);

const here = dirname(fileURLToPath(import.meta.url));

type LibraryHandlers = {
  onOpenAlbum: (albumId: string) => void;
  onOpenArtist: (artistKey: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
};

function renderLibrary(
  library: ShellLibrary = demoLibrary(),
  overrides: { currentTrackId?: string } = {},
): LibraryHandlers {
  const handlers: LibraryHandlers = {
    onOpenAlbum: vi.fn(),
    onOpenArtist: vi.fn(),
    onPlayAlbum: vi.fn(),
    onPlayTrack: vi.fn(),
  };
  render(
    <Library
      messages={destinationMessages()}
      library={library}
      currentTrackId={overrides.currentTrackId}
      onOpenAlbum={handlers.onOpenAlbum}
      onOpenArtist={handlers.onOpenArtist}
      onPlayAlbum={handlers.onPlayAlbum}
      onPlayTrack={handlers.onPlayTrack}
    />,
  );
  return handlers;
}

function selectTab(name: string): void {
  fireEvent.click(screen.getByRole('tab', { name }));
}

test('segmented tabs carry live counts derived from whatever catalogue arrives', () => {
  const library = demoLibrary();
  renderLibrary(library);
  const counts = [...document.querySelectorAll('#library-tabs [data-tab-count="1"]')].map((node) => node.textContent);
  const trackTotal = library.albums.reduce((total, album) => total + album.tracks.length, 0);
  expect(counts).toStrictEqual([`${library.albums.length}`, `${library.artists.length}`, `${trackTotal}`]);
  // Counts are visual chrome; the accessible tab names stay the bare labels.
  expect(screen.getByRole('tab', { name: 'Albums' })).not.toBeNull();
  expect(screen.getByRole('tab', { name: 'Artists' })).not.toBeNull();
  expect(screen.getByRole('tab', { name: 'Tracks' })).not.toBeNull();
  expect(screen.getByRole('tab', { name: 'Albums' }).getAttribute('data-selected')).toStrictEqual('1');
});

test('tabs switch by pointer and by keyboard', () => {
  renderLibrary();
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Artists' }), { key: 'Enter' });
  expect(document.querySelector('#library-artist-list')).not.toBeNull();
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Tracks' }), { key: ' ' });
  expect(document.querySelector('#library-track-list')).not.toBeNull();
  fireEvent.click(screen.getByRole('tab', { name: 'Albums' }));
  expect(document.querySelector('#library-album-grid')).not.toBeNull();
});

test('album grid renders every album of any library size with safe hostile labels', () => {
  const library = demoLibrary();
  renderLibrary(library);
  const tiles = document.querySelectorAll('[data-album-tile]');
  expect(tiles.length).toStrictEqual(library.albums.length);
  expect(document.querySelector('#album-tile-demo-album-09')).not.toBeNull();
  const hostileTile = document.querySelector('#album-tile-demo-album-08');
  expect(hostileTile?.getAttribute('data-hostile')).toStrictEqual('1');
  expect(hostileTile?.textContent).toContain('Hostile metadata (fixture)');
  expect(document.body.textContent).not.toContain(hostileCorpus());

  // A different catalogue size renders exactly that many tiles — never a
  // hard-coded count.
  cleanup();
  const onOpenAlbum = vi.fn();
  render(
    <Library
      messages={destinationMessages()}
      library={{ albums: library.albums.slice(0, 2), artists: [] }}
      onOpenAlbum={onOpenAlbum}
      onOpenArtist={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  expect(document.querySelectorAll('[data-album-tile]').length).toStrictEqual(2);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(onOpenAlbum).toHaveBeenCalledWith('demo-album-01');
});

test('artist rows show photo or initial, the album count, and open the artist', () => {
  const library = demoLibrary();
  const handlers = renderLibrary(library);
  selectTab('Artists');
  const rows = document.querySelectorAll('[data-artist-row]');
  expect(rows.length).toStrictEqual(library.artists.length);

  // Artists with images get the same-origin photo; the hostile fixture
  // artist deliberately has none and falls back to the initial glyph.
  expect(document.querySelectorAll('[data-artist-photo="1"]').length).toBeGreaterThan(0);
  const hostileRow = document.querySelector('#artist-row-hostile-artist');
  expect(hostileRow?.getAttribute('data-artist-row')).toStrictEqual('hostile-artist');
  expect(hostileRow?.textContent).toContain('Security corpus');
  expect(hostileRow?.querySelector('[data-artist-initial="1"]')?.textContent).toStrictEqual('S');
  expect(document.body.textContent).not.toContain(hostileCorpus());

  const mira = library.artists.find((artist) => artist.key === 'mira-sol');
  if (mira === undefined) {
    throw new Error('fixture artist mira-sol missing');
  }
  const miraRow = document.querySelector('#artist-row-mira-sol');
  expect(miraRow?.textContent).toContain(`${mira.albumIds.length} albums`);
  fireEvent.click(screen.getByRole('button', { name: 'Mira Sol' }));
  expect(handlers.onOpenArtist).toHaveBeenCalledWith('mira-sol');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Keratin' }), { key: 'Enter' });
  expect(handlers.onOpenArtist).toHaveBeenCalledWith('keratin');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Mira Sol' }), { key: ' ' });
  expect(handlers.onOpenArtist).toHaveBeenCalledTimes(3);
});

test('track table lists every track with its album, header chrome and honest flags', () => {
  const library = demoLibrary();
  renderLibrary(library);
  selectTab('Tracks');
  const rows = document.querySelectorAll('[data-track-row]');
  const trackTotal = library.albums.reduce((total, album) => total + album.tracks.length, 0);
  expect(rows.length).toStrictEqual(trackTotal);

  // Header chrome carries the column labels; RN-web strips aria-hidden, so
  // the assertion pins the exact label set instead.
  const head = document.querySelector('[data-track-table-head="1"]');
  expect(head?.textContent).toStrictEqual('#TitleAlbumTime');

  // Every row shows its album name; durations are the formatted tabular text.
  const harbour = library.albums[0];
  if (harbour === undefined) {
    throw new Error('fixture album missing');
  }
  const firstRow = document.querySelector('#track-row-demo-track-01-01');
  expect(firstRow?.querySelector('[data-track-album="1"]')?.textContent).toStrictEqual(harbour.title);
  expect(firstRow?.querySelector('[data-track-duration="1"]')?.textContent).toStrictEqual('3:34');

  // Flag badges exist and are exact; every other row is unflagged.
  const unplayable = document.querySelector('#track-row-demo-track-07-02');
  const damaged = document.querySelector('#track-row-demo-track-07-03');
  const healthy = document.querySelector('#track-row-demo-track-07-01');
  expect(unplayable?.getAttribute('data-flagged')).toStrictEqual('1');
  expect(unplayable?.querySelector('[data-track-flag]')?.textContent).toStrictEqual('Cannot play');
  expect(damaged?.getAttribute('data-flagged')).toStrictEqual('1');
  expect(damaged?.querySelector('[data-track-flag]')?.textContent).toStrictEqual('Damaged');
  expect(healthy?.getAttribute('data-flagged')).toStrictEqual('0');
  expect(healthy?.querySelector('[data-track-flag]')).toBeNull();
});

test('hostile album tracks render safe catalogue labels only', () => {
  renderLibrary();
  selectTab('Tracks');
  const hostileRow = document.querySelector('#track-row-demo-track-08-01');
  expect(hostileRow?.getAttribute('data-hostile')).toStrictEqual('1');
  expect(hostileRow?.querySelector('[data-track-title="1"]')?.textContent).toStrictEqual('Hostile metadata (fixture)');
  expect(hostileRow?.querySelector('[data-track-artist="1"]')?.textContent).toStrictEqual('Security corpus');
  expect(hostileRow?.querySelector('[data-track-album="1"]')?.textContent).toStrictEqual('Hostile metadata (fixture)');
  // The safe label is also the accessible name — the corpus reaches no tree.
  // Both fixture rows share it, so there are exactly two.
  expect(screen.getAllByRole('button', { name: 'Hostile metadata (fixture)' }).length).toStrictEqual(2);
  expect(document.body.textContent).not.toContain(hostileCorpus());
});

test('the playing row paints the current state and playing a row reports album and track', () => {
  const handlers = renderLibrary(demoLibrary(), { currentTrackId: 'demo-track-02-02' });
  selectTab('Tracks');
  const current = document.querySelector('#track-row-demo-track-02-02');
  const other = document.querySelector('#track-row-demo-track-02-01');
  expect(current?.getAttribute('data-current')).toStrictEqual('1');
  expect(current?.querySelector('[data-now-playing="1"]')).not.toBeNull();
  expect(other?.getAttribute('data-current')).toStrictEqual('0');
  expect(other?.querySelector('[data-now-playing="1"]')).toBeNull();

  fireEvent.click(screen.getByRole('button', { name: 'Clock In' }));
  expect(handlers.onPlayTrack).toHaveBeenCalledWith('demo-album-02', 'demo-track-02-01');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Freight Elevator' }), { key: 'Enter' });
  expect(handlers.onPlayTrack).toHaveBeenCalledWith('demo-album-02', 'demo-track-02-02');
});

test('an empty library renders the tabs at zero and no crash', () => {
  renderLibrary({ albums: [], artists: [] });
  const counts = [...document.querySelectorAll('#library-tabs [data-tab-count="1"]')].map((node) => node.textContent);
  expect(counts).toStrictEqual(['0', '0', '0']);
  expect(document.querySelector('#library-album-grid')?.children.length).toStrictEqual(0);
});

test('a hostile-only library stays usable and never renders corpus text', () => {
  const base = demoLibrary();
  const hostileAlbum = base.albums.find((album) => album.hostile);
  if (hostileAlbum === undefined) {
    throw new Error('fixture hostile album missing');
  }
  renderLibrary({
    albums: [hostileAlbum],
    artists: [{ key: hostileAlbum.artistKey, name: hostileAlbum.artistName, albumIds: [hostileAlbum.id] }],
  });
  expect(document.querySelectorAll('[data-album-tile]').length).toStrictEqual(1);
  selectTab('Tracks');
  expect(document.querySelectorAll('[data-track-row]').length).toStrictEqual(hostileAlbum.tracks.length);
  expect(document.body.textContent).toContain('Hostile metadata (fixture)');
  expect(document.body.textContent).not.toContain(hostileCorpus());
});

test('area-library.css stays on tokens: no raw colours, no pills, no translucency tricks', async () => {
  const css = await readFile(join(here, '../../../../../apps/demo/public/area-library.css'), 'utf8');
  // Every colour is a --gm token (possibly inside a color-mix).
  expect(/#[0-9a-fA-F]{3,8}\b/.test(css)).toStrictEqual(false);
  expect(css.includes('var(--gm-')).toStrictEqual(true);
  // Machined radii only — never pills, never blur.
  expect(css.includes('999px')).toStrictEqual(false);
  expect(css.includes('backdrop-filter')).toStrictEqual(false);
  // Motion uses the token, and reduced motion resolves it to instant.
  expect(css.includes('var(--gm-motion)')).toStrictEqual(true);
  expect(css.includes('prefers-reduced-motion: reduce')).toStrictEqual(true);
  // Sticky tab bar under the content top edge with the brass indicator.
  expect(css.includes('#library-tabs')).toStrictEqual(true);
  expect(css.includes('position: sticky')).toStrictEqual(true);
  expect(css.includes('var(--gm-accent-indicator)')).toStrictEqual(true);
  // Density contract: 44px tab and kebab targets, 52px rows, hover wash.
  expect(css.includes('min-height: 44px')).toStrictEqual(true);
  expect(css.includes('min-height: 52px')).toStrictEqual(true);
  expect(css.includes('44px')).toStrictEqual(true);
  expect(css.includes('color-mix(in srgb, var(--gm-text-primary) 6%, transparent)')).toStrictEqual(true);
  // Fluid grid.
  expect(css.includes('minmax(164px, 1fr)')).toStrictEqual(true);
});
