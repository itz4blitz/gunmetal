import type { ShellLibrary } from './library-types.ts';
import {
  emptyPlayback,
  findAlbum,
  findTrack,
  playbackFromAlbum,
  playbackFromTrack,
  type PlaybackSnapshot,
} from './playback.ts';

export function resolveAlbumPlayback(
  library: ShellLibrary | undefined,
  albumId: string,
): PlaybackSnapshot | undefined {
  if (library === undefined) {
    return undefined;
  }
  const album = findAlbum(library, albumId);
  if (album === undefined) {
    return undefined;
  }
  return playbackFromAlbum(album);
}

export function resolveTrackPlayback(
  library: ShellLibrary | undefined,
  albumId: string,
  trackId: string,
): PlaybackSnapshot | undefined {
  if (library === undefined) {
    return undefined;
  }
  const found = findTrack(library, trackId);
  if (found === undefined || found.album.id !== albumId) {
    return undefined;
  }
  return playbackFromTrack(found.album, found.track);
}

export function applyPlayback(
  library: ShellLibrary | undefined,
  albumId: string,
  trackId: string | undefined,
): PlaybackSnapshot {
  if (trackId === undefined) {
    return resolveAlbumPlayback(library, albumId) ?? emptyPlayback();
  }
  return resolveTrackPlayback(library, albumId, trackId) ?? emptyPlayback();
}
