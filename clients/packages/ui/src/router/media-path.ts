/**
 * Media addresses. One pure parse of a path into a closed set of typed
 * routes (SEC-CLI-025). A query, a fragment, or an extra segment is not a
 * route. This module does not fetch or run anything.
 *
 * The official address style is a unique name slug
 * (`/music/albums/harbour-lights`). An opaque id
 * (`/music/albums/demo-album-01`) is the other style. Both parse. The
 * player emits whichever style `addressStyle()` names. This build does
 * not download that choice.
 */

export type MediaKind = 'artist' | 'album' | 'track' | 'movie' | 'show';

export type MediaRoute = {
  kind: MediaKind;
  key: string;
  path: string;
};

const PREFIX: Record<MediaKind, string> = {
  artist: '/music/artists/',
  album: '/music/albums/',
  track: '/music/tracks/',
  movie: '/watch/movies/',
  show: '/watch/shows/',
};

function isKey(value: string): boolean {
  return /^[a-z0-9](?:[a-z0-9-]{0,79})$/.test(value) && !value.includes('--');
}

/** A path parameter, or nothing if this is not a media address. */
export function parseMediaPath(pathname: string): MediaRoute | undefined {
  const kinds = Object.keys(PREFIX) as MediaKind[];
  for (const kind of kinds) {
    const prefix = PREFIX[kind];
    if (!pathname.startsWith(prefix)) {
      continue;
    }
    const key = pathname.slice(prefix.length);
    if (!isKey(key)) {
      return undefined;
    }
    return { kind, key, path: pathname };
  }
  return undefined;
}

export function mediaPath(kind: MediaKind, key: string): string | undefined {
  return parseMediaPath(`${PREFIX[kind]}${key}`)?.path;
}

/** A unique name slug. Empty or hostile names become `item`. */
export function slugify(name: string): string {
  const folded = name.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
  const slug = folded
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48);
  return slug.length > 0 ? slug : 'item';
}

/**
 * A slug that does not collide with another item of the same kind.
 * The id is the tie-break, so two "Intro" tracks do not share an address.
 */
export function uniqueSlug(name: string, id: string, used: ReadonlyMap<string, string>): string {
  const stem = slugify(name);
  if (!used.has(stem) || used.get(stem) === id) {
    return stem;
  }
  const suffix = id.toLowerCase().replace(/[^a-z0-9]+/g, '').slice(0, 8);
  if (suffix.length === 0) {
    return stem;
  }
  return `${stem}-${suffix}`.slice(0, 80);
}

/** A heading for an address whose item is not in the library yet. */
export function labelFromKey(key: string): string {
  return key
    .split('-')
    .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join(' ');
}
