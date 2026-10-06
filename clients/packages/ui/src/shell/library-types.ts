/**
 * Provisional opaque display DTOs accepted by the shell for demo compose.
 * Mirrored by clients/packages/fake-server — NOT Rust catalogue types.
 */

export type ShellTrackFlag = 'ok' | 'unplayable' | 'damaged';

export type ShellLyricsKind = 'none' | 'plain' | 'synced';

export type ShellDisc = {
  index: number;
  title: string;
};

export type ShellTrack = {
  id: string;
  albumId: string;
  discIndex: number;
  number: number;
  title: string;
  artistName: string;
  durationMs: number;
  flag: ShellTrackFlag;
  lyricsKind: ShellLyricsKind;
};

export type ShellAlbum = {
  id: string;
  title: string;
  artistName: string;
  artistKey: string;
  year: number;
  coverTone: string;
  discs: readonly ShellDisc[];
  tracks: readonly ShellTrack[];
  hostile: boolean;
};

export type ShellArtist = {
  key: string;
  name: string;
  albumIds: readonly string[];
};

export type ShellLibrary = {
  kind: 'demo-fixtures';
  albums: readonly ShellAlbum[];
  artists: readonly ShellArtist[];
};
