import type { ShellLibrary } from '../../../packages/ui/src/shell/library-types.ts';
import {
  appendAlbum,
  appendQueue,
  emptyPlayback,
  findAlbum,
  findTrack,
  insertAlbumNext,
  insertPlayNext,
  playbackFromAlbum,
  playbackFromTrack,
  queueLineFrom,
  type PlaybackSnapshot,
} from './playback.ts';

export function resolveAlbumPlayback(library: ShellLibrary | undefined, albumId: string): PlaybackSnapshot | undefined {
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

export function applyAlbumQueue(
  library: ShellLibrary | undefined,
  snapshot: PlaybackSnapshot,
  albumId: string,
  mode: 'next' | 'append',
): PlaybackSnapshot {
  if (library === undefined) {
    return snapshot;
  }
  const album = findAlbum(library, albumId);
  if (album === undefined) {
    return snapshot;
  }
  return mode === 'next' ? insertAlbumNext(snapshot, album) : appendAlbum(snapshot, album);
}

export function applyTrackQueue(
  library: ShellLibrary | undefined,
  snapshot: PlaybackSnapshot,
  albumId: string,
  trackId: string,
  mode: 'next' | 'append',
): PlaybackSnapshot {
  if (library === undefined) {
    return snapshot;
  }
  const found = findTrack(library, trackId);
  if (found === undefined || found.album.id !== albumId) {
    return snapshot;
  }
  const line = queueLineFrom(found.album, found.track);
  return mode === 'next' ? insertPlayNext(snapshot, line) : appendQueue(snapshot, line);
}
