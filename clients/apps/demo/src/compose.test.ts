import { expect, test } from 'vitest';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';
import { composeDemo, timedVerse } from './compose.ts';

test('the demo composition root injects fixture library and labels demo data', () => {
  const demo = composeDemo();
  expect(demo.showDemoLabel).toStrictEqual(true);
  expect(demo.library.kind).toStrictEqual('demo-fixtures');
  expect(demo.library.albums.map((album) => album.id)).toStrictEqual([
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
});

test('a folder library hides the demo label and does not invent fixture verses', () => {
  const album = 'a'.repeat(16);
  const artist = 'b'.repeat(16);
  const track = 'c'.repeat(16);
  const library: DemoLibrary = {
    kind: 'folder',
    albums: [
      {
        id: album,
        title: 'St. Elsewhere',
        artistName: 'Gnarls Barkley',
        artistKey: artist,
        year: 2006,
        coverTone: '01',
        coverUrl: `/media/library/covers/${album}.jpg`,
        discs: [{ index: 1, title: '' }],
        hostile: false,
        tracks: [
          {
            id: track,
            albumId: album,
            discIndex: 1,
            number: 1,
            title: 'Crazy',
            artistName: 'Gnarls Barkley',
            durationMs: 178_000,
            flag: 'ok',
            lyricsKind: 'none',
            mediaUrl: `/media/library/${track}`,
          },
        ],
      },
    ],
    artists: [{ key: artist, name: 'Gnarls Barkley', albumIds: [album] }],
  };
  const demo = composeDemo(library);
  expect(demo.showDemoLabel).toStrictEqual(false);
  expect(demo.library).toStrictEqual(library);
  expect(demo.lyricsFor(track, 'plain')).toStrictEqual(['This file has no lyrics.']);
  expect(demo.timedLyricsFor(track, 'synced')).toStrictEqual([]);
  const release = library.albums[0];
  if (release === undefined) {
    throw new Error('fixture album missing');
  }
  const trackRow = release.tracks[0];
  if (trackRow === undefined) {
    throw new Error('fixture track missing');
  }
  expect(demo.searchLibrary(library, 'crazy')).toStrictEqual({
    albums: [],
    tracks: [trackRow],
  });
  expect(demo.pluginSlots).toStrictEqual([
    { id: 'metadata-provider', status: 'on' },
    { id: 'lyrics-provider', status: 'not-in-build' },
    { id: 'search-provider', status: 'not-in-build' },
    { id: 'scrobbler', status: 'not-in-build' },
    { id: 'theme-pack', status: 'not-a-plugin' },
    { id: 'home-row', status: 'not-a-plugin' },
  ]);
});

test('the lyrics resolver returns the fixture lines, the fallback verse, or the no-lyrics line', () => {
  const demo = composeDemo();
  // A track with a fixture verse gets exactly its lines.
  expect(demo.lyricsFor('demo-track-01-03', 'plain')).toStrictEqual([
    'The harbour keeps the letter',
    'folded under glass',
    'until the tide comes back',
  ]);
  // A plain track without a table entry gets the shared fallback verse.
  expect(demo.lyricsFor('demo-track-01-01', 'plain')).toStrictEqual([
    'A short demo verse',
    'lives in this fixture',
    'no markup, only lines',
  ]);
  // A track without lyrics gets the one honest line.
  expect(demo.lyricsFor('demo-track-01-01', 'none')).toStrictEqual(['This file has no lyrics.']);
});

test('fixture extensions jobs are first-party facts, and Cover Art Archive is not on', () => {
  const demo = composeDemo();
  expect(demo.pluginSlots).toStrictEqual([
    { id: 'metadata-provider', status: 'not-serving' },
    { id: 'lyrics-provider', status: 'not-in-build' },
    { id: 'search-provider', status: 'not-in-build' },
    { id: 'scrobbler', status: 'not-in-build' },
    { id: 'theme-pack', status: 'not-a-plugin' },
    { id: 'home-row', status: 'not-a-plugin' },
  ]);
});

test('timedVerse spreads the lines evenly across the duration, first line off zero', () => {
  // Three lines over 20 seconds: steps at 5s, 10s, 15s.
  expect(timedVerse(['a', 'b', 'c'], 20_000)).toStrictEqual([
    { atMs: 5_000, text: 'a' },
    { atMs: 10_000, text: 'b' },
    { atMs: 15_000, text: 'c' },
  ]);
  // No usable duration: one second per line, still ordered.
  expect(timedVerse(['a', 'b'], 0)).toStrictEqual([
    { atMs: 1_000, text: 'a' },
    { atMs: 2_000, text: 'b' },
  ]);
  // Rounding: four lines over 10 seconds land on 2s, 4s, 6s, 8s.
  expect(timedVerse(['a', 'b', 'c', 'd'], 10_000).map((line) => line.atMs)).toStrictEqual([2_000, 4_000, 6_000, 8_000]);
});

test('the timed resolver answers the fixture verse timed to the track, or nothing', () => {
  const demo = composeDemo();
  // The fixture track runs 241 seconds; the verse's three lines land a
  // quarter, a half and three quarters of the way into it.
  expect(demo.timedLyricsFor('demo-track-01-03', 'plain')).toStrictEqual([
    { atMs: 60_250, text: 'The harbour keeps the letter' },
    { atMs: 120_500, text: 'folded under glass' },
    { atMs: 180_750, text: 'until the tide comes back' },
  ]);
  // A track without lyrics never reaches the timed resolver: its kind is
  // 'none' and the pane keeps its plain render. An unknown plain track gets
  // the fallback verse, timed at one second a line.
  expect(demo.timedLyricsFor('demo-track-not-real', 'plain')).toStrictEqual([
    { atMs: 1_000, text: 'A short demo verse' },
    { atMs: 2_000, text: 'lives in this fixture' },
    { atMs: 3_000, text: 'no markup, only lines' },
  ]);
});
