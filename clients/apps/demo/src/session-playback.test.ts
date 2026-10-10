import { expect, test } from 'vitest';
import { emptyPlayback, type PlaybackSnapshot, type QueueLine } from './playback.ts';
import { parseSessionPlayback, serializeSessionPlayback } from './session-playback.ts';

function line(trackId: string, albumId = 'demo-album-01'): QueueLine {
  return {
    trackId,
    albumId,
    title: 'Pier at Dusk',
    artistName: 'Mira Sol',
    coverTone: '01',
    coverUrl: '/media/covers/demo-album-01.svg',
    mediaUrl: '/media/library/demo-track-01-01',
    durationMs: 214_000,
    lyricsKind: 'none',
  };
}

function playing(positionMs: number): PlaybackSnapshot {
  const first = line('demo-track-01-01');
  const second = line('demo-track-01-02');
  return {
    ...emptyPlayback(),
    trackId: first.trackId,
    albumId: first.albumId,
    title: first.title,
    artistName: first.artistName,
    coverTone: first.coverTone,
    coverUrl: first.coverUrl,
    mediaUrl: first.mediaUrl,
    playing: true,
    positionMs,
    durationMs: first.durationMs,
    lyricsKind: 'none',
    queue: [first, second],
    queueOpen: true,
    shuffleOn: true,
    shuffleSeed: 7,
    repeatMode: 'all',
    buffering: true,
    playbackError: 'nope',
  };
}

test('nothing stored, or a value that is not a session, reads as an empty player', () => {
  expect(parseSessionPlayback(null)).toStrictEqual(emptyPlayback());
  expect(parseSessionPlayback('')).toStrictEqual(emptyPlayback());
  expect(parseSessionPlayback('not json')).toStrictEqual(emptyPlayback());
  expect(parseSessionPlayback('null')).toStrictEqual(emptyPlayback());
  expect(parseSessionPlayback('[]')).toStrictEqual(emptyPlayback());
  expect(parseSessionPlayback('{"queue":[]}')).toStrictEqual(emptyPlayback());
  expect(parseSessionPlayback('{"queue":[{"trackId":"x"}]}')).toStrictEqual(emptyPlayback());
});

test('a stored session restores the queue and the place in the song, and drops engine noise', () => {
  const restored = parseSessionPlayback(serializeSessionPlayback(playing(42_000)));
  expect(restored.trackId).toStrictEqual('demo-track-01-01');
  expect(restored.queue.map((entry) => entry.trackId)).toStrictEqual(['demo-track-01-01', 'demo-track-01-02']);
  expect(restored.positionMs).toStrictEqual(42_000);
  // A reload pauses. The place is kept; playback does not start itself.
  expect(restored.playing).toStrictEqual(false);
  expect(restored.shuffleOn).toStrictEqual(true);
  expect(restored.shuffleSeed).toStrictEqual(7);
  expect(restored.repeatMode).toStrictEqual('all');
  expect(restored.queueOpen).toStrictEqual(false);
  expect(restored.buffering).toStrictEqual(undefined);
  expect(restored.playbackError).toStrictEqual(undefined);
  expect(serializeSessionPlayback(playing(42_000))).not.toContain('nope');
  expect(serializeSessionPlayback(playing(42_000))).not.toContain('queueOpen');
});

test('a session that names another site, a missing track, or a broken line is dropped', () => {
  const offSite = playing(1_000);
  offSite.queue = [{ ...line('demo-track-01-01'), mediaUrl: 'https://evil.example/track' }];
  offSite.trackId = 'demo-track-01-01';
  offSite.mediaUrl = 'https://evil.example/track';
  expect(parseSessionPlayback(JSON.stringify(offSite))).toStrictEqual(emptyPlayback());

  const slash = playing(1_000);
  slash.queue = [{ ...line('demo-track-01-01'), mediaUrl: '//evil.example/track' }];
  expect(parseSessionPlayback(serializeSessionPlayback(slash))).toStrictEqual(emptyPlayback());

  const missing = playing(1_000);
  missing.trackId = 'not-in-queue';
  expect(parseSessionPlayback(JSON.stringify({ ...missing, queue: missing.queue }))).toStrictEqual(emptyPlayback());

  expect(serializeSessionPlayback(emptyPlayback())).toStrictEqual('{"queue":[]}');
});

test('a session without the optional fields restores, and empty cover art is allowed', () => {
  const minimal = {
    trackId: 'demo-track-01-01',
    positionMs: 1_000,
    queue: [{ ...line('demo-track-01-01'), coverUrl: '', durationMs: 0 }],
  };
  const restored = parseSessionPlayback(JSON.stringify(minimal));
  expect(restored.trackId).toStrictEqual('demo-track-01-01');
  // The queue line carries no duration; the stored place stands.
  expect(restored.positionMs).toStrictEqual(1_000);
  expect(restored.durationMs).toStrictEqual(0);
  expect(restored.queue[0]?.coverUrl).toStrictEqual('');
  expect(restored.shuffleSeed).toStrictEqual(0);
  expect(restored.repeatMode).toStrictEqual('off');
  expect(restored.shuffleOn).toStrictEqual(false);
});

test('a broken entry or a bad optional field drops the whole session', () => {
  const base = { trackId: 'demo-track-01-01', positionMs: 0, queue: [line('demo-track-01-01')] };
  expect(parseSessionPlayback(JSON.stringify({ ...base, queue: [null] }))).toStrictEqual(emptyPlayback());
  expect(parseSessionPlayback(JSON.stringify({ ...base, queue: [42] }))).toStrictEqual(emptyPlayback());
  expect(
    parseSessionPlayback(JSON.stringify({ ...base, queue: [{ ...line('demo-track-01-01'), trackId: 'a b' }] })),
  ).toStrictEqual(emptyPlayback());
  expect(
    parseSessionPlayback(JSON.stringify({ ...base, queue: [{ ...line('demo-track-01-01'), mediaUrl: '' }] })),
  ).toStrictEqual(emptyPlayback());
  expect(
    parseSessionPlayback(JSON.stringify({ ...base, queue: [{ ...line('demo-track-01-01'), lyricsKind: 'karaoke' }] })),
  ).toStrictEqual(emptyPlayback());
  // A missing place in the song is not a place.
  expect(parseSessionPlayback(JSON.stringify({ trackId: 'demo-track-01-01', queue: base.queue }))).toStrictEqual(
    emptyPlayback(),
  );
  expect(parseSessionPlayback(JSON.stringify({ ...base, repeatMode: 'sometimes' }))).toStrictEqual(emptyPlayback());
  expect(parseSessionPlayback(JSON.stringify({ ...base, shuffleSeed: 'x' }))).toStrictEqual(emptyPlayback());
});
