/**
 * PROVISIONAL catalogue DTOs (client plan CP-005: ports/src/provisional/).
 *
 * These describe what the UI reads from a library BEFORE the facade's
 * generated declarations exist. They name the backend packages that will
 * define the real types:
 * - catalogue records (track, album, artist): WP-040 (server wave 1),
 *   reaching TypeScript through WP-235's generated declarations.
 * - the queue document and player state: WP-025 and WP-030, through WP-236.
 *
 * When those land, this file is deleted (CP-058 is not done while this
 * directory holds a file) and the UI reads the generated types instead.
 * Nothing here is a hand-written copy of an existing Rust type.
 */

export type CatalogueTrackFlag = 'ok' | 'unplayable' | 'damaged';

export type CatalogueLyricsKind = 'none' | 'plain' | 'synced';

export type CatalogueDisc = {
  index: number;
  title: string;
};

export type CatalogueTrack = {
  id: string;
  albumId: string;
  discIndex: number;
  number: number;
  title: string;
  artistName: string;
  durationMs: number;
  flag: CatalogueTrackFlag;
  lyricsKind: CatalogueLyricsKind;
  /** Same-origin URL for the playable media of this track. */
  mediaUrl: string;
  /** ReplayGain track gain in decibels. Absent when the file has no tag. */
  trackGainDb?: number | undefined;
  /** ReplayGain album gain in decibels. Absent when the file has no tag. */
  albumGainDb?: number | undefined;
};

export type CatalogueLicense = {
  spdx: 'CC0-1.0' | 'CC-BY-3.0' | 'CC-BY-4.0' | 'CC-BY-SA-3.0';
  attribution: string;
  source: string;
};

export type CatalogueAlbum = {
  id: string;
  title: string;
  artistName: string;
  artistKey: string;
  year: number;
  coverTone: string;
  /** Same-origin URL for generated cover art. */
  coverUrl: string;
  discs: readonly CatalogueDisc[];
  tracks: readonly CatalogueTrack[];
  hostile: boolean;
  license?: CatalogueLicense | undefined;
};

export type CatalogueArtist = {
  key: string;
  name: string;
  albumIds: readonly string[];
  /** Same-origin URL for the artist image, when one exists. */
  imageUrl?: string | undefined;
};

export type CatalogueLibrary = {
  albums: readonly CatalogueAlbum[];
  artists: readonly CatalogueArtist[];
};
