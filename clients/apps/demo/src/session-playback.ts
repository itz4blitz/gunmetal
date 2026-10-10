/**
 * The playing session, kept only for this tab.
 *
 * A refresh unloads the audio element. The queue and the place in the song
 * have to come back, or the visit ends. This is Activity, so it does not go
 * in localStorage, IndexedDB or Cache Storage (SEC-PRV-019). The composition
 * root holds it in sessionStorage, which the tab drops when it closes.
 * A stored value is never trusted. This module does not touch storage.
 */

import { emptyPlayback, type PlaybackSnapshot, type QueueLine } from './playback.ts';

export type SessionPlaybackStore = {
  read(): string | null;
  write(value: string): void;
};

const MAX_QUEUE = 200;
const MAX_TEXT = 200;
const MAX_ID = 128;
const MAX_POSITION_MS = 86_400_000;

function isId(value: string): boolean {
  return value.length > 0 && value.length <= MAX_ID && /^[A-Za-z0-9._:-]+$/.test(value);
}

function isText(value: string): boolean {
  return value.length <= MAX_TEXT && !value.includes('\0');
}

/** A same-origin path. A stored session must not name another site. */
function isSameOriginPath(value: string): boolean {
  if (value === '') {
    return true;
  }
  if (!value.startsWith('/') || value.startsWith('//') || value.length > 512) {
    return false;
  }
  return !value.includes('\\') && !value.includes('..') && !value.includes('\0');
}

function isLyricsKind(value: string): value is QueueLine['lyricsKind'] {
  return value === 'none' || value === 'plain' || value === 'synced';
}

function isRepeat(value: string): value is 'off' | 'all' | 'one' {
  return value === 'off' || value === 'all' || value === 'one';
}

function lineFrom(entry: unknown): QueueLine | undefined {
  if (typeof entry !== 'object' || entry === null) {
    return undefined;
  }
  const record = entry as Record<string, unknown>;
  const trackId = record.trackId;
  const albumId = record.albumId;
  const title = record.title;
  const artistName = record.artistName;
  const coverTone = record.coverTone;
  const coverUrl = record.coverUrl;
  const mediaUrl = record.mediaUrl;
  const durationMs = record.durationMs;
  const lyricsKind = record.lyricsKind;
  if (
    typeof trackId !== 'string' ||
    typeof albumId !== 'string' ||
    typeof title !== 'string' ||
    typeof artistName !== 'string' ||
    typeof coverTone !== 'string' ||
    typeof coverUrl !== 'string' ||
    typeof mediaUrl !== 'string' ||
    typeof durationMs !== 'number' ||
    typeof lyricsKind !== 'string'
  ) {
    return undefined;
  }
  if (!isId(trackId) || !isId(albumId) || !isText(title) || !isText(artistName) || !isText(coverTone)) {
    return undefined;
  }
  if (!isSameOriginPath(coverUrl) || !isSameOriginPath(mediaUrl) || mediaUrl === '') {
    return undefined;
  }
  if (!isLyricsKind(lyricsKind) || !Number.isFinite(durationMs) || durationMs < 0 || durationMs > MAX_POSITION_MS) {
    return undefined;
  }
  return {
    trackId,
    albumId,
    title,
    artistName,
    coverTone,
    coverUrl,
    mediaUrl,
    durationMs,
    lyricsKind,
  };
}

function positionOf(value: unknown): number | undefined {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0 || value > MAX_POSITION_MS) {
    return undefined;
  }
  return value;
}

/** Read one stored session. Anything unreadable is an empty player, not an error. */
export function parseSessionPlayback(raw: string | null): PlaybackSnapshot {
  if (raw === null || raw === '') {
    return emptyPlayback();
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return emptyPlayback();
  }
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    return emptyPlayback();
  }
  const record = parsed as Record<string, unknown>;
  if (!Array.isArray(record.queue) || record.queue.length === 0 || record.queue.length > MAX_QUEUE) {
    return emptyPlayback();
  }
  const queue: QueueLine[] = [];
  for (const entry of record.queue) {
    const line = lineFrom(entry);
    if (line === undefined) {
      return emptyPlayback();
    }
    queue.push(line);
  }
  const trackId = record.trackId;
  if (typeof trackId !== 'string' || !queue.some((line) => line.trackId === trackId)) {
    return emptyPlayback();
  }
  const current = queue.find((line) => line.trackId === trackId) as QueueLine;
  const positionMs = positionOf(record.positionMs);
  const repeatMode = record.repeatMode;
  const shuffleSeed = record.shuffleSeed;
  if (positionMs === undefined) {
    return emptyPlayback();
  }
  if (repeatMode !== undefined && (typeof repeatMode !== 'string' || !isRepeat(repeatMode))) {
    return emptyPlayback();
  }
  if (shuffleSeed !== undefined && (typeof shuffleSeed !== 'number' || !Number.isFinite(shuffleSeed))) {
    return emptyPlayback();
  }
  return {
    ...emptyPlayback(),
    trackId: current.trackId,
    albumId: current.albumId,
    title: current.title,
    artistName: current.artistName,
    coverTone: current.coverTone,
    coverUrl: current.coverUrl,
    mediaUrl: current.mediaUrl,
    playing: false,
    positionMs: Math.min(positionMs, current.durationMs > 0 ? current.durationMs : positionMs),
    durationMs: current.durationMs,
    lyricsKind: current.lyricsKind,
    queue,
    queueOpen: false,
    shuffleOn: record.shuffleOn === true,
    shuffleSeed: typeof shuffleSeed === 'number' ? shuffleSeed : 0,
    repeatMode: typeof repeatMode === 'string' && isRepeat(repeatMode) ? repeatMode : 'off',
  };
}

/** Write the visit's player. Transient engine fields stay out. */
export function serializeSessionPlayback(snapshot: PlaybackSnapshot): string {
  if (snapshot.queue.length === 0 || snapshot.trackId === undefined) {
    return '{"queue":[]}';
  }
  return JSON.stringify({
    trackId: snapshot.trackId,
    playing: snapshot.playing,
    positionMs: snapshot.positionMs,
    shuffleOn: snapshot.shuffleOn === true,
    shuffleSeed: snapshot.shuffleSeed ?? 0,
    repeatMode: snapshot.repeatMode ?? 'off',
    queue: snapshot.queue.map((line) => ({
      trackId: line.trackId,
      albumId: line.albumId,
      title: line.title,
      artistName: line.artistName,
      coverTone: line.coverTone,
      coverUrl: line.coverUrl,
      mediaUrl: line.mediaUrl,
      durationMs: line.durationMs,
      lyricsKind: line.lyricsKind,
    })),
  });
}
