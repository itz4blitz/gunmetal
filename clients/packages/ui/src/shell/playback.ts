import type { ShellAlbum, ShellLibrary, ShellTrack } from './library-types.ts';

export type QueueLine = {
  trackId: string;
  albumId: string;
  title: string;
  artistName: string;
  coverTone: string;
  durationMs: number;
};

export type PlaybackSnapshot = {
  trackId: string | undefined;
  albumId: string | undefined;
  title: string;
  artistName: string;
  coverTone: string;
  playing: boolean;
  positionMs: number;
  durationMs: number;
  queue: readonly QueueLine[];
  queueOpen: boolean;
};

export function emptyPlayback(): PlaybackSnapshot {
  return {
    trackId: undefined,
    albumId: undefined,
    title: '',
    artistName: '',
    coverTone: '01',
    playing: false,
    positionMs: 0,
    durationMs: 0,
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
    durationMs: track.durationMs,
  };
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
    playing: true,
    positionMs: 0,
    durationMs: first.durationMs,
    queue,
    queueOpen: true,
  };
}

export function playbackFromTrack(album: ShellAlbum, track: ShellTrack): PlaybackSnapshot {
  const playable = playableTracks(album);
  const startIndex = playable.findIndex((entry) => entry.id === track.id);
  const ordered =
    startIndex >= 0 ? [...playable.slice(startIndex), ...playable.slice(0, startIndex)] : playable;
  const active = ordered[0] ?? track;
  const queue = ordered.map((entry) => lineFrom(album, entry));
  return {
    trackId: active.id,
    albumId: album.id,
    title: active.title,
    artistName: active.artistName,
    coverTone: album.coverTone,
    playing: true,
    positionMs: 0,
    durationMs: active.durationMs,
    queue,
    queueOpen: true,
  };
}

export function findAlbum(library: ShellLibrary, id: string): ShellAlbum | undefined {
  return library.albums.find((album) => album.id === id);
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
    positionMs: 0,
    durationMs: line.durationMs,
    playing: true,
  };
}

export function togglePlaying(snapshot: PlaybackSnapshot): PlaybackSnapshot {
  if (snapshot.trackId === undefined) {
    return snapshot;
  }
  return { ...snapshot, playing: !snapshot.playing };
}

export function setQueueOpen(snapshot: PlaybackSnapshot, queueOpen: boolean): PlaybackSnapshot {
  return { ...snapshot, queueOpen };
}
