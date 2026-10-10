import { expect, test } from 'vitest';
import { openPlayer } from './open-player.ts';

const ALBUM = 'a'.repeat(16);
const ARTIST = 'b'.repeat(16);
const TRACK = 'c'.repeat(16);

function folderDocument(): {
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

test('the player opens from GET /library.json and keeps the relative media URL', async () => {
  const previous = globalThis.fetch;
  const calls: string[] = [];
  globalThis.fetch = (async (input: RequestInfo | URL) => {
    calls.push(String(input));
    return new Response(JSON.stringify(folderDocument()), { status: 200 });
  }) as typeof fetch;
  try {
    const root = document.createElement('div');
    const seen: Array<{ kind: string | undefined; mediaUrl: string | undefined; coverUrl: string | undefined }> = [];
    const result = await openPlayer(root, (element, library) => {
      seen.push({
        kind: library?.kind,
        mediaUrl: library?.albums[0]?.tracks[0]?.mediaUrl,
        coverUrl: library?.albums[0]?.coverUrl,
      });
      expect(element).toStrictEqual(root);
    });
    expect({ result, calls, seen }).toStrictEqual({
      result: 'opened',
      calls: ['/library.json'],
      seen: [
        {
          kind: 'folder',
          mediaUrl: `/media/library/${TRACK}`,
          coverUrl: `/media/library/covers/${ALBUM}.jpg`,
        },
      ],
    });
  } finally {
    globalThis.fetch = previous;
  }
});

test('a missing or refused library still opens, and the caller keeps the fixture', async () => {
  const root = document.createElement('div');
  const seen: Array<string | undefined> = [];
  const missing = await openPlayer(
    root,
    (_element, library) => {
      seen.push(library?.kind);
    },
    async () => new Response('no', { status: 404 }),
  );
  const refused = await openPlayer(
    root,
    (_element, library) => {
      seen.push(library?.kind);
    },
    async () => new Response('{"kind":"nope"}', { status: 200 }),
  );
  const down = await openPlayer(
    root,
    (_element, library) => {
      seen.push(library?.kind);
    },
    async () => {
      throw new Error('down');
    },
  );
  expect({ missing, refused, down, seen }).toStrictEqual({
    missing: 'opened',
    refused: 'opened',
    down: 'opened',
    seen: [undefined, undefined, undefined],
  });
});
