import { describe, expect, test } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import type { ShellLibrary } from '../library-types.ts';
import { albumsByArtistIndex, hostileArtistKeys, indexAlbums, indexArtists, otherAlbums } from './library-index.ts';

describe('indexAlbums', () => {
  test('answers every album by id in O(1) and keeps the album object identical', () => {
    const library = demoLibrary();
    const index = indexAlbums(library);
    for (const album of library.albums) {
      expect(index.get(album.id)).toBe(album);
    }
    expect(index.size).toStrictEqual(library.albums.length);
    expect(index.get('demo-album-01')?.title).toStrictEqual('Harbour Lights');
    expect(index.get('no-such-album')).toBeUndefined();
  });

  test('an empty library indexes to an empty map', () => {
    expect(indexAlbums({ albums: [], artists: [] }).size).toStrictEqual(0);
  });
});

describe('indexArtists', () => {
  test('answers every artist by key and misses politely', () => {
    const library = demoLibrary();
    const index = indexArtists(library);
    for (const artist of library.artists) {
      expect(index.get(artist.key)).toBe(artist);
    }
    expect(index.size).toStrictEqual(library.artists.length);
    expect(index.get('no-such-artist')).toBeUndefined();
  });
});

describe('albumsByArtistIndex', () => {
  test('groups the browsable releases under each artist key, in library order', () => {
    const library = demoLibrary();
    const index = albumsByArtistIndex(library);
    const mira = index.get('mira-sol');
    expect(mira?.map((album) => album.id)).toStrictEqual(['demo-album-01', 'demo-album-02']);
  });

  test('a hostile release is grouped under its artist but excluded from every other album’s view', () => {
    const library = demoLibrary();
    const index = albumsByArtistIndex(library);
    const hostile = library.albums.find((album) => album.hostile);
    if (hostile === undefined) {
      throw new Error('fixture hostile album missing');
    }
    // The hostile fixture's own artist key holds only the hostile album.
    expect(index.get(hostile.artistKey) ?? []).toStrictEqual([]);
  });

  test('a clean album sharing the hostile album’s artist key still recommends nothing hostile', () => {
    // The pinned rule: a shared key must not surface either one beside the other.
    const base = demoLibrary();
    const library: ShellLibrary = {
      ...base,
      albums: base.albums.map((album) =>
        album.id === 'demo-album-01' ? { ...album, artistKey: 'hostile-artist' } : album,
      ),
    };
    const index = albumsByArtistIndex(library);
    // The clean album is the only browsable one under the shared key…
    expect(index.get('hostile-artist')?.map((album) => album.id)).toStrictEqual(['demo-album-01']);
    // …so its own page lists nothing (the hostile release never shows).
    const album01 = indexAlbums(library).get('demo-album-01');
    expect(otherAlbums(index, album01)).toStrictEqual([]);
    // And the hostile page recommends nothing at all.
    const hostile = library.albums.find((row) => row.hostile);
    if (hostile === undefined) {
      throw new Error('fixture hostile album missing');
    }
    expect(otherAlbums(index, hostile)).toStrictEqual([]);
  });
});

describe('otherAlbums', () => {
  test('lists the artist’s other browsable releases in library order', () => {
    const library = demoLibrary();
    const index = albumsByArtistIndex(library);
    const harbour = indexAlbums(library).get('demo-album-01');
    expect(otherAlbums(index, harbour).map((album) => album.id)).toStrictEqual(['demo-album-02']);
  });

  test('a hostile album recommends nothing', () => {
    const library = demoLibrary();
    const index = albumsByArtistIndex(library);
    const hostile = library.albums.find((album) => album.hostile);
    if (hostile === undefined) {
      throw new Error('fixture hostile album missing');
    }
    expect(otherAlbums(index, hostile)).toStrictEqual([]);
  });

  test('a missing album recommends nothing', () => {
    const library = demoLibrary();
    expect(otherAlbums(albumsByArtistIndex(library), undefined)).toStrictEqual([]);
  });

  test('an index without the artist key recommends nothing rather than guessing', () => {
    const library = demoLibrary();
    const harbour = indexAlbums(library).get('demo-album-01');
    expect(otherAlbums(new Map(), harbour)).toStrictEqual([]);
  });
});

describe('hostileArtistKeys', () => {
  test('marks exactly the artists whose own album list holds a hostile release', () => {
    const library = demoLibrary();
    const keys = hostileArtistKeys(library);
    const hostile = library.albums.find((album) => album.hostile);
    if (hostile === undefined) {
      throw new Error('fixture hostile album missing');
    }
    expect(keys.has(hostile.artistKey)).toStrictEqual(true);
    expect(keys.has('mira-sol')).toStrictEqual(false);
    expect(keys.size).toStrictEqual(1);
  });

  test('a synthetic artist row is marked through its albumIds, not its key', () => {
    const base = demoLibrary();
    const hostile = base.albums.find((album) => album.hostile);
    if (hostile === undefined) {
      throw new Error('fixture hostile album missing');
    }
    const keys = hostileArtistKeys({
      albums: base.albums,
      artists: [{ key: 'mixed', name: 'Corpus Mixed', albumIds: ['demo-album-01', hostile.id] }],
    });
    expect(keys.has('mixed')).toStrictEqual(true);
  });

  test('a library without hostile releases marks nobody', () => {
    const base = demoLibrary();
    expect(
      hostileArtistKeys({ albums: base.albums.filter((album) => !album.hostile), artists: [] }).size,
    ).toStrictEqual(0);
  });

  test('an empty library marks nobody', () => {
    expect(hostileArtistKeys({ albums: [], artists: [] }).size).toStrictEqual(0);
  });
});
