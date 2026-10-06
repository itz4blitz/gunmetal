/**
 * Provisional opaque display DTOs for the demo fake-server (CP-011 / CP-012 style).
 * These are NOT Rust catalogue replacements from WP-235. Demo / UI chrome only.
 */

export type DemoTrackFlag = 'ok' | 'unplayable' | 'damaged';

export type DemoLyricsKind = 'none' | 'plain' | 'synced';

export type DemoDisc = {
  index: number;
  title: string;
};

export type DemoTrack = {
  id: string;
  albumId: string;
  discIndex: number;
  number: number;
  title: string;
  artistName: string;
  durationMs: number;
  flag: DemoTrackFlag;
  lyricsKind: DemoLyricsKind;
};

export type DemoAlbum = {
  id: string;
  title: string;
  artistName: string;
  /** Distinguishes same-name artists in the demo library. */
  artistKey: string;
  year: number;
  coverTone: string;
  discs: readonly DemoDisc[];
  tracks: readonly DemoTrack[];
  /** True when every text field holds the hostile-metadata corpus (SEC-CLI-001 demo). */
  hostile: boolean;
};

export type DemoArtist = {
  key: string;
  name: string;
  albumIds: readonly string[];
};

export type DemoLibrary = {
  /** Permanent notice: fixture data only. */
  kind: 'demo-fixtures';
  albums: readonly DemoAlbum[];
  artists: readonly DemoArtist[];
};
