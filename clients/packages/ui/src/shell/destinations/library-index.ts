import type { ShellAlbum, ShellArtist, ShellLibrary } from '../library-types.ts';

/**
 * Map indexes over the catalogue the surface was handed. The pages re-render
 * on every playback tick, so a per-render linear scan per tile or per row
 * (O(n²) across a grid) is replaced by one O(n) build per library change.
 * Lookups only — no rules live here beyond the hostile-exclusion contract
 * the pages already pin.
 */

/** Album id → album. */
export function indexAlbums(library: ShellLibrary): ReadonlyMap<string, ShellAlbum> {
  return new Map(library.albums.map((album) => [album.id, album]));
}

/** Artist key → artist. */
export function indexArtists(library: ShellLibrary): ReadonlyMap<string, ShellArtist> {
  return new Map(library.artists.map((artist) => [artist.key, artist]));
}

/**
 * Artist key → that artist's browsable releases, in library order. A hostile
 * release is never listed — not even beside a clean release that shares its
 * artist key (the corpus must not be recommended from either side).
 */
export function albumsByArtistIndex(library: ShellLibrary): ReadonlyMap<string, readonly ShellAlbum[]> {
  const index = new Map<string, ShellAlbum[]>();
  for (const album of library.albums) {
    if (album.hostile) {
      continue;
    }
    const held = index.get(album.artistKey);
    if (held === undefined) {
      index.set(album.artistKey, [album]);
    } else {
      held.push(album);
    }
  }
  return index;
}

/**
 * The album artist's other releases, in library order — never the album
 * itself. A hostile album recommends nothing, and is never recommended.
 */
export function otherAlbums(
  index: ReadonlyMap<string, readonly ShellAlbum[]>,
  album: ShellAlbum | undefined,
): readonly ShellAlbum[] {
  if (album === undefined || album.hostile) {
    return [];
  }
  return (index.get(album.artistKey) ?? []).filter((other) => other.id !== album.id);
}

/**
 * The artist keys whose own album list holds a hostile release: their rows
 * show the safe label. The rule reads the artist record's albumIds — a
 * synthetic artist row is hostile-labeled even when its key differs from the
 * hostile album's artistKey.
 */
export function hostileArtistKeys(library: ShellLibrary): ReadonlySet<string> {
  const hostileIds = new Set<string>();
  for (const album of library.albums) {
    if (album.hostile) {
      hostileIds.add(album.id);
    }
  }
  const keys = new Set<string>();
  for (const artist of library.artists) {
    if (artist.albumIds.some((id) => hostileIds.has(id))) {
      keys.add(artist.key);
    }
  }
  return keys;
}
