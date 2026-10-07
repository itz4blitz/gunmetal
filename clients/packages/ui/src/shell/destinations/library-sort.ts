import type { ShellAlbum, ShellArtist, ShellTrack } from '../library-types.ts';

/**
 * Deterministic sort comparators for the Library tabs. Pure and dependency-
 * free: ordering uses raw catalogue values (`<`/`>` only — no `Intl`, so the
 * order is identical in every environment) while rendering stays free to
 * swap hostile labels on top.
 */

export type AlbumSort = 'recent' | 'title' | 'artist' | 'year';
export type ArtistSort = 'default' | 'name' | 'albums';
export type TrackSort = 'default' | 'title' | 'duration';

/** One flattened track-table row: the track and the album it lives on. */
export type TrackRow = { album: ShellAlbum; track: ShellTrack };

/** Case-insensitive text order, falling back through the given tiebreakers. */
function byText(left: string, right: string, ...ties: ReadonlyArray<() => number>): number {
  const a = left.toLowerCase();
  const b = right.toLowerCase();
  if (a < b) {
    return -1;
  }
  if (a > b) {
    return 1;
  }
  for (const tie of ties) {
    const outcome = tie();
    if (outcome !== 0) {
      return outcome;
    }
  }
  return 0;
}

function byNumber(left: number, right: number, ...ties: ReadonlyArray<() => number>): number {
  if (left < right) {
    return -1;
  }
  if (left > right) {
    return 1;
  }
  for (const tie of ties) {
    const outcome = tie();
    if (outcome !== 0) {
      return outcome;
    }
  }
  return 0;
}

/**
 * Album order for the Library's albums tab. `recent` is the library's own
 * order (the feed the sync wrote); `year` is newest first.
 */
export function sortAlbums(albums: readonly ShellAlbum[], sort: AlbumSort): readonly ShellAlbum[] {
  const copy = [...albums];
  if (sort === 'title') {
    copy.sort((left, right) => byText(left.title, right.title, () => byText(left.id, right.id)));
  } else if (sort === 'artist') {
    copy.sort((left, right) =>
      byText(
        left.artistName,
        right.artistName,
        () => byText(left.title, right.title),
        () => byText(left.id, right.id),
      ),
    );
  } else if (sort === 'year') {
    copy.sort((left, right) =>
      byNumber(
        right.year,
        left.year,
        () => byText(left.title, right.title),
        () => byText(left.id, right.id),
      ),
    );
  }
  return copy;
}

/** Artist order for the Library's artists tab; `default` is the library's own, `albums` is most releases first. */
export function sortArtists(artists: readonly ShellArtist[], sort: ArtistSort): readonly ShellArtist[] {
  const copy = [...artists];
  if (sort === 'name') {
    copy.sort((left, right) => byText(left.name, right.name, () => byText(left.key, right.key)));
  } else if (sort === 'albums') {
    copy.sort((left, right) =>
      byNumber(
        right.albumIds.length,
        left.albumIds.length,
        () => byText(left.name, right.name),
        () => byText(left.key, right.key),
      ),
    );
  }
  return copy;
}

/** Track-table order for the Library's tracks tab; `default` is the library's own. */
export function sortTracks(rows: readonly TrackRow[], sort: TrackSort): readonly TrackRow[] {
  const copy = [...rows];
  if (sort === 'title') {
    copy.sort((left, right) =>
      byText(
        left.track.title,
        right.track.title,
        () => byText(left.album.title, right.album.title),
        () => byText(left.track.id, right.track.id),
      ),
    );
  } else if (sort === 'duration') {
    copy.sort((left, right) =>
      byNumber(
        left.track.durationMs,
        right.track.durationMs,
        () => byText(left.track.title, right.track.title),
        () => byText(left.track.id, right.track.id),
      ),
    );
  }
  return copy;
}

/** One choice in the sort radiogroup: a catalogue key with its label. */
export type SortOption = { readonly key: string; readonly label: string };

/**
 * The option the arrows move to, wrapping at the ends — the Settings
 * radiogroup's automatic model. Any other key, or a current choice the tab
 * does not offer, answers undefined and the control does nothing.
 */
export function nextSortOption(options: readonly SortOption[], current: string, key: string): SortOption | undefined {
  const index = options.findIndex((option) => option.key === current);
  if (index === -1) {
    return undefined;
  }
  const delta = key === 'ArrowRight' || key === 'ArrowDown' ? 1 : key === 'ArrowLeft' || key === 'ArrowUp' ? -1 : 0;
  if (delta === 0) {
    return undefined;
  }
  return options[(index + delta + options.length) % options.length];
}
