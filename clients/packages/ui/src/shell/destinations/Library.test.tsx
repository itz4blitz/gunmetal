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

test('the header totals line re-derives albums · artists · tracks from any catalogue', () => {
  const library = demoLibrary();
  renderLibrary(library);
  const trackTotal = library.albums.reduce((total, album) => total + album.tracks.length, 0);
  expect(document.querySelector('#library-totals')?.textContent).toStrictEqual(
    `${library.albums.length} albums · ${library.artists.length} artists · ${trackTotal} tracks`,
  );
  // A different catalogue size reports exactly that — never a pinned figure.
  cleanup();
  const shrunk = { albums: library.albums.slice(0, 3), artists: library.artists.slice(0, 2) };
  const shrunkTracks = shrunk.albums.reduce((total, album) => total + album.tracks.length, 0);
  renderLibrary(shrunk);
  expect(document.querySelector('#library-totals')?.textContent).toStrictEqual(
    `3 albums · 2 artists · ${shrunkTracks} tracks`,
  );
});

test('tabs follow a roving tabindex: arrows move focus and selection, wrapping at the ends', () => {
  renderLibrary();
  const albums = screen.getByRole('tab', { name: 'Albums' });
  const artists = screen.getByRole('tab', { name: 'Artists' });
  const tracks = screen.getByRole('tab', { name: 'Tracks' });
  // The selected tab is the only tab stop.
  expect(albums.getAttribute('tabindex')).toStrictEqual('0');
  expect(artists.getAttribute('tabindex')).toStrictEqual('-1');
  expect(tracks.getAttribute('tabindex')).toStrictEqual('-1');

  fireEvent.keyDown(albums, { key: 'ArrowRight' });
  expect(document.activeElement).toStrictEqual(artists);
  expect(artists.getAttribute('data-selected')).toStrictEqual('1');
  expect(albums.getAttribute('data-selected')).toStrictEqual('0');
  expect(document.querySelector('#library-artist-list')).not.toBeNull();

  fireEvent.keyDown(artists, { key: 'ArrowRight' });
  expect(document.activeElement).toStrictEqual(tracks);
  expect(document.querySelector('#library-track-list')).not.toBeNull();

  // Wraps forward and backward; Home and End jump to the ends.
  fireEvent.keyDown(tracks, { key: 'ArrowRight' });
  expect(document.activeElement).toStrictEqual(albums);
  fireEvent.keyDown(albums, { key: 'ArrowLeft' });
  expect(document.activeElement).toStrictEqual(tracks);
  fireEvent.keyDown(tracks, { key: 'Home' });
  expect(document.activeElement).toStrictEqual(albums);
  fireEvent.keyDown(albums, { key: 'End' });
  expect(document.activeElement).toStrictEqual(tracks);
  // Other keys are left alone.
  fireEvent.keyDown(tracks, { key: 'ArrowDown' });
  expect(document.activeElement).toStrictEqual(tracks);
});

test('the density toggle switches comfortable and compact locally and survives tab switches', () => {
  renderLibrary();
  const root = document.querySelector('#destination-library');
  expect(root?.getAttribute('data-density')).toStrictEqual('comfortable');
  fireEvent.click(screen.getByRole('button', { name: 'Compact' }));
  expect(root?.getAttribute('data-density')).toStrictEqual('compact');
  // The chosen density persists across tab switches — local state, no reset.
  fireEvent.click(screen.getByRole('tab', { name: 'Tracks' }));
  expect(root?.getAttribute('data-density')).toStrictEqual('compact');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Comfortable' }), { key: 'Enter' });
  expect(root?.getAttribute('data-density')).toStrictEqual('comfortable');
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

  // Rows enter with a capped stagger slot for the choreography.
  expect(rows[0]?.getAttribute('data-row-stagger')).toStrictEqual('0');
  expect(rows[3]?.getAttribute('data-row-stagger')).toStrictEqual('3');

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

test('the artist row play affordance plays the first album without opening the row', () => {
  const library = demoLibrary();
  const handlers = renderLibrary(library);
  selectTab('Artists');
  const mira = library.artists.find((artist) => artist.key === 'mira-sol');
  if (mira === undefined) {
    throw new Error('fixture artist mira-sol missing');
  }
  const firstAlbum = mira.albumIds[0];
  if (firstAlbum === undefined) {
    throw new Error('fixture artist mira-sol has no albums');
  }
  const play = screen.getByRole('button', { name: `Play ${mira.name}` });
  fireEvent.click(play);
  expect(handlers.onPlayAlbum).toHaveBeenCalledTimes(1);
  expect(handlers.onPlayAlbum).toHaveBeenCalledWith(firstAlbum);
  // The row stays closed: the affordance stops propagation.
  expect(handlers.onOpenArtist).not.toHaveBeenCalled();
  fireEvent.keyDown(play, { key: 'Enter' });
  expect(handlers.onPlayAlbum).toHaveBeenCalledTimes(2);
  // Other keys do nothing.
  fireEvent.keyDown(play, { key: 'Escape' });
  expect(handlers.onPlayAlbum).toHaveBeenCalledTimes(2);
});

test('an artist with no albums offers no play affordance — nothing dishonest', () => {
  renderLibrary({
    albums: [],
    artists: [{ key: 'lonely', name: 'Lonely Artist', albumIds: [] }],
  });
  selectTab('Artists');
  expect(document.querySelectorAll('[data-artist-row]').length).toStrictEqual(1);
  expect(document.querySelector('[data-artist-play="1"]')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Play Lonely Artist' })).toBeNull();
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

  // Entrance stagger: the first row starts at slot 0 and any row's slot is
  // its capped list position — fluid for whatever catalogue arrives.
  const allRows = [...document.querySelectorAll('[data-track-row]')];
  expect(firstRow?.getAttribute('data-row-stagger')).toStrictEqual('0');
  if (damaged === null) {
    throw new Error('fixture damaged row missing');
  }
  const expectedSlot = `${Math.min(Math.max(allRows.indexOf(damaged), 0), 6)}`;
  expect(damaged.getAttribute('data-row-stagger')).toStrictEqual(expectedSlot);
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
  // Sticky tab bar with the brass indicator; the track header sticks under it.
  expect(css.includes('#library-tabs')).toStrictEqual(true);
  expect(css.includes('position: sticky')).toStrictEqual(true);
  expect(css.includes('var(--gm-accent-indicator)')).toStrictEqual(true);
  expect(css.includes('[data-track-table-head]')).toStrictEqual(true);
  expect(css.includes('top: 49px')).toStrictEqual(true);
  // Density contract: 44px tab and kebab targets, 52px comfortable rows, and
  // the compact toggle tightening rows to 44px.
  expect(css.includes('min-height: 44px')).toStrictEqual(true);
  expect(css.includes('min-height: 52px')).toStrictEqual(true);
  expect(css.includes("data-density='compact'] [data-track-row]")).toStrictEqual(true);
  expect(css.includes('44px')).toStrictEqual(true);
  expect(css.includes('color-mix(in srgb, var(--gm-text-primary) 6%, transparent)')).toStrictEqual(true);
  // Fluid grid sweeping 150–180px with the window.
  expect(css.includes('minmax(clamp(150px, 18vw, 180px), 1fr)')).toStrictEqual(true);
  // The 2026 additions: header totals, density control, artist play hex,
  // on-art control scrim, row entrance stagger, search clear affordance.
  expect(css.includes('#library-totals')).toStrictEqual(true);
  expect(css.includes('[data-density-option]')).toStrictEqual(true);
  expect(css.includes('[data-artist-play]')).toStrictEqual(true);
  expect(css.includes('[data-album-art]::after')).toStrictEqual(true);
  expect(css.includes('gm-row-enter')).toStrictEqual(true);
  expect(css.includes('[data-search-clear]')).toStrictEqual(true);
  // Machined focus on the search field warms toward the focus ring token.
  expect(css.includes('#search-field:focus')).toStrictEqual(true);
  expect(css.includes('var(--gm-focus-ring)')).toStrictEqual(true);
  // Reduced motion resolves every new entrance and transition to instant.
  const reduced = css.slice(css.indexOf('prefers-reduced-motion: reduce'));
  expect(reduced.includes('animation: none')).toStrictEqual(true);
  expect(reduced.includes('[data-artist-play]')).toStrictEqual(true);
});
