/**
 * Turn a library item into the address this build emits, and turn an
 * address key back into that item. The official url-style extension names
 * the canonical style. An id still resolves when the style is a slug, and
 * a slug still resolves when the style is an id, so both open the same page.
 */

import type { ShellAlbum, ShellLibrary, ShellTrack } from '../shell/library-types.ts';
import { addressStyle, type AddressStyle } from '../plugins/repository.ts';
import { mediaPath, uniqueSlug, type MediaKind, type MediaRoute } from './media-path.ts';

export type ResolvedMedia = {
  kind: 'artist' | 'album' | 'track';
  itemId: string;
  label: string;
  path: string;
};

type Named = {
  id: string;
  name: string;
  label: string;
  itemId: string;
};

const PREFIX: Record<MediaKind, string> = {
  artist: '/music/artists/',
  album: '/music/albums/',
  track: '/music/tracks/',
  movie: '/watch/movies/',
  show: '/watch/shows/',
};

function emit(kind: MediaKind, id: string, slug: string, style: AddressStyle): string {
  if (style === 'id') {
    const byId = mediaPath(kind, id);
    if (byId !== undefined) {
      return byId;
    }
  }
  return `${PREFIX[kind]}${slug}`;
}

function pairsFor(entries: readonly Named[]): { entry: Named; slug: string }[] {
  const used = new Map<string, string>();
  for (const entry of entries) {
    if (/^[a-z0-9](?:[a-z0-9-]{0,79})$/.test(entry.id) && !entry.id.includes('--')) {
      used.set(entry.id, entry.id);
    }
  }
  const pairs: { entry: Named; slug: string }[] = [];
  for (const entry of entries) {
    const slug = uniqueSlug(entry.name, entry.id, used);
    used.set(slug, entry.id);
    pairs.push({ entry, slug });
  }
  return pairs;
}

function addressFor(kind: MediaKind, id: string, entries: readonly Named[], style: AddressStyle): string | undefined {
  const pair = pairsFor(entries).find((item) => item.entry.id === id);
  if (pair === undefined) {
    return undefined;
  }
  return emit(kind, id, pair.slug, style);
}

function findNamed(entries: readonly Named[], key: string): { entry: Named; slug: string } | undefined {
  const pairs = pairsFor(entries);
  const byId = pairs.find((pair) => pair.entry.id === key);
  if (byId !== undefined) {
    return byId;
  }
  return pairs.find((pair) => pair.slug === key);
}

function artists(library: ShellLibrary): Named[] {
  return library.artists.map((artist) => ({
    id: artist.key,
    name: artist.name,
    label: artist.name,
    itemId: artist.key,
  }));
}

function albums(library: ShellLibrary): Named[] {
  return library.albums.map((album) => ({
    id: album.id,
    name: album.title,
    label: album.title,
    itemId: album.id,
  }));
}

function tracks(library: ShellLibrary): Named[] {
  return library.albums.flatMap((album) => album.tracks.map((track) => trackNamed(album, track)));
}

function trackNamed(album: ShellAlbum, track: ShellTrack): Named {
  return {
    id: track.id,
    name: track.title,
    label: track.title,
    itemId: album.id,
  };
}

export function albumAddress(
  library: ShellLibrary,
  albumId: string,
  style: AddressStyle = addressStyle(),
): string | undefined {
  return addressFor('album', albumId, albums(library), style);
}

export function artistAddress(
  library: ShellLibrary,
  artistKey: string,
  style: AddressStyle = addressStyle(),
): string | undefined {
  return addressFor('artist', artistKey, artists(library), style);
}

export function trackAddress(
  library: ShellLibrary,
  trackId: string,
  style: AddressStyle = addressStyle(),
): string | undefined {
  return addressFor('track', trackId, tracks(library), style);
}

/** The address an album or artist click should push. A missing library has none. */
export function itemAddress(
  library: ShellLibrary | undefined,
  itemId: string,
  style: AddressStyle = addressStyle(),
): string | undefined {
  if (library === undefined) {
    return undefined;
  }
  return artistAddress(library, itemId, style) ?? albumAddress(library, itemId, style);
}

export function resolveMedia(
  library: ShellLibrary,
  media: MediaRoute,
  style: AddressStyle = addressStyle(),
): ResolvedMedia | undefined {
  if (media.kind === 'movie') {
    return undefined;
  }
  if (media.kind === 'show') {
    return undefined;
  }
  const found = findNamed(entriesFor(library, media.kind), media.key);
  if (found === undefined) {
    return undefined;
  }
  return {
    kind: media.kind,
    itemId: found.entry.itemId,
    label: found.entry.label,
    path: emit(media.kind, found.entry.id, found.slug, style),
  };
}

function entriesFor(library: ShellLibrary, kind: 'artist' | 'album' | 'track'): readonly Named[] {
  if (kind === 'artist') {
    return artists(library);
  }
  if (kind === 'album') {
    return albums(library);
  }
  return tracks(library);
}
