import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { demoLocalFilter } from '../../../../fake-server/src/filter.ts';
import { hostileCorpus } from '../../../../fake-server/src/hostile.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
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

test('an empty query shows the recent state and no results chrome', () => {
  renderSearch();
  expect(document.querySelector('#search-recent')?.textContent).toContain('No recent searches');
  expect(document.querySelector('#search-results')).toBeNull();
  expect(document.querySelector('#search-results-count')).toBeNull();
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
  const groupCounts = [...document.querySelectorAll('[data-search-group-count="1"]')].map((node) => node.textContent);
  expect(groupCounts).toStrictEqual([`${hits.albums.length}`, `${hits.tracks.length}`]);
  expect(document.querySelector('[data-search-query-echo="1"]')).toBeNull();
});

test('a non-empty query offers a clear control that empties the field and refocuses it', () => {
  renderSearch();
  expect(document.querySelector('#search-clear')).toBeNull();

  typeQuery('Cylinder');
  const clear = screen.getByRole('button', { name: 'Clear search' });
  expect(clear.getAttribute('tabindex')).toStrictEqual('0');

  const field = screen.getByLabelText('Search albums and tracks') as HTMLInputElement;
  fireEvent.click(clear);
  expect(field.value).toStrictEqual('');
  // Clearing lands the person back in the empty state, field still focused.
  expect(document.querySelector('#search-results')).toBeNull();
  expect(document.querySelector('#search-recent')?.textContent).toContain('No recent searches');
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

  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('[data-search-albums="1"]')).toBeNull();
  expect(document.querySelector('[data-search-tracks="1"]')).not.toBeNull();
  const tracksOnly = demoLocalFilter(demoLibrary(), 'Cylinder');
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual(
    `${tracksOnly.tracks.length} results`,
  );

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
});

test('no matches echo the query as an isolated text node', () => {
  renderSearch();
  typeQuery('zzquadrazz');
  expect(document.querySelector('#search-no-hits')?.textContent).toContain('No matches for this query in Music');
  const echo = document.querySelector('[data-search-query-echo="1"]');
  expect(echo?.textContent).toStrictEqual('“zzquadrazz”');
  expect(echo?.getAttribute('dir')).toStrictEqual('auto');
  expect(document.querySelector('#search-results-count')?.textContent).toStrictEqual('0 results');
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
