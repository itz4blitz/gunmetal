import type { PlayerQueueLine, PlayerSnapshot } from '../../../packages/ports/src/provisional/player.ts';
import type {
  ShellAlbum,
  ShellArtist,
  ShellLibrary,
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
    queueOpen: false,
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
    queueOpen: false,
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
  /* Shuffle rides on the queue: the same lines, walked in the seed's order.
     Without shuffle the queue's own order is the walk. */
  const order = snapshot.shuffleOn === true ? shuffledIndices(snapshot.queue.length, snapshot.shuffleSeed ?? 0) : null;
  const walked = order === null ? index : order.indexOf(index);
  let nextWalked = walked + direction;
  if (nextWalked < 0 || nextWalked >= snapshot.queue.length) {
    /* Repeat all turns the end of the walk into its beginning; without it
       the queue ends and playback stops, as it always has. */
    if (snapshot.repeatMode === 'all') {
      nextWalked = (nextWalked + snapshot.queue.length) % snapshot.queue.length;
    } else {
      return { ...snapshot, playing: false };
    }
  }
  /* The walk indexes the seed's order. A missing entry (a hole left by a
     vanished line) reads as no index at all, which indexes to no line. */
  const queueIndex = order === null ? nextWalked : Number(order[nextWalked]);
  const line = snapshot.queue[queueIndex];
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

/**
 * A track that ran to its own end hands over: repeat one replays it from
 * the top, repeat all walks on past the queue's end, and with repeat off
 * the queue ends and playback stops.
 */
export function advanceQueue(snapshot: PlaybackSnapshot): PlaybackSnapshot {
  if (snapshot.trackId === undefined || snapshot.queue.length === 0) {
    return snapshot;
  }
  if (snapshot.repeatMode === 'one') {
    return { ...snapshot, positionMs: 0, playing: true };
  }
  /* Repeat all walks past the queue's end; with repeat off the walk stops
     and playback ends, as it always has. */
  return stepQueue(snapshot, 1);
}

/** A small prime-modulo LCG step, enough order for a display-only shuffle. */
function nextSeed(state: number): number {
  return (state * 48271) % 2147483647;
}

/** The queue's indices in the seed's order; the same seed walks the same walk. */
export function shuffledIndices(count: number, seed: number): readonly number[] {
  const indices = Array.from({ length: count }, (_, index) => index);
  let state = seed % 2147483647;
  if (state <= 0) {
    state += 2147483646;
  }
  for (let i = count - 1; i > 0; i -= 1) {
    state = nextSeed(state);
    const j = state % (i + 1);
    /* The swap is between two indexes of the array being built, so both
       entries are present; the bounds are the loop's own. */
    const atI = indices[i] as number;
    const atJ = indices[j] as number;
    indices[i] = atJ;
    indices[j] = atI;
  }
  return indices;
}

/** Turn shuffle on (recording the order's seed) or off (dropping the order). */
export function toggleShuffleMode(snapshot: PlaybackSnapshot, seed: number): PlaybackSnapshot {
  if (snapshot.shuffleOn === true) {
    return { ...snapshot, shuffleOn: false, shuffleSeed: seed };
  }
  return { ...snapshot, shuffleOn: true, shuffleSeed: seed };
}

/** Walk the repeat modes: off plays the queue once, all loops it, one hammers the track. */
export function cycleRepeatMode(snapshot: PlaybackSnapshot): PlaybackSnapshot {
  const next =
    snapshot.repeatMode === undefined || snapshot.repeatMode === 'off'
      ? 'all'
      : snapshot.repeatMode === 'all'
        ? 'one'
        : 'off';
  return { ...snapshot, repeatMode: next };
}

/** Drop one line from the queue; a removed current line promotes the next. */
export function removeLine(snapshot: PlaybackSnapshot, trackId: string): PlaybackSnapshot {
  if (snapshot.trackId === undefined) {
    return snapshot;
  }
  const index = snapshot.queue.findIndex((line) => line.trackId === trackId);
  if (index < 0) {
    return snapshot;
  }
  const queue = snapshot.queue.filter((_, at) => at !== index);
  if (trackId !== snapshot.trackId) {
    return { ...snapshot, queue };
  }
  if (queue.length === 0) {
    return { ...emptyPlayback(), queueOpen: snapshot.queueOpen };
  }
  /* index sat in the old queue, so it has a neighbour in the new one. */
  const promoted = queue[Math.min(index, queue.length - 1)] as QueueLine;
  return {
    ...snapshot,
    trackId: promoted.trackId,
    albumId: promoted.albumId,
    title: promoted.title,
    artistName: promoted.artistName,
    coverTone: promoted.coverTone,
    coverUrl: promoted.coverUrl,
    mediaUrl: promoted.mediaUrl,
    positionMs: 0,
    durationMs: promoted.durationMs,
    lyricsKind: promoted.lyricsKind,
    playing: snapshot.playing,
    queue,
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
    queueOpen: false,
  };
}

export function insertPlayNext(snapshot: PlaybackSnapshot, line: QueueLine): PlaybackSnapshot {
  if (snapshot.trackId === undefined) {
    return playFromLine(line);
  }
  const index = snapshot.queue.findIndex((entry) => entry.trackId === snapshot.trackId);
  if (index < 0) {
    return { ...snapshot, queue: [...snapshot.queue, line] };
  }
  return {
    ...snapshot,
    queue: [...snapshot.queue.slice(0, index + 1), line, ...snapshot.queue.slice(index + 1)],
  };
}

export function appendQueue(snapshot: PlaybackSnapshot, line: QueueLine): PlaybackSnapshot {
  if (snapshot.trackId === undefined) {
    return playFromLine(line);
  }
  return { ...snapshot, queue: [...snapshot.queue, line] };
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

/**
 * The engine's loadedmetadata reported a track length. The snapshot takes
 * it when the catalogue had none to give (the file is the truth about the
 * file); a catalogue duration stays — the demo's fixture clock runs at
 * that scale on purpose.
 */
export function adoptDuration(snapshot: PlaybackSnapshot, reportedMs: number): PlaybackSnapshot {
  if (snapshot.trackId === undefined || snapshot.durationMs > 0 || !(reportedMs > 0)) {
    return snapshot;
  }
  return { ...snapshot, durationMs: Math.round(reportedMs) };
}

export function setQueueOpen(snapshot: PlaybackSnapshot, queueOpen: boolean): PlaybackSnapshot {
  return { ...snapshot, queueOpen };
}

/**
 * Starting playback never decides whether the queue is on screen, and it
 * never changes how the listener walks the queue: the sheet's state and
 * the transport choices (shuffle, its seed, repeat) carry over from the
 * play they replace. Only the queue itself is fresh.
 */
export function carryQueueOpen(previous: PlaybackSnapshot, next: PlaybackSnapshot): PlaybackSnapshot {
  return {
    ...next,
    queueOpen: previous.queueOpen,
    shuffleOn: previous.shuffleOn,
    shuffleSeed: previous.shuffleSeed,
    repeatMode: previous.repeatMode,
  };
}
