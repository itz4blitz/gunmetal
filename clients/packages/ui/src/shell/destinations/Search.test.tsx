import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { demoLocalFilter } from '../../../../fake-server/src/filter.ts';
import { hostileCorpus } from '../../../../fake-server/src/hostile.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { LibrarySearch } from '../content.ts';
import type { ShellLibrary } from '../library-types.ts';
import { Search } from './Search.tsx';

afterEach(cleanup);

const here = dirname(fileURLToPath(import.meta.url));

function renderSearch() {
  const library = demoLibrary();
  const rendered = render(
    <Search
      searchLibrary={demoLocalFilter}
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  return { library, rendered };
}

type WiredHandlers = {
  onOpenAlbum: (albumId: string) => void;
  onOpenArtist: (artistKey: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
};

/** The surface as the shell mounts it: every handler wired, artists openable. */
function renderWired(
  library: ShellLibrary = demoLibrary(),
  searchLibrary: LibrarySearch = demoLocalFilter,
): WiredHandlers {
  const handlers: WiredHandlers = {
    onOpenAlbum: vi.fn(),
    onOpenArtist: vi.fn(),
    onPlayAlbum: vi.fn(),
    onPlayTrack: vi.fn(),
  };
  render(
    <Search
      searchLibrary={searchLibrary}
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={handlers.onOpenAlbum}
      onOpenArtist={handlers.onOpenArtist}
      onPlayAlbum={handlers.onPlayAlbum}
      onPlayTrack={handlers.onPlayTrack}
    />,
  );
  return handlers;
}

function typeQuery(value: string): void {
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), { target: { value } });
}

test('local lookup finds Cylinders by artist, SPDX, attribution, source and track title', () => {
  const { library } = renderSearch();
  typeQuery('zabriskie');
  expect(document.querySelector('#album-tile-demo-album-09')).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Cylinders' })).not.toBeNull();
  expect(document.querySelector('#search-plugin-notice')?.textContent).toStrictEqual(
    'A signed catalogue plugin can find releases outside this library. None is loaded.',
  );

  typeQuery('cc-by-4.0');
  expect(document.querySelector('#album-tile-demo-album-09')).not.toBeNull();
  // The fixture set grows; the licence lookup stays exact: the grid shows
  // exactly what the demo-local filter answers, and every hit is CC-BY-4.0.
  const ccTiles = [...document.querySelectorAll('[data-album-tile]')];
  expect(ccTiles.length).toStrictEqual(demoLocalFilter(library, 'cc-by-4.0').albums.length);
  expect(ccTiles.length).toBeGreaterThan(0);
  for (const tile of ccTiles) {
    const id = tile.getAttribute('data-album-tile') ?? '';
    const album = library.albums.find((entry) => entry.id === id);
    expect(album?.license?.spdx).toStrictEqual('CC-BY-4.0');
  }

  typeQuery('Cylinders by Chris Zabriskie');
  expect(document.querySelector('#album-tile-demo-album-09')).not.toBeNull();

  typeQuery('chriszabriskie.com');
  expect(document.querySelector('#album-tile-demo-album-09')).not.toBeNull();

  typeQuery('Cylinder Seven');
  expect(document.querySelector('#track-row-demo-track-09-07')).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Cylinder Seven' })).not.toBeNull();
});

// Verifies: SEC-HIS-024, SEC-API-044
test('search lookup stays on the local fixture set and does not fetch a third party', async () => {
  renderSearch();
  typeQuery('Cylinders');
  expect(document.querySelector('#search-demo-notice')?.textContent).toStrictEqual(
    'Demo-local filter — not CorePort search',
  );
  expect(document.querySelector('#search-plugin-notice')?.textContent).toStrictEqual(
    'A signed catalogue plugin can find releases outside this library. None is loaded.',
  );
  const source = await readFile(join(here, 'Search.tsx'), 'utf8');
  expect(source.includes('fetch(')).toStrictEqual(false);
  expect(source.includes('https://')).toStrictEqual(false);
  expect(source.includes('jamendo')).toStrictEqual(false);
  expect(source.includes('archive.org')).toStrictEqual(false);
  expect(source.includes('WebAssembly')).toStrictEqual(false);
});

test('an empty query is a calm hint: no results chrome, no filters, no always-empty recents card', () => {
  renderSearch();
  expect(document.querySelector('#search-idle #search-hint')?.textContent).toStrictEqual(
    'Search by album, track or artist name.',
  );
  expect(document.querySelector('#search-results')).toBeNull();
  expect(document.querySelector('#search-results-count')).toBeNull();
  // The type filter belongs to results; before a query it has nothing to filter.
  expect(document.querySelector('#search-type-chips')).toBeNull();
  // Nothing records searches in this build, so no recents block is drawn.
  expect(document.querySelector('#search-recent')).toBeNull();
  expect(document.body.textContent).not.toContain('No recent searches');
  // The loupe is a real icon, not a drawn pseudo-element.
  expect(document.querySelector('#search-affordance svg')?.getAttribute('data-icon')).toStrictEqual('search');
  // Without a way to open an artist, no browse tiles are offered.
  expect(document.querySelector('#search-browse')).toBeNull();
});

test('before a query the page offers browse-by-artist tiles built from the library it was handed', () => {
  const library = demoLibrary();
  const handlers = renderWired(library);
  expect(screen.getByRole('heading', { name: 'Browse artists' }).getAttribute('data-search-group')).toStrictEqual(
    'artists',
  );
  expect(document.querySelector('#search-browse [data-search-group-count]')?.textContent).toStrictEqual(
    `${library.artists.length}`,
  );
  const tiles = [...document.querySelectorAll('#search-browse [data-search-artist]')];
  expect(tiles.map((tile) => tile.getAttribute('data-search-artist'))).toStrictEqual(
    library.artists.map((artist) => artist.key),
  );
  // Every tile is a keyboard-reachable button named by the artist.
  expect(tiles.map((tile) => [tile.getAttribute('role'), tile.getAttribute('tabindex')])).toStrictEqual(
    library.artists.map(() => ['button', '0']),
  );
  const mira = document.querySelector('[data-search-artist="mira-sol"]');
  expect(mira?.getAttribute('aria-label')).toStrictEqual('Mira Sol');
  expect(mira?.querySelector('[data-search-artist-name]')?.textContent).toStrictEqual('Mira Sol');
  expect(mira?.querySelector('[data-artist-photo="1"]')).not.toBeNull();
  // The hostile fixture artist keeps its safe label and has no photo.
  const hostile = document.querySelector('[data-search-artist="hostile-artist"]');
  expect(hostile?.getAttribute('aria-label')).toStrictEqual('Security corpus');
  expect(hostile?.querySelector('[data-search-artist-name]')?.textContent).toStrictEqual('Security corpus');
  expect(hostile?.querySelector('[data-artist-initial="1"]')?.textContent).toStrictEqual('S');
  expect(hostile?.querySelector('[data-artist-photo="1"]')).toBeNull();
  expect(document.body.textContent).not.toContain(hostileCorpus());

  fireEvent.click(screen.getByRole('button', { name: 'Mira Sol' }));
  expect(handlers.onOpenArtist).toHaveBeenCalledWith('mira-sol');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Keratin' }), { key: 'Enter' });
  expect(handlers.onOpenArtist).toHaveBeenCalledWith('keratin');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Keratin' }), { key: ' ' });
  expect(handlers.onOpenArtist).toHaveBeenCalledTimes(3);
  // Other keys do nothing.
  fireEvent.keyDown(screen.getByRole('button', { name: 'Keratin' }), { key: 'Tab' });
  expect(handlers.onOpenArtist).toHaveBeenCalledTimes(3);

  // Typing replaces the browse tiles with results.
  typeQuery('Cylinder');
  expect(document.querySelector('#search-browse')).toBeNull();
  expect(document.querySelector('#search-idle')).toBeNull();
});

test('a library with no artists offers the hint alone — nothing is invented to browse', () => {
  renderWired({ albums: [], artists: [] });
  expect(document.querySelector('#search-hint')?.textContent).toStrictEqual('Search by album, track or artist name.');
  expect(document.querySelector('#search-browse')).toBeNull();
  expect(document.querySelectorAll('[data-search-artist]').length).toStrictEqual(0);
});

test('the results meta line counts what is shown, per group, from the filter', () => {
  const library = demoLibrary();
  renderSearch();
  typeQuery('Cylinder');
  const hits = demoLocalFilter(library, 'Cylinder');
  const expectedTotal = hits.albums.length + hits.tracks.length;
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual(`${expectedTotal} results`);
  expect(document.querySelector('[data-search-group="albums"]')?.textContent).toStrictEqual('Albums');
  expect(document.querySelector('[data-search-group="tracks"]')?.textContent).toStrictEqual('Tracks');
  const groupCounts = [...document.querySelectorAll('#search-results [data-search-group-count="1"]')].map(
    (node) => node.textContent,
  );
  expect(groupCounts).toStrictEqual([`${hits.albums.length}`, `${hits.tracks.length}`]);
  expect(document.querySelector('[data-search-query-echo="1"]')).toBeNull();
  // Sections arrive in one order: top result, albums, tracks, then the notices.
  expect(
    [...(document.querySelector('#search-results')?.children ?? [])].map(
      (node) =>
        node.id ||
        (node.hasAttribute('data-search-albums') ? 'albums' : node.hasAttribute('data-search-tracks') ? 'tracks' : ''),
    ),
  ).toStrictEqual(['search-results-bar', 'search-top', 'albums', 'tracks', 'search-notices']);
});

test('a single match is counted in the singular', () => {
  const library = demoLibrary();
  const harbour = library.albums[0];
  if (harbour === undefined) {
    throw new Error('fixture album missing');
  }
  renderWired(library, () => ({ albums: [harbour], tracks: [] }));
  typeQuery('anything');
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual('1 result');
});

test('the top result is the first album hit: it opens the album and its nut plays it', () => {
  const handlers = renderWired();
  typeQuery('Cylinders');
  const top = document.querySelector('#search-top');
  expect(top?.getAttribute('data-search-top')).toStrictEqual('album');
  expect(screen.getByRole('heading', { name: 'Top result' }).getAttribute('data-search-group')).toStrictEqual('top');
  expect(top?.querySelector('[data-search-top-title]')?.textContent).toStrictEqual('Cylinders');
  expect(top?.querySelector('[data-search-top-byline]')?.textContent).toStrictEqual('Album · Chris Zabriskie');
  expect(top?.querySelector('#search-top-cover')?.getAttribute('aria-label')).toStrictEqual('Cylinders');

  const open = screen.getByRole('button', { name: 'Top result: Cylinders' });
  fireEvent.click(open);
  expect(handlers.onOpenAlbum).toHaveBeenCalledWith('demo-album-09');
  fireEvent.keyDown(open, { key: 'Enter' });
  expect(handlers.onOpenAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(open, { key: 'Escape' });
  expect(handlers.onOpenAlbum).toHaveBeenCalledTimes(2);

  // The play control is the shared brass nut inside its focus plate.
  const play = screen.getByRole('button', { name: 'Play Cylinders' });
  expect(play.getAttribute('data-brass-hex')).toStrictEqual('1');
  expect(play.parentElement?.getAttribute('data-hex-wrap')).toStrictEqual('1');
  fireEvent.click(play);
  expect(handlers.onPlayAlbum).toHaveBeenCalledWith('demo-album-09');
  fireEvent.keyDown(play, { key: ' ' });
  expect(handlers.onPlayAlbum).toHaveBeenCalledTimes(2);
  expect(handlers.onPlayTrack).not.toHaveBeenCalled();
});

test('with no album hit the top result is the first track: it opens its album and plays the track', () => {
  const handlers = renderWired();
  typeQuery('Cylinder Seven');
  expect(demoLocalFilter(demoLibrary(), 'Cylinder Seven').albums.length).toStrictEqual(0);
  const top = document.querySelector('#search-top');
  expect(top?.getAttribute('data-search-top')).toStrictEqual('track');
  expect(top?.querySelector('[data-search-top-title]')?.textContent).toStrictEqual('Cylinder Seven');
  expect(top?.querySelector('[data-search-top-byline]')?.textContent).toStrictEqual('Track · Chris Zabriskie');
  // The card borrows the cover of the album the track is on.
  expect(top?.querySelector('#search-top-cover')?.getAttribute('data-cover-art')).toStrictEqual('1');

  fireEvent.click(screen.getByRole('button', { name: 'Top result: Cylinder Seven' }));
  expect(handlers.onOpenAlbum).toHaveBeenCalledWith('demo-album-09');
  fireEvent.click(screen.getByRole('button', { name: 'Play Cylinder Seven' }));
  expect(handlers.onPlayTrack).toHaveBeenCalledWith('demo-album-09', 'demo-track-09-07');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play Cylinder Seven' }), { key: 'Enter' });
  expect(handlers.onPlayTrack).toHaveBeenCalledTimes(2);
  expect(handlers.onPlayAlbum).not.toHaveBeenCalled();
});

test('a top result from a hostile album shows the safe labels, as an album and as a track', () => {
  const library = demoLibrary();
  const hostileAlbum = library.albums.find((album) => album.hostile);
  const hostileTrack = hostileAlbum?.tracks[0];
  if (hostileAlbum === undefined || hostileTrack === undefined) {
    throw new Error('fixture hostile album missing');
  }
  renderWired(library, () => ({ albums: [hostileAlbum], tracks: [hostileTrack] }));
  typeQuery('anything');
  expect(document.querySelector('#search-top')?.getAttribute('data-search-top')).toStrictEqual('album');
  expect(document.querySelector('[data-search-top-title]')?.textContent).toStrictEqual('Hostile metadata (fixture)');
  expect(document.querySelector('[data-search-top-byline]')?.textContent).toStrictEqual('Album · Security corpus');
  expect(screen.getByRole('button', { name: 'Top result: Hostile metadata (fixture)' })).not.toBeNull();
  expect(document.body.textContent).not.toContain(hostileCorpus());

  cleanup();
  renderWired(library, () => ({ albums: [], tracks: [hostileTrack] }));
  typeQuery('anything');
  expect(document.querySelector('#search-top')?.getAttribute('data-search-top')).toStrictEqual('track');
  expect(document.querySelector('[data-search-top-title]')?.textContent).toStrictEqual('Hostile metadata (fixture)');
  expect(document.querySelector('[data-search-top-byline]')?.textContent).toStrictEqual('Track · Security corpus');
  expect(document.body.textContent).not.toContain(hostileCorpus());
});

test('a track whose album is not in the library still tops the results, on the plain tone plate', () => {
  const library = demoLibrary();
  const track = library.albums[0]?.tracks[0];
  if (track === undefined) {
    throw new Error('fixture track missing');
  }
  const handlers = renderWired({ albums: [], artists: [] }, () => ({ albums: [], tracks: [track] }));
  typeQuery('pier');
  const top = document.querySelector('#search-top');
  expect(top?.getAttribute('data-search-top')).toStrictEqual('track');
  expect(top?.querySelector('[data-search-top-title]')?.textContent).toStrictEqual('Pier at Dusk');
  expect(top?.querySelector('[data-search-top-byline]')?.textContent).toStrictEqual('Track · Mira Sol');
  const cover = top?.querySelector('#search-top-cover');
  expect(cover?.getAttribute('data-cover')).toStrictEqual('');
  expect(cover?.getAttribute('data-cover-art')).toStrictEqual('0');
  fireEvent.click(screen.getByRole('button', { name: 'Play Pier at Dusk' }));
  expect(handlers.onPlayTrack).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
});

test('a non-empty query offers a clear control that empties the field and refocuses it', () => {
  renderSearch();
  expect(document.querySelector('#search-clear')).toBeNull();

  typeQuery('Cylinder');
  const clear = screen.getByRole('button', { name: 'Clear search' });
  expect(clear.getAttribute('tabindex')).toStrictEqual('0');
  // The control is the close icon; its name lives on the button.
  expect(clear.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('close');
  expect(clear.textContent).toStrictEqual('');

  const field = screen.getByLabelText('Search albums and tracks') as HTMLInputElement;
  fireEvent.click(clear);
  expect(field.value).toStrictEqual('');
  // Clearing lands the person back in the idle state, field still focused.
  expect(document.querySelector('#search-results')).toBeNull();
  expect(document.querySelector('#search-idle #search-hint')?.textContent).toStrictEqual(
    'Search by album, track or artist name.',
  );
  expect(document.activeElement).toStrictEqual(field);

  typeQuery('zzquadrazz');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Clear search' }), { key: 'Enter' });
  expect((screen.getByLabelText('Search albums and tracks') as HTMLInputElement).value).toStrictEqual('');
  expect(document.querySelector('[data-search-query-echo="1"]')).toBeNull();
});

test('type toggles are quiet: each hides its group, and one type always stays on', () => {
  renderSearch();
  typeQuery('Cylinder');
  expect(document.querySelector('[data-search-albums="1"]')).not.toBeNull();
  expect(document.querySelector('[data-search-tracks="1"]')).not.toBeNull();
  // Each toggle says how many hits its type has and whether it is on.
  const all = demoLocalFilter(demoLibrary(), 'Cylinder');
  const chipState = () =>
    [...document.querySelectorAll('#search-type-chips [data-search-chip]')].map((chip) => [
      chip.getAttribute('aria-label'),
      chip.getAttribute('aria-pressed'),
      chip.querySelector('[data-search-chip-count]')?.textContent,
    ]);
  expect(chipState()).toStrictEqual([
    ['Albums', 'true', `${all.albums.length}`],
    ['Tracks', 'true', `${all.tracks.length}`],
  ]);

  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('[data-search-albums="1"]')).toBeNull();
  expect(document.querySelector('[data-search-tracks="1"]')).not.toBeNull();
  // A hidden type keeps its count: the toggle still says what it would show.
  expect(chipState()).toStrictEqual([
    ['Albums', 'false', `${all.albums.length}`],
    ['Tracks', 'true', `${all.tracks.length}`],
  ]);
  const tracksOnly = demoLocalFilter(demoLibrary(), 'Cylinder');
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual(
    `${tracksOnly.tracks.length} results`,
  );
  // With albums hidden, the top result falls to the first track.
  expect(document.querySelector('#search-top')?.getAttribute('data-search-top')).toStrictEqual('track');

  // Turning Albums back on works; with Albums on, Tracks may turn off — but
  // the last-on type refuses, so one type always stays on.
  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('[data-search-albums="1"]')).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('[data-search-albums="1"]')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Tracks' }));
  expect(document.querySelector('[data-search-tracks="1"]')).not.toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Tracks' }), { key: 'Enter' });
  expect(document.querySelector('[data-search-tracks="1"]')).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('[data-search-albums="1"]')).not.toBeNull();
  expect(document.querySelector('[data-search-tracks="1"]')).not.toBeNull();

  // The same holds the other way round: Tracks turns off while Albums is on,
  // and then Albums — the last type on — refuses to turn off.
  fireEvent.click(screen.getByRole('button', { name: 'Tracks' }));
  expect(document.querySelector('[data-search-tracks="1"]')).toBeNull();
  expect(document.querySelector('[data-search-albums="1"]')).not.toBeNull();
  expect(chipState()).toStrictEqual([
    ['Albums', 'true', `${all.albums.length}`],
    ['Tracks', 'false', `${all.tracks.length}`],
  ]);
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual('1 result');
  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('[data-search-albums="1"]')).not.toBeNull();
  expect(chipState()).toStrictEqual([
    ['Albums', 'true', `${all.albums.length}`],
    ['Tracks', 'false', `${all.tracks.length}`],
  ]);
  // Space toggles as Enter does; any other key is left alone.
  fireEvent.keyDown(screen.getByRole('button', { name: 'Tracks' }), { key: ' ' });
  expect(document.querySelector('[data-search-tracks="1"]')).not.toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Tracks' }), { key: 'Tab' });
  expect(document.querySelector('[data-search-tracks="1"]')).not.toBeNull();
});

test('an empty library says so instead of pretending the query missed', () => {
  render(
    <Search
      searchLibrary={demoLocalFilter}
      messages={destinationMessages()}
      library={{ albums: [], artists: [] }}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  typeQuery('anything');
  expect(document.querySelector('#search-empty-library')?.textContent).toStrictEqual(
    'The library is empty — nothing to search yet.',
  );
  // The generic miss would lie about a library that has nothing to miss in.
  expect(document.querySelector('#search-no-hits')).toBeNull();
  expect(document.querySelector('#search-top')).toBeNull();
});

test('hiding the only hit type names the filter instead of a miss', () => {
  const library = demoLibrary();
  const albumsOnly: LibrarySearch = (read, query) => ({
    albums: demoLocalFilter(read, query).albums,
    tracks: [],
  });
  renderWired(library, albumsOnly);
  typeQuery('zabriskie');
  const hits = albumsOnly(library, 'zabriskie');
  expect(hits.albums.length).toBeGreaterThan(0);
  // Both types are on: the album hits render normally.
  expect(document.querySelector('[data-search-albums="1"]')).not.toBeNull();
  // Hide the albums: results exist but none are shown — the filter says so.
  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('#search-filter-empty')).not.toBeNull();
  expect(screen.getByRole('heading', { name: 'No results for the current filter' })).not.toBeNull();
  expect(document.querySelector('[data-search-filter-remaining="1"]')?.textContent).toStrictEqual('Tracks');
  expect(document.querySelector('#search-filter-empty [data-search-filter-hint="1"]')?.textContent).toStrictEqual(
    'Turn a type back on to see its results.',
  );
  // The chips keep their counts and the count line stays honest.
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual('0 results');
  expect(document.querySelector('#search-no-hits')).toBeNull();
  // Turning the type back on restores the results.
  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('#search-filter-empty')).toBeNull();
  expect(document.querySelector('[data-search-albums="1"]')).not.toBeNull();
});

test('hiding the tracks type names Albums as what remains', () => {
  const library = demoLibrary();
  const tracksOnly: LibrarySearch = (read, query) => ({
    albums: [],
    tracks: demoLocalFilter(read, query).tracks,
  });
  renderWired(library, tracksOnly);
  typeQuery('zabriskie');
  expect(demoLocalFilter(library, 'zabriskie').tracks.length).toBeGreaterThan(0);
  fireEvent.click(screen.getByRole('button', { name: 'Tracks' }));
  expect(document.querySelector('#search-filter-empty')).not.toBeNull();
  expect(document.querySelector('[data-search-filter-remaining="1"]')?.textContent).toStrictEqual('Albums');
});

test('no matches say so, echo the query as an isolated text node, and say what to try', () => {
  renderSearch();
  typeQuery('zzquadrazz');
  expect(screen.getByRole('heading', { name: 'No matches for this query in Music' })).not.toBeNull();
  const echo = document.querySelector('[data-search-query-echo="1"]');
  expect(echo?.textContent).toStrictEqual('“zzquadrazz”');
  expect(echo?.getAttribute('dir')).toStrictEqual('auto');
  expect(document.querySelector('#search-no-hits [data-search-empty-hint]')?.textContent).toStrictEqual(
    'Check the spelling, or try a shorter word.',
  );
  expect(
    document.querySelector('#search-no-hits [data-search-empty-mark] svg')?.getAttribute('data-icon'),
  ).toStrictEqual('search');
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual('0 results');
  // No hits means no top result and no groups — only the statement.
  expect(document.querySelector('#search-top')).toBeNull();
  expect(document.querySelector('[data-search-albums]')).toBeNull();
  expect(document.querySelector('[data-search-tracks]')).toBeNull();
});

test('hostile corpus matches stay labelled: no fixture text reaches any tree', () => {
  renderSearch();
  typeQuery('script');
  const hits = demoLocalFilter(demoLibrary(), 'script');
  expect(hits.albums.length).toBeGreaterThan(0);
  expect(hits.tracks.length).toBeGreaterThan(0);
  for (const row of document.querySelectorAll('[data-track-row]')) {
    expect(row.getAttribute('data-hostile')).toStrictEqual('1');
    expect(row.getAttribute('data-flagged')).toStrictEqual('0');
  }
  // Two track rows plus the hostile album tile all carry the safe label.
  expect(screen.getAllByRole('button', { name: 'Hostile metadata (fixture)' }).length).toStrictEqual(
    hits.tracks.length + hits.albums.length,
  );
  expect(document.body.textContent).toContain('Security corpus');
  expect(document.body.textContent).not.toContain(hostileCorpus());
});

test('the playing row is marked inside search results', () => {
  render(
    <Search
      searchLibrary={demoLocalFilter}
      messages={destinationMessages()}
      library={demoLibrary()}
      currentTrackId="demo-track-09-07"
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  typeQuery('Cylinder Seven');
  const row = document.querySelector('#track-row-demo-track-09-07');
  expect(row?.getAttribute('data-current')).toStrictEqual('1');
  expect(row?.querySelector('[data-now-playing="1"]')).not.toBeNull();
  expect(row?.querySelector('[data-track-album="1"]')?.textContent).toStrictEqual('Cylinders');
  fireEvent.click(screen.getByRole('button', { name: 'Cylinder Seven' }));
});

test('without a lookup the surface answers every query with no matches', () => {
  render(
    <Search
      messages={destinationMessages()}
      library={demoLibrary()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  typeQuery('Cylinder');
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual('0 results');
  expect(screen.getByRole('heading', { name: 'No matches for this query in Music' })).not.toBeNull();
});
