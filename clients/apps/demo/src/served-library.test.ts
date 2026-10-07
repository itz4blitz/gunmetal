import { expect, test } from 'vitest';
import { libraryFromDocument, loadServedLibrary } from './served-library.ts';

function at<T>(list: readonly T[], index: number): T {
  const value = list[index];
  if (value === undefined) {
    throw new Error(`missing ${index}`);
  }
  return value;
}

const ALBUM = 'a'.repeat(16);
const ARTIST = 'b'.repeat(16);
const TRACK = 'c'.repeat(16);

function folder(): {
  kind: 'folder';
  albums: Array<Record<string, unknown>>;
  artists: Array<Record<string, unknown>>;
} {
  return {
    kind: 'folder',
    albums: [
      {
        id: ALBUM,
        title: 'St. Elsewhere',
        artistName: 'Gnarls Barkley',
        artistKey: ARTIST,
        year: 2006,
        coverTone: '01',
        coverUrl: `/media/library/covers/${ALBUM}.jpg`,
        discs: [{ index: 1, title: '' }],
        hostile: false,
        tracks: [
          {
            id: TRACK,
            albumId: ALBUM,
            discIndex: 1,
            number: 1,
            title: 'Crazy',
            artistName: 'Gnarls Barkley',
            durationMs: 178_000,
            flag: 'ok',
            lyricsKind: 'none',
            mediaUrl: `/media/library/${TRACK}`,
          },
        ],
      },
    ],
    artists: [{ key: ARTIST, name: 'Gnarls Barkley', albumIds: [ALBUM] }],
  };
}

test('a folder document is the library, unchanged', () => {
  const doc = folder();
  expect(libraryFromDocument(doc)).toStrictEqual(doc);
});

test('an empty folder is a library with no rows', () => {
  expect(libraryFromDocument({ kind: 'folder', albums: [], artists: [] })).toStrictEqual({
    kind: 'folder',
    albums: [],
    artists: [],
  });
});

test('png and empty covers, and the other flag and lyric kinds, are kept', () => {
  const png = folder();
  at(png.albums, 0).coverUrl = `/media/library/covers/${ALBUM}.png`;
  expect(libraryFromDocument(png)).toStrictEqual(png);
  const bare = folder();
  at(bare.albums, 0).coverUrl = '';
  expect(libraryFromDocument(bare)).toStrictEqual(bare);
  const unplayable = folder();
  const unplayableTrack = at(unplayable.albums, 0).tracks as Array<Record<string, unknown>>;
  at(unplayableTrack, 0).flag = 'unplayable';
  expect(libraryFromDocument(unplayable)).toStrictEqual(unplayable);
  const damaged = folder();
  at(at(damaged.albums, 0).tracks as Array<Record<string, unknown>>, 0).flag = 'damaged';
  expect(libraryFromDocument(damaged)).toStrictEqual(damaged);
  const plain = folder();
  at(at(plain.albums, 0).tracks as Array<Record<string, unknown>>, 0).lyricsKind = 'plain';
  expect(libraryFromDocument(plain)).toStrictEqual(plain);
  const synced = folder();
  at(at(synced.albums, 0).tracks as Array<Record<string, unknown>>, 0).lyricsKind = 'synced';
  expect(libraryFromDocument(synced)).toStrictEqual(synced);
});

test('two artists stay in key order and a reversed list is refused', () => {
  const secondAlbum = 'd'.repeat(16);
  const secondArtist = 'e'.repeat(16);
  const secondTrack = 'f'.repeat(16);
  const doc = folder();
  doc.albums.push({
    id: secondAlbum,
    title: 'Master of Puppets',
    artistName: 'Metallica',
    artistKey: secondArtist,
    year: 1986,
    coverTone: '04',
    coverUrl: '',
    discs: [{ index: 1, title: 'Side A' }],
    hostile: false,
    tracks: [
      {
        id: secondTrack,
        albumId: secondAlbum,
        discIndex: 1,
        number: 2,
        title: 'Battery',
        artistName: 'Metallica',
        durationMs: 312_000,
        flag: 'ok',
        lyricsKind: 'none',
        mediaUrl: `/media/library/${secondTrack}`,
      },
    ],
  });
  doc.artists.push({ key: secondArtist, name: 'Metallica', albumIds: [secondAlbum] });
  expect(libraryFromDocument(doc)).toStrictEqual(doc);
  const reversed = folder();
  reversed.albums.push(at(doc.albums, 1));
  reversed.artists.unshift(at(doc.artists, 1));
  expect(libraryFromDocument(reversed)).toStrictEqual(undefined);
});

test('anything that is not a folder document is refused', () => {
  expect(libraryFromDocument(null)).toStrictEqual(undefined);
  expect(libraryFromDocument(['folder'])).toStrictEqual(undefined);
  expect(libraryFromDocument('folder')).toStrictEqual(undefined);
  expect(libraryFromDocument({ kind: 'demo-fixtures', albums: [], artists: [] })).toStrictEqual(undefined);
  expect(libraryFromDocument({ kind: 'folder', albums: {}, artists: [] })).toStrictEqual(undefined);
  expect(libraryFromDocument({ kind: 'folder', albums: [], artists: {} })).toStrictEqual(undefined);
  expect(libraryFromDocument({ kind: 'folder', albums: Array(2001), artists: [] })).toStrictEqual(undefined);
  expect(libraryFromDocument({ kind: 'folder', albums: [], artists: Array(2001) })).toStrictEqual(undefined);
  expect(libraryFromDocument({ kind: 'folder', albums: [null], artists: [] })).toStrictEqual(undefined);
  const hostile = folder();
  at(hostile.albums, 0).hostile = true;
  expect(libraryFromDocument(hostile)).toStrictEqual(undefined);
  const missingHostile = folder();
  delete at(missingHostile.albums, 0).hostile;
  expect(libraryFromDocument(missingHostile)).toStrictEqual(undefined);
  const licensed = folder();
  at(licensed.albums, 0).license = { spdx: 'CC0-1.0' };
  expect(libraryFromDocument(licensed)).toStrictEqual(undefined);
});

test('album identity, text, year, tone and cover url are bounded', () => {
  const cases: Array<(doc: ReturnType<typeof folder>) => void> = [
    (doc) => {
      at(doc.albums, 0).id = 'zzzzzzzzzzzzzzzz';
    },
    (doc) => {
      at(doc.albums, 0).id = 'aa';
    },
    (doc) => {
      at(doc.albums, 0).title = '';
    },
    (doc) => {
      at(doc.albums, 0).title = 'x'.repeat(201);
    },
    (doc) => {
      at(doc.albums, 0).title = 'bad\0name';
    },
    (doc) => {
      at(doc.albums, 0).title = 12;
    },
    (doc) => {
      at(doc.albums, 0).artistName = '';
    },
    (doc) => {
      at(doc.albums, 0).artistKey = 'zzzzzzzzzzzzzzzz';
    },
    (doc) => {
      at(doc.albums, 0).year = '2006';
    },
    (doc) => {
      at(doc.albums, 0).year = 2006.5;
    },
    (doc) => {
      at(doc.albums, 0).year = -1;
    },
    (doc) => {
      at(doc.albums, 0).year = 10_000;
    },
    (doc) => {
      at(doc.albums, 0).coverTone = '08';
    },
    (doc) => {
      at(doc.albums, 0).coverTone = '';
    },
    (doc) => {
      at(doc.albums, 0).coverUrl = 'https://evil.example/a.jpg';
    },
    (doc) => {
      at(doc.albums, 0).coverUrl = `/media/library/covers/${'b'.repeat(16)}.jpg`;
    },
    (doc) => {
      at(doc.albums, 0).coverUrl = `x\0`;
    },
    (doc) => {
      at(doc.albums, 0).coverUrl = 'y'.repeat(81);
    },
    (doc) => {
      at(doc.albums, 0).coverUrl = 4;
    },
  ];
  for (const mutate of cases) {
    const doc = folder();
    mutate(doc);
    expect(libraryFromDocument(doc)).toStrictEqual(undefined);
  }
});

test('discs and tracks that do not belong to the album are refused', () => {
  const noDiscs = folder();
  at(noDiscs.albums, 0).discs = [];
  expect(libraryFromDocument(noDiscs)).toStrictEqual(undefined);
  const manyDiscs = folder();
  at(manyDiscs.albums, 0).discs = Array(21);
  expect(libraryFromDocument(manyDiscs)).toStrictEqual(undefined);
  const discsObject = folder();
  at(discsObject.albums, 0).discs = {};
  expect(libraryFromDocument(discsObject)).toStrictEqual(undefined);
  const noTracks = folder();
  at(noTracks.albums, 0).tracks = [];
  expect(libraryFromDocument(noTracks)).toStrictEqual(undefined);
  const manyTracks = folder();
  at(manyTracks.albums, 0).tracks = Array(501);
  expect(libraryFromDocument(manyTracks)).toStrictEqual(undefined);
  const tracksObject = folder();
  at(tracksObject.albums, 0).tracks = {};
  expect(libraryFromDocument(tracksObject)).toStrictEqual(undefined);
  const badDisc = folder();
  at(badDisc.albums, 0).discs = [null];
  expect(libraryFromDocument(badDisc)).toStrictEqual(undefined);
  const discIndex = folder();
  at(discIndex.albums, 0).discs = [{ index: 0, title: '' }];
  expect(libraryFromDocument(discIndex)).toStrictEqual(undefined);
  const discTitle = folder();
  at(discTitle.albums, 0).discs = [{ index: 1, title: 5 }];
  expect(libraryFromDocument(discTitle)).toStrictEqual(undefined);
  const discNul = folder();
  at(discNul.albums, 0).discs = [{ index: 1, title: '\0' }];
  expect(libraryFromDocument(discNul)).toStrictEqual(undefined);
  const discLong = folder();
  at(discLong.albums, 0).discs = [{ index: 1, title: 'z'.repeat(201) }];
  expect(libraryFromDocument(discLong)).toStrictEqual(undefined);
  const duplicateDisc = folder();
  at(duplicateDisc.albums, 0).discs = [
    { index: 1, title: '' },
    { index: 1, title: 'again' },
  ];
  expect(libraryFromDocument(duplicateDisc)).toStrictEqual(undefined);
  const badTrack = folder();
  at(badTrack.albums, 0).tracks = [null];
  expect(libraryFromDocument(badTrack)).toStrictEqual(undefined);
  const otherDisc = folder();
  at(at(otherDisc.albums, 0).tracks as Array<Record<string, unknown>>, 0).discIndex = 2;
  expect(libraryFromDocument(otherDisc)).toStrictEqual(undefined);
  const sameTrack = folder();
  const tracks = at(sameTrack.albums, 0).tracks as Array<Record<string, unknown>>;
  tracks.push({ ...at(tracks, 0) });
  expect(libraryFromDocument(sameTrack)).toStrictEqual(undefined);
});

test('a track field outside the row is refused', () => {
  const mutateTrack = (patch: Record<string, unknown>) => {
    const doc = folder();
    Object.assign(at(at(doc.albums, 0).tracks as Array<Record<string, unknown>>, 0), patch);
    expect(libraryFromDocument(doc)).toStrictEqual(undefined);
  };
  mutateTrack({ id: 'zzzzzzzzzzzzzzzz' });
  mutateTrack({ id: 'cc' });
  mutateTrack({ albumId: 'b'.repeat(16) });
  mutateTrack({ number: 0 });
  mutateTrack({ number: 1.5 });
  mutateTrack({ title: '' });
  mutateTrack({ artistName: 9 });
  mutateTrack({ durationMs: 0 });
  mutateTrack({ durationMs: 86_400_001 });
  mutateTrack({ flag: 'nope' });
  mutateTrack({ lyricsKind: 'html' });
  mutateTrack({ mediaUrl: 'https://evil.example/track' });
  mutateTrack({ mediaUrl: `/media/library/${'d'.repeat(16)}` });
});

test('artists that do not name this catalogue are refused', () => {
  const image = folder();
  at(image.artists, 0).imageUrl = `/media/library/artists/${ARTIST}.jpg`;
  expect(libraryFromDocument(image)).toStrictEqual(undefined);
  const notObject = folder();
  notObject.artists.splice(0, 1, null as unknown as Record<string, unknown>);
  expect(libraryFromDocument(notObject)).toStrictEqual(undefined);
  const badKey = folder();
  at(badKey.artists, 0).key = 'zzzzzzzzzzzzzzzz';
  expect(libraryFromDocument(badKey)).toStrictEqual(undefined);
  const shortKey = folder();
  at(shortKey.artists, 0).key = 'bb';
  expect(libraryFromDocument(shortKey)).toStrictEqual(undefined);
  const renamed = folder();
  at(renamed.artists, 0).name = 'Someone Else';
  expect(libraryFromDocument(renamed)).toStrictEqual(undefined);
  const stranger = folder();
  at(stranger.artists, 0).key = 'd'.repeat(16);
  at(stranger.artists, 0).albumIds = [];
  expect(libraryFromDocument(stranger)).toStrictEqual(undefined);
  const dropped = folder();
  at(dropped.artists, 0).albumIds = [];
  expect(libraryFromDocument(dropped)).toStrictEqual(undefined);
  const swapped = folder();
  at(swapped.artists, 0).albumIds = ['d'.repeat(16)];
  expect(libraryFromDocument(swapped)).toStrictEqual(undefined);
  const missing = folder();
  missing.artists = [];
  expect(libraryFromDocument(missing)).toStrictEqual(undefined);
  const duplicateAlbum = folder();
  const copy = { ...(at(duplicateAlbum.albums, 0) as Record<string, unknown>) };
  const tracks = copy.tracks as Array<Record<string, unknown>>;
  copy.tracks = [{ ...at(tracks, 0), id: 'd'.repeat(16), mediaUrl: `/media/library/${'d'.repeat(16)}` }];
  duplicateAlbum.albums.push(copy);
  expect(libraryFromDocument(duplicateAlbum)).toStrictEqual(undefined);
});

test('loadServedLibrary keeps a folder and refuses everything else', async () => {
  const doc = folder();
  const loaded = await loadServedLibrary(async () => new Response(JSON.stringify(doc), { status: 200 }));
  expect(loaded).toStrictEqual(doc);
  expect(await loadServedLibrary(async () => new Response('', { status: 404 }))).toStrictEqual(undefined);
  expect(await loadServedLibrary(async () => new Response('[]', { status: 200 }))).toStrictEqual(undefined);
  expect(await loadServedLibrary(async () => Promise.reject(new Error('down')))).toStrictEqual(undefined);
  const broken = { ok: true, json: () => Promise.reject(new Error('bad')) };
  expect(await loadServedLibrary(async () => broken as unknown as Response)).toStrictEqual(undefined);
  const original = globalThis.fetch;
  globalThis.fetch = () => Promise.reject(new Error('down'));
  try {
    expect(await loadServedLibrary()).toStrictEqual(undefined);
  } finally {
    globalThis.fetch = original;
  }
});
