import { expect, test } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { albumById, allTracks, demoLibrary, trackById } from './catalogue.ts';
import { coverDataUri } from './cover.ts';
import { demoLocalFilter } from './filter.ts';
import { hostileCorpus } from './hostile.ts';

test('the demo library holds fifteen fixture albums with unique opaque ids', () => {
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
    'demo-album-10',
    'demo-album-11',
    'demo-album-12',
    'demo-album-13',
    'demo-album-14',
    'demo-album-15',
  ]);
  const ids = new Set(library.albums.map((album) => album.id));
  expect(ids.size).toStrictEqual(15);
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
  expect(demoLocalFilter(library, 'zabriskie').albums.map((album) => album.id)).toStrictEqual([
    'demo-album-09',
    'demo-album-10',
  ]);
  expect(demoLocalFilter(library, 'cc-by-4.0').albums.map((album) => album.id)).toStrictEqual([
    'demo-album-09',
    'demo-album-10',
    'demo-album-11',
    'demo-album-12',
    'demo-album-15',
  ]);
  expect(demoLocalFilter(library, 'Cylinders by Chris').albums.map((album) => album.id)).toStrictEqual([
    'demo-album-09',
  ]);
  expect(demoLocalFilter(library, 'chriszabriskie.com').albums.map((album) => album.id)).toStrictEqual([
    'demo-album-09',
    'demo-album-10',
  ]);
  expect(library.artists.some((artist) => artist.key === 'chris-zabriskie')).toStrictEqual(true);
  expect(mira.tracks.some((track) => track.id === 'demo-track-06-03')).toStrictEqual(true);
  expect(albumById(library, 'missing')).toStrictEqual(undefined);
  expect(trackById(library, 'missing')).toStrictEqual(undefined);
});

const newAlbumIds = demoLibrary()
  .albums.map((album) => album.id)
  .filter((id) => id >= 'demo-album-10');

// Verifies the generated media set stays in lockstep with the catalogue:
// every fixture cover and tone must exist on disk, and the generated
// directories must hold exactly the catalogue ids (no strays).
test('every album coverUrl and track mediaUrl resolves to a generated file', () => {
  const library = demoLibrary();
  const mediaRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', 'apps', 'demo', 'public', 'media');
  const coverFiles = new Set(readdirSync(join(mediaRoot, 'covers')));
  const audioFiles = new Set(readdirSync(join(mediaRoot, 'audio')));
  expect([...coverFiles].sort()).toStrictEqual(library.albums.map((album) => `${album.id}.svg`).sort());
  expect([...audioFiles].sort()).toStrictEqual(library.albums.map((album) => `${album.id}.wav`).sort());
  for (const album of library.albums) {
    expect(statSync(join(mediaRoot, 'covers', `${album.id}.svg`)).size).toBeGreaterThan(0);
    expect(album.coverUrl).toStrictEqual(`/media/covers/${album.id}.svg`);
    const audioPath = join(mediaRoot, 'audio', `${album.id}.wav`);
    expect(statSync(audioPath).size).toBeGreaterThan(0);
    // The WAV header carries the RIFF magic; a truncated write would not.
    expect(readFileSync(audioPath).subarray(0, 4).toString('ascii')).toStrictEqual('RIFF');
    for (const track of album.tracks) {
      expect(track.mediaUrl).toStrictEqual(`/media/audio/${album.id}.wav`);
    }
  }
});

test('every non-hostile artist key maps to a generated artist image', () => {
  const library = demoLibrary();
  const mediaRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', 'apps', 'demo', 'public', 'media');
  const artistKeys = library.artists.map((artist) => artist.key);
  expect(artistKeys).toContain('kai-engel');
  expect(artistKeys).toContain('scott-buckley');
  expect(artistKeys).toContain('kevin-macleod');
  for (const artist of library.artists) {
    if (artist.key === 'hostile-artist') {
      expect(artist.imageUrl).toStrictEqual(undefined);
      continue;
    }
    expect(artist.imageUrl).toStrictEqual(`/media/artists/${artist.key}.svg`);
    expect(statSync(join(mediaRoot, 'artists', `${artist.key}.svg`)).size).toBeGreaterThan(0);
  }
});

test('the new real albums carry exactly one unplayable and one damaged flag', () => {
  const library = demoLibrary();
  const newTracks = allTracks({ albums: library.albums.filter((album) => newAlbumIds.includes(album.id)) });
  expect(newTracks.filter((track) => track.flag === 'unplayable').map((track) => track.id)).toStrictEqual([
    'demo-track-13-07',
  ]);
  expect(newTracks.filter((track) => track.flag === 'damaged').map((track) => track.id)).toStrictEqual([
    'demo-track-14-04',
  ]);
  // Dimmed rows must never empty an album: every new album keeps a playable
  // first track so playback totals stay working.
  for (const id of newAlbumIds) {
    const album = albumById(library, id);
    expect(album?.tracks.some((track) => track.flag === 'ok')).toStrictEqual(true);
  }
});

test('every new real album records a licence from the allowed SPDX union', () => {
  const library = demoLibrary();
  const allowedSpdx = new Set(['CC0-1.0', 'CC-BY-3.0', 'CC-BY-4.0', 'CC-BY-SA-3.0']);
  const licences = newAlbumIds.map((id) => {
    const album = albumById(library, id);
    expect(album?.hostile).toStrictEqual(false);
    expect(album?.license).toBeDefined();
    return album!.license!;
  });
  for (const licence of licences) {
    expect(allowedSpdx.has(licence.spdx)).toStrictEqual(true);
    expect(licence.attribution.length).toBeGreaterThan(0);
    expect(licence.source.length).toBeGreaterThan(0);
  }
  expect(licences.map((licence) => licence.spdx)).toStrictEqual([
    'CC-BY-4.0',
    'CC-BY-4.0',
    'CC-BY-4.0',
    'CC-BY-3.0',
    'CC-BY-3.0',
    'CC-BY-4.0',
  ]);
});

test('the new real albums reuse cover tones and exercise multi-disc structure', () => {
  const library = demoLibrary();
  const newAlbums = newAlbumIds.map((id) => albumById(library, id)!);
  const tones = new Set(newAlbums.map((album) => album.coverTone));
  for (const tone of tones) {
    expect(['01', '02', '03', '04', '05', '06', '07', '08'].includes(tone)).toStrictEqual(true);
  }
  const multiDisc = newAlbums.filter((album) => album.discs.length > 1);
  expect(multiDisc.map((album) => album.id)).toStrictEqual(['demo-album-12', 'demo-album-15']);
  expect(albumById(library, 'demo-album-12')?.discs).toStrictEqual([
    { index: 1, title: 'Library Songs 8' },
    { index: 2, title: 'Library Songs 9' },
  ]);
  expect(albumById(library, 'demo-album-15')?.discs).toStrictEqual([
    { index: 1, title: 'Chapter One / Cold' },
    { index: 2, title: 'Chapter Two / Mild' },
  ]);
  for (const album of newAlbums) {
    for (const disc of album.discs) {
      expect(album.tracks.some((track) => track.discIndex === disc.index)).toStrictEqual(true);
    }
  }
});
