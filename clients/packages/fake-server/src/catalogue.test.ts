import { expect, test } from 'vitest';
import { albumById, allTracks, demoLibrary, trackById } from './catalogue.ts';
import { coverDataUri } from './cover.ts';
import { demoLocalFilter } from './filter.ts';
import { hostileCorpus } from './hostile.ts';

test('the demo library holds nine fixture albums with unique opaque ids', () => {
  const library = demoLibrary();
  expect(library.kind).toStrictEqual('demo-fixtures');
  expect(library.albums.map((album) => album.id)).toStrictEqual([
    'demo-album-01',
    'demo-album-02',
    'demo-album-03',
    'demo-album-04',
    'demo-album-05',
    'demo-album-06',
    'demo-album-07',
    'demo-album-08',
    'demo-album-09',
  ]);
  const ids = new Set(library.albums.map((album) => album.id));
  expect(ids.size).toStrictEqual(9);
  const trackIds = allTracks(library).map((track) => track.id);
  expect(new Set(trackIds).size).toStrictEqual(trackIds.length);
});

test('fixture rows exercise multi-disc, compilation, same-name artists and flags', () => {
  const library = demoLibrary();
  const stages = albumById(library, 'demo-album-05');
  expect(stages?.discs).toStrictEqual([
    { index: 1, title: 'Act One' },
    { index: 2, title: 'Act Two' },
  ]);
  expect(stages?.tracks.map((track) => track.discIndex)).toStrictEqual([1, 1, 2, 2]);

  const compilation = albumById(library, 'demo-album-06');
  expect(compilation?.artistName).toStrictEqual('Various Artists');
  expect(compilation?.tracks.map((track) => track.artistName)).toStrictEqual([
    'Ivy North',
    'Jun Park',
    'Mira Sol',
    'Keratin',
  ]);

  const alexKeys = library.artists.filter((artist) => artist.name === 'Alex Reed').map((a) => a.key);
  expect(alexKeys).toStrictEqual(['alex-reed-north', 'alex-reed-south']);

  const signal = albumById(library, 'demo-album-07');
  expect(trackById(library, 'demo-track-07-02')?.flag).toStrictEqual('unplayable');
  expect(trackById(library, 'demo-track-07-03')?.flag).toStrictEqual('damaged');
  expect(trackById(library, 'demo-track-02-02')?.lyricsKind).toStrictEqual('synced');
  expect(trackById(library, 'demo-track-01-03')?.lyricsKind).toStrictEqual('plain');
  expect(signal?.title).toStrictEqual('Signal Loss');
});

// Verifies: SEC-CLI-001, SEC-API-046
test('the hostile album holds the corpus payload in every text field', () => {
  const payload = hostileCorpus();
  const album = albumById(demoLibrary(), 'demo-album-08');
  expect(album?.hostile).toStrictEqual(true);
  expect(album?.title).toStrictEqual(payload);
  expect(album?.artistName).toStrictEqual(payload);
  expect(album?.discs[0]?.title).toStrictEqual(payload);
  expect(album?.tracks[0]?.title).toStrictEqual(payload);
  expect(album?.tracks[0]?.artistName).toStrictEqual(payload);
  expect(album?.tracks[1]?.title).toStrictEqual(payload);
});

test('cover data URIs are same-origin SVG placeholders without network URLs', () => {
  const uri = coverDataUri('01', 'Harbour');
  expect(uri.startsWith('data:image/svg+xml;utf8,')).toStrictEqual(true);
  expect(uri.includes('http://') || uri.includes('https://')).toStrictEqual(false);
  expect(decodeURIComponent(uri).includes('#3a5a6e')).toStrictEqual(true);
  expect(
    coverDataUri('02', 'A').includes(encodeURIComponent('#5c4a3a')) ||
      decodeURIComponent(coverDataUri('02', 'A')).includes('#5c4a3a'),
  ).toStrictEqual(true);
  expect(decodeURIComponent(coverDataUri('03', 'A')).includes('#2f4f4f')).toStrictEqual(true);
  expect(decodeURIComponent(coverDataUri('04', 'A')).includes('#4a3f5c')).toStrictEqual(true);
  expect(decodeURIComponent(coverDataUri('05', 'A')).includes('#3f4a32')).toStrictEqual(true);
  expect(decodeURIComponent(coverDataUri('06', 'A')).includes('#5a3a3a')).toStrictEqual(true);
  expect(decodeURIComponent(coverDataUri('07', 'A')).includes('#2a3a4a')).toStrictEqual(true);
  expect(decodeURIComponent(coverDataUri('08', 'A')).includes('#1f262d')).toStrictEqual(true);
  expect(decodeURIComponent(coverDataUri('01', 'a&b<c>"d\'e')).includes('&amp;')).toStrictEqual(true);
});

test('demo-local filter matches substring on titles and artists and ignores blanks', () => {
  const library = demoLibrary();
  expect(demoLocalFilter(library, '   ')).toStrictEqual({ albums: [], tracks: [] });
  const harbour = demoLocalFilter(library, 'harbour');
  expect(harbour.albums.map((album) => album.id)).toStrictEqual(['demo-album-01']);
  expect(harbour.tracks.map((track) => track.id)).toStrictEqual([]);
  const mira = demoLocalFilter(library, 'Mira');
  expect(mira.albums.map((album) => album.id)).toStrictEqual(['demo-album-01', 'demo-album-02']);
  expect(demoLocalFilter(library, 'zabriskie').albums.map((album) => album.id)).toStrictEqual(['demo-album-09']);
  expect(demoLocalFilter(library, 'cc-by-4.0').albums.map((album) => album.id)).toStrictEqual(['demo-album-09']);
  expect(demoLocalFilter(library, 'Cylinders by Chris').albums.map((album) => album.id)).toStrictEqual([
    'demo-album-09',
  ]);
  expect(demoLocalFilter(library, 'chriszabriskie.com').albums.map((album) => album.id)).toStrictEqual([
    'demo-album-09',
  ]);
  expect(library.artists.some((artist) => artist.key === 'chris-zabriskie')).toStrictEqual(true);
  expect(mira.tracks.some((track) => track.id === 'demo-track-06-03')).toStrictEqual(true);
  expect(albumById(library, 'missing')).toStrictEqual(undefined);
  expect(trackById(library, 'missing')).toStrictEqual(undefined);
});
