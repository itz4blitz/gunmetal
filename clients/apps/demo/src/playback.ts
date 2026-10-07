import type { PlayerQueueLine, PlayerSnapshot } from '../../../packages/ports/src/provisional/player.ts';
import type {
  ShellAlbum,
  ShellArtist,
  ShellLibrary,
  ShellLyricsKind,
  ShellTrack,
} from '../../../packages/ui/src/shell/library-types.ts';

/** Demo-side aliases of the provisional player types (ports/src/provisional). */
export type QueueLine = PlayerQueueLine;
export type PlaybackSnapshot = PlayerSnapshot;

export function emptyPlayback(): PlaybackSnapshot {
  return {
    trackId: undefined,
    albumId: undefined,
    title: '',
    artistName: '',
    coverTone: '01',
    coverUrl: '',
    mediaUrl: '',
    playing: false,
    positionMs: 0,
    durationMs: 0,
    lyricsKind: 'none',
    queue: [],
    queueOpen: false,
  };
}

function lineFrom(album: ShellAlbum, track: ShellTrack): QueueLine {
  return {
    trackId: track.id,
    albumId: album.id,
    title: track.title,
    artistName: track.artistName,
    coverTone: album.coverTone,
    coverUrl: album.coverUrl,
    mediaUrl: track.mediaUrl,
    durationMs: track.durationMs,
    lyricsKind: track.lyricsKind,
  };
}

export function queueLineFrom(album: ShellAlbum, track: ShellTrack): QueueLine {
  return lineFrom(album, track);
}

function playableTracks(album: ShellAlbum): readonly ShellTrack[] {
  return album.tracks.filter((track) => track.flag === 'ok');
}

export function playbackFromAlbum(album: ShellAlbum): PlaybackSnapshot {
  const playable = playableTracks(album);
  const first = playable[0] ?? album.tracks[0];
  if (first === undefined) {
    return emptyPlayback();
  }
  const queue = playable.map((track) => lineFrom(album, track));
  return {
    trackId: first.id,
    albumId: album.id,
    title: first.title,
    artistName: first.artistName,
    coverTone: album.coverTone,
    coverUrl: album.coverUrl,
    mediaUrl: first.mediaUrl,
    playing: true,
    positionMs: 0,
    durationMs: first.durationMs,
    lyricsKind: first.lyricsKind,
    queue,
    queueOpen: true,
  };
}

export function playbackFromTrack(album: ShellAlbum, track: ShellTrack): PlaybackSnapshot {
  const playable = playableTracks(album);
  const startIndex = playable.findIndex((entry) => entry.id === track.id);
  const ordered = startIndex >= 0 ? [...playable.slice(startIndex), ...playable.slice(0, startIndex)] : playable;
  const active = ordered[0] ?? track;
  const queue = ordered.map((entry) => lineFrom(album, entry));
  return {
    trackId: active.id,
    albumId: album.id,
    title: active.title,
    artistName: active.artistName,
    coverTone: album.coverTone,
    coverUrl: album.coverUrl,
    mediaUrl: active.mediaUrl,
    playing: true,
    positionMs: 0,
    durationMs: active.durationMs,
    lyricsKind: active.lyricsKind,
    queue,
    queueOpen: true,
  };
}

export function findAlbum(library: ShellLibrary, id: string): ShellAlbum | undefined {
  return library.albums.find((album) => album.id === id);
}

export function findArtist(library: ShellLibrary, key: string): ShellArtist | undefined {
  return library.artists.find((artist) => artist.key === key);
}

export function albumsForArtist(library: ShellLibrary, artist: ShellArtist): readonly ShellAlbum[] {
  const albums: ShellAlbum[] = [];
  for (const id of artist.albumIds) {
    const album = findAlbum(library, id);
    if (album !== undefined) {
      albums.push(album);
    }
  }
  return albums;
}

export function findTrack(
  library: ShellLibrary,
  trackId: string,
): { album: ShellAlbum; track: ShellTrack } | undefined {
  for (const album of library.albums) {
    const track = album.tracks.find((entry) => entry.id === trackId);
    if (track !== undefined) {
      return { album, track };
    }
  }
  return undefined;
}

export function stepQueue(snapshot: PlaybackSnapshot, direction: -1 | 1): PlaybackSnapshot {
  if (snapshot.queue.length === 0 || snapshot.trackId === undefined) {
    return snapshot;
  }
  const index = snapshot.queue.findIndex((line) => line.trackId === snapshot.trackId);
  if (index < 0) {
    return snapshot;
  }
  const nextIndex = index + direction;
  if (nextIndex < 0 || nextIndex >= snapshot.queue.length) {
    return { ...snapshot, playing: false };
  }
  const line = snapshot.queue[nextIndex];
  if (line === undefined) {
    return snapshot;
  }
  return {
    ...snapshot,
    trackId: line.trackId,
    albumId: line.albumId,
    title: line.title,
    artistName: line.artistName,
    coverTone: line.coverTone,
    coverUrl: line.coverUrl,
    mediaUrl: line.mediaUrl,
    positionMs: 0,
    durationMs: line.durationMs,
    lyricsKind: line.lyricsKind,
    playing: true,
  };
}

export function playFromLine(line: QueueLine): PlaybackSnapshot {
  return {
    trackId: line.trackId,
    albumId: line.albumId,
    title: line.title,
    artistName: line.artistName,
    coverTone: line.coverTone,
    coverUrl: line.coverUrl,
    mediaUrl: line.mediaUrl,
    playing: true,
    positionMs: 0,
    durationMs: line.durationMs,
    lyricsKind: line.lyricsKind,
    queue: [line],
    queueOpen: true,
  };
}

export function insertPlayNext(snapshot: PlaybackSnapshot, line: QueueLine): PlaybackSnapshot {
  if (snapshot.trackId === undefined) {
    return playFromLine(line);
  }
  const index = snapshot.queue.findIndex((entry) => entry.trackId === snapshot.trackId);
  if (index < 0) {
    return { ...snapshot, queue: [...snapshot.queue, line], queueOpen: true };
  }
  return {
    ...snapshot,
    queue: [...snapshot.queue.slice(0, index + 1), line, ...snapshot.queue.slice(index + 1)],
    queueOpen: true,
  };
}

export function appendQueue(snapshot: PlaybackSnapshot, line: QueueLine): PlaybackSnapshot {
  if (snapshot.trackId === undefined) {
    return playFromLine(line);
  }
  return { ...snapshot, queue: [...snapshot.queue, line], queueOpen: true };
}

export function insertAlbumNext(snapshot: PlaybackSnapshot, album: ShellAlbum): PlaybackSnapshot {
  const lines = playableTracks(album).map((track) => lineFrom(album, track));
  if (lines.length === 0) {
    return snapshot;
  }
  if (snapshot.trackId === undefined) {
    return playbackFromAlbum(album);
  }
  let next = snapshot;
  for (const line of [...lines].reverse()) {
    next = insertPlayNext(next, line);
  }
  return next;
}

export function appendAlbum(snapshot: PlaybackSnapshot, album: ShellAlbum): PlaybackSnapshot {
  const lines = playableTracks(album).map((track) => lineFrom(album, track));
  if (lines.length === 0) {
    return snapshot;
  }
  if (snapshot.trackId === undefined) {
    return playbackFromAlbum(album);
  }
  let next = snapshot;
  for (const line of lines) {
    next = appendQueue(next, line);
  }
  return next;
}

export function togglePlaying(snapshot: PlaybackSnapshot): PlaybackSnapshot {
  if (snapshot.trackId === undefined) {
    return snapshot;
  }
  return { ...snapshot, playing: !snapshot.playing };
}

/** Demo-only display state: clamps the reported position into the track. */
export function seekTo(snapshot: PlaybackSnapshot, positionMs: number): PlaybackSnapshot {
  if (snapshot.trackId === undefined || snapshot.durationMs <= 0) {
    return snapshot;
  }
  const clamped = Math.max(0, Math.min(snapshot.durationMs, positionMs));
  return { ...snapshot, positionMs: clamped };
}

export function setQueueOpen(snapshot: PlaybackSnapshot, queueOpen: boolean): PlaybackSnapshot {
  return { ...snapshot, queueOpen };
}
