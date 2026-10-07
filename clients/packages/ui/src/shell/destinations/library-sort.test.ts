import { describe, expect, test } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import type { ShellAlbum, ShellArtist, ShellTrack } from '../library-types.ts';
import {
  nextSortOption,
  sortAlbums,
  sortArtists,
  sortTracks,
  type SortOption,
  type TrackRow as TrackRowEntry,
} from './library-sort.ts';

const harbour: ShellAlbum = {
  id: 'a1',
  title: 'harbour lights',
  artistName: 'Mira Sol',
  artistKey: 'mira-sol',
  year: 2021,
  coverTone: '01',
  coverUrl: '',
  hostile: false,
  discs: [{ index: 1, title: '' }],
  tracks: [],
};
const ember: ShellAlbum = {
  ...harbour,
  id: 'a2',
  title: 'Ember',
  artistName: 'Keratin',
  artistKey: 'keratin',
  year: 2019,
};
const delta: ShellAlbum = { ...harbour, id: 'a3', title: 'delta', artistName: 'Ander', artistKey: 'ander', year: 2024 };
const twin: ShellAlbum = {
  ...harbour,
  id: 'a4',
  title: 'Ember',
  artistName: 'Keratin',
  artistKey: 'keratin',
  year: 2020,
};

function track(id: string, title: string, durationMs: number, albumId = 'a1'): ShellTrack {
  return {
    id,
    albumId,
    discIndex: 1,
    number: 1,
    title,
    artistName: 'Mira Sol',
    durationMs,
    flag: 'ok',
    lyricsKind: 'none',
    mediaUrl: '',
  };
}

describe('sortAlbums', () => {
  test('recently added keeps the library order and returns a copy', () => {
    const albums = [harbour, ember, delta];
    const sorted = sortAlbums(albums, 'recent');
    expect(sorted).toStrictEqual([harbour, ember, delta]);
    expect(sorted).not.toBe(albums);
  });

  test('title sorts case-insensitively and falls back to the id for ties', () => {
    const sorted = sortAlbums([harbour, twin, ember, delta], 'title');
    expect(sorted.map((album) => album.id)).toStrictEqual(['a3', 'a2', 'a4', 'a1']);
  });

  test('artist sorts by artist name, then title', () => {
    const sorted = sortAlbums([harbour, twin, delta, ember], 'artist');
    expect(sorted.map((album) => album.id)).toStrictEqual(['a3', 'a2', 'a4', 'a1']);
  });

  test('year sorts newest first, then by title', () => {
    const sorted = sortAlbums([harbour, twin, delta, ember], 'year');
    expect(sorted.map((album) => album.id)).toStrictEqual(['a3', 'a1', 'a4', 'a2']);
  });

  test('a year tie falls through to the title, then the id', () => {
    const tiedYearA: ShellAlbum = { ...harbour, id: 'y2', title: 'Same Tide', year: 2022 };
    const tiedYearB: ShellAlbum = { ...harbour, id: 'y1', title: 'same tide', year: 2022 };
    expect(sortAlbums([tiedYearA, tiedYearB], 'year').map((album) => album.id)).toStrictEqual(['y1', 'y2']);
  });

  test('an empty list sorts to an empty list', () => {
    expect(sortAlbums([], 'title')).toStrictEqual([]);
  });
});

describe('sortArtists', () => {
  const busy: ShellArtist = { key: 'busy', name: 'busy', albumIds: ['a1', 'a2', 'a3'] };
  const lone: ShellArtist = { key: 'lone', name: 'lone', albumIds: ['a4'] };
  const idle: ShellArtist = { key: 'idle', name: 'Idle', albumIds: [] };

  test('default keeps the library order and returns a copy', () => {
    const artists = [busy, lone, idle];
    const sorted = sortArtists(artists, 'default');
    expect(sorted).toStrictEqual([busy, lone, idle]);
    expect(sorted).not.toBe(artists);
  });

  test('name sorts case-insensitively by key for ties', () => {
    expect(sortArtists([busy, lone, idle], 'name').map((artist) => artist.key)).toStrictEqual(['busy', 'idle', 'lone']);
  });

  test('album count sorts most releases first, then by name', () => {
    expect(sortArtists([lone, idle, busy], 'albums').map((artist) => artist.key)).toStrictEqual([
      'busy',
      'lone',
      'idle',
    ]);
  });

  test('a count tie falls through to the name, then the key', () => {
    const zed: ShellArtist = { key: 'zeta', name: 'Zed', albumIds: ['a1', 'a2'] };
    const ada: ShellArtist = { key: 'alpha', name: 'ada', albumIds: ['a3', 'a4'] };
    expect(sortArtists([zed, ada], 'albums').map((artist) => artist.key)).toStrictEqual(['alpha', 'zeta']);
  });

  test('a name tie falls through to the key', () => {
    const one: ShellArtist = { key: 'zeta', name: 'Same Name', albumIds: [] };
    const two: ShellArtist = { key: 'alpha', name: 'Same Name', albumIds: [] };
    expect(sortArtists([one, two], 'name').map((artist) => artist.key)).toStrictEqual(['alpha', 'zeta']);
  });

  test('a count-and-name tie falls through to the key', () => {
    const one: ShellArtist = { key: 'zeta', name: 'Same Name', albumIds: ['a1'] };
    const two: ShellArtist = { key: 'alpha', name: 'Same Name', albumIds: ['a2'] };
    expect(sortArtists([one, two], 'albums').map((artist) => artist.key)).toStrictEqual(['alpha', 'zeta']);
  });
});

describe('sortTracks', () => {
  const rows: TrackRowEntry[] = [
    { album: harbour, track: track('t2', 'b-sky', 200_000) },
    { album: ember, track: track('t1', 'Aurora', 180_000, 'a2') },
    { album: harbour, track: track('t3', 'a-rock', 240_000) },
    { album: ember, track: track('t4', 'b-sky', 100_000) },
  ];

  test('default keeps the flattened library order', () => {
    expect(sortTracks(rows, 'default').map((row) => row.track.id)).toStrictEqual(['t2', 't1', 't3', 't4']);
  });

  test('title sorts case-insensitively, ties by album title', () => {
    expect(sortTracks(rows, 'title').map((row) => row.track.id)).toStrictEqual(['t3', 't1', 't4', 't2']);
  });

  test('duration sorts shortest first, ties by title', () => {
    expect(sortTracks(rows, 'duration').map((row) => row.track.id)).toStrictEqual(['t4', 't1', 't2', 't3']);
  });

  test('a duration tie falls through to the title, then the id', () => {
    const tied: TrackRowEntry[] = [
      { album: harbour, track: track('t9', 'same name', 100_000) },
      { album: ember, track: track('t8', 'same name', 100_000) },
      { album: harbour, track: track('t7', 'same name', 100_000) },
    ];
    expect(sortTracks(tied, 'duration').map((row) => row.track.id)).toStrictEqual(['t7', 't8', 't9']);
  });

  test('a completely tied row compares equal and keeps its place', () => {
    const same = { album: harbour, track: track('t1', 'one colour', 100_000) };
    expect(sortTracks([same, same], 'duration')).toStrictEqual([same, same]);
    expect(sortTracks([same, same], 'title')).toStrictEqual([same, same]);
  });

  test('an empty table sorts to an empty table', () => {
    expect(sortTracks([], 'title')).toStrictEqual([]);
  });

  test('the demo fixture title-sort is total and leaves the input untouched', () => {
    const library = demoLibrary();
    const all: TrackRowEntry[] = library.albums.flatMap((album) => album.tracks.map((track) => ({ album, track })));
    const sorted = sortTracks(all, 'title');
    expect(sorted.length).toStrictEqual(all.length);
    expect(sorted).not.toBe(all);
    for (let index = 1; index < sorted.length; index += 1) {
      const previous = sorted[index - 1]?.track.title.toLowerCase() ?? '';
      const current = sorted[index]?.track.title.toLowerCase() ?? '';
      expect(previous <= current).toStrictEqual(true);
    }
  });
});

describe('nextSortOption', () => {
  const options: SortOption[] = [
    { key: 'a', label: 'A' },
    { key: 'b', label: 'B' },
    { key: 'c', label: 'C' },
  ];

  test('arrows step forward and backward, wrapping at both ends', () => {
    expect(nextSortOption(options, 'a', 'ArrowRight')).toStrictEqual({ key: 'b', label: 'B' });
    expect(nextSortOption(options, 'c', 'ArrowRight')).toStrictEqual({ key: 'a', label: 'A' });
    expect(nextSortOption(options, 'a', 'ArrowLeft')).toStrictEqual({ key: 'c', label: 'C' });
    expect(nextSortOption(options, 'c', 'ArrowDown')).toStrictEqual({ key: 'a', label: 'A' });
    expect(nextSortOption(options, 'a', 'ArrowUp')).toStrictEqual({ key: 'c', label: 'C' });
  });

  test('a key the control does not use answers undefined', () => {
    expect(nextSortOption(options, 'b', 'Tab')).toBeUndefined();
    expect(nextSortOption(options, 'b', 'Enter')).toBeUndefined();
  });

  test('a current choice the tab does not offer answers undefined', () => {
    expect(nextSortOption(options, 'no-such-key', 'ArrowRight')).toBeUndefined();
    expect(nextSortOption([], 'a', 'ArrowRight')).toBeUndefined();
  });
});
