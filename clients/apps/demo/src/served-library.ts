import type { DemoAlbum, DemoArtist, DemoLibrary, DemoTrack } from '../../../packages/fake-server/src/types.ts';

const HEX_ID = /^[a-f0-9]{16}$/;
const TONE = /^0[1-7]$/;
const MAX_TEXT = 200;
const MAX_ALBUMS = 2_000;
const MAX_TRACKS = 500;
const MAX_DURATION_MS = 86_400_000;
const GAIN_MIN_DB = -15;
const GAIN_MAX_DB = 15;

/**
 * A folder library served beside this app. The document is data from
 * another process, so anything that is not this shape is refused and the
 * caller keeps the fixture library.
 */
export function libraryFromDocument(value: unknown): DemoLibrary | undefined {
  if (!isRecord(value) || value.kind !== 'folder') {
    return undefined;
  }
  if (!Array.isArray(value.albums) || !Array.isArray(value.artists)) {
    return undefined;
  }
  if (value.albums.length > MAX_ALBUMS || value.artists.length > MAX_ALBUMS) {
    return undefined;
  }
  const albums: DemoAlbum[] = [];
  const albumIds = new Set<string>();
  const trackIds = new Set<string>();
  for (const entry of value.albums) {
    const album = readAlbum(entry, trackIds);
    if (album === undefined || albumIds.has(album.id)) {
      return undefined;
    }
    albumIds.add(album.id);
    albums.push(album);
  }
  const artists: DemoArtist[] = [];
  let previousKey = '';
  for (const entry of value.artists) {
    const artist = readArtist(entry, albums);
    if (artist === undefined || artist.key <= previousKey) {
      return undefined;
    }
    previousKey = artist.key;
    artists.push(artist);
  }
  const keys = new Set(albums.map((album) => album.artistKey));
  if (artists.length !== keys.size) {
    return undefined;
  }
  return { kind: 'folder', albums, artists };
}

/** Fetch `/library.json`. A missing host, a bad status or a refused document is `undefined`. */
export async function loadServedLibrary(fetchImpl: typeof fetch = globalThis.fetch): Promise<DemoLibrary | undefined> {
  try {
    const response = await fetchImpl('/library.json');
    if (!response.ok) {
      return undefined;
    }
    return libraryFromDocument(await response.json());
  } catch {
    return undefined;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function text(value: unknown, max: number): string | undefined {
  if (typeof value !== 'string' || value.length < 1 || value.length > max || value.includes('\0')) {
    return undefined;
  }
  return value;
}

function textOrEmpty(value: unknown, max: number): string | undefined {
  if (typeof value !== 'string' || value.length > max || value.includes('\0')) {
    return undefined;
  }
  return value;
}

function intIn(value: unknown, min: number, max: number): number | undefined {
  if (typeof value !== 'number' || !Number.isInteger(value) || value < min || value > max) {
    return undefined;
  }
  return value;
}

function readAlbum(value: unknown, trackIds: Set<string>): DemoAlbum | undefined {
  if (!isRecord(value) || value.hostile !== false || 'license' in value) {
    return undefined;
  }
  const id = text(value.id, 16);
  const title = text(value.title, MAX_TEXT);
  const artistName = text(value.artistName, MAX_TEXT);
  const artistKey = text(value.artistKey, 16);
  const year = intIn(value.year, 0, 9999);
  const coverTone = text(value.coverTone, 2);
  const coverUrl = textOrEmpty(value.coverUrl, 80);
  if (
    id === undefined ||
    !HEX_ID.test(id) ||
    title === undefined ||
    artistName === undefined ||
    artistKey === undefined ||
    !HEX_ID.test(artistKey) ||
    year === undefined ||
    coverTone === undefined ||
    !TONE.test(coverTone) ||
    coverUrl === undefined ||
    !coverMatches(id, coverUrl)
  ) {
    return undefined;
  }
  if (!Array.isArray(value.discs) || value.discs.length < 1 || value.discs.length > 20) {
    return undefined;
  }
  if (!Array.isArray(value.tracks) || value.tracks.length < 1 || value.tracks.length > MAX_TRACKS) {
    return undefined;
  }
  const discs = [];
  const discIndexes = new Set<number>();
  for (const entry of value.discs) {
    const disc = readDisc(entry);
    if (disc === undefined || discIndexes.has(disc.index)) {
      return undefined;
    }
    discIndexes.add(disc.index);
    discs.push(disc);
  }
  const tracks: DemoTrack[] = [];
  for (const entry of value.tracks) {
    const track = readTrack(entry, id, discIndexes, trackIds);
    if (track === undefined) {
      return undefined;
    }
    tracks.push(track);
  }
  return { id, title, artistName, artistKey, year, coverTone, coverUrl, discs, tracks, hostile: false };
}

function coverMatches(id: string, coverUrl: string): boolean {
  return (
    coverUrl === '' || coverUrl === `/media/library/covers/${id}.jpg` || coverUrl === `/media/library/covers/${id}.png`
  );
}

function readDisc(value: unknown): { index: number; title: string } | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  const index = intIn(value.index, 1, 99);
  const title = textOrEmpty(value.title, MAX_TEXT);
  if (index === undefined || title === undefined) {
    return undefined;
  }
  return { index, title };
}

function readTrack(
  value: unknown,
  albumId: string,
  discIndexes: ReadonlySet<number>,
  trackIds: Set<string>,
): DemoTrack | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  const id = text(value.id, 16);
  const discIndex = intIn(value.discIndex, 1, 99);
  const number = intIn(value.number, 1, 999);
  const title = text(value.title, MAX_TEXT);
  const artistName = text(value.artistName, MAX_TEXT);
  const durationMs = intIn(value.durationMs, 1, MAX_DURATION_MS);
  const flag = readFlag(value.flag);
  const lyricsKind = readLyricsKind(value.lyricsKind);
  const mediaUrl = text(value.mediaUrl, 40);
  const trackGainDb = readGainDb(value, 'trackGainDb');
  const albumGainDb = readGainDb(value, 'albumGainDb');
  if (
    id === undefined ||
    !HEX_ID.test(id) ||
    trackIds.has(id) ||
    value.albumId !== albumId ||
    discIndex === undefined ||
    !discIndexes.has(discIndex) ||
    number === undefined ||
    title === undefined ||
    artistName === undefined ||
    durationMs === undefined ||
    flag === undefined ||
    lyricsKind === undefined ||
    mediaUrl !== `/media/library/${id}` ||
    trackGainDb === null ||
    albumGainDb === null
  ) {
    return undefined;
  }
  trackIds.add(id);
  const track: DemoTrack = { id, albumId, discIndex, number, title, artistName, durationMs, flag, lyricsKind, mediaUrl };
  if (trackGainDb !== undefined) {
    track.trackGainDb = trackGainDb;
  }
  if (albumGainDb !== undefined) {
    track.albumGainDb = albumGainDb;
  }
  return track;
}

/** Absent is no tag. `null` is a value this document must not carry. */
function readGainDb(source: Record<string, unknown>, key: 'trackGainDb' | 'albumGainDb'): number | null | undefined {
  if (!(key in source)) {
    return undefined;
  }
  const value = source[key];
  if (typeof value !== 'number' || !Number.isFinite(value) || value < GAIN_MIN_DB || value > GAIN_MAX_DB) {
    return null;
  }
  return value;
}

function readFlag(value: unknown): DemoTrack['flag'] | undefined {
  if (value === 'ok' || value === 'unplayable' || value === 'damaged') {
    return value;
  }
  return undefined;
}

function readLyricsKind(value: unknown): DemoTrack['lyricsKind'] | undefined {
  if (value === 'none' || value === 'plain' || value === 'synced') {
    return value;
  }
  return undefined;
}

function artistImageUrl(value: unknown, key: string): string | undefined {
  if (value === undefined || value === '') {
    return '';
  }
  if (value === `/media/library/artists/${key}.jpg` || value === `/media/library/artists/${key}.png`) {
    return value;
  }
  return undefined;
}

function readArtist(value: unknown, albums: readonly DemoAlbum[]): DemoArtist | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  const key = text(value.key, 16);
  const name = text(value.name, MAX_TEXT);
  if (key === undefined || !HEX_ID.test(key) || name === undefined || !Array.isArray(value.albumIds)) {
    return undefined;
  }
  const imageUrl = artistImageUrl(value.imageUrl, key);
  if (imageUrl === undefined) {
    return undefined;
  }
  const owned = albums.filter((album) => album.artistKey === key);
  const expected = owned.map((album) => album.id);
  const named = owned[0];
  if (named === undefined || name !== named.artistName) {
    return undefined;
  }
  if (value.albumIds.length !== expected.length) {
    return undefined;
  }
  for (let index = 0; index < expected.length; index += 1) {
    if (value.albumIds[index] !== expected[index]) {
      return undefined;
    }
  }
  return imageUrl === '' ? { key, name, albumIds: expected } : { key, name, albumIds: expected, imageUrl };
}
