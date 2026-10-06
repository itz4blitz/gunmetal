import { expect, test } from 'vitest';
import { demoLibrary } from '../../../fake-server/src/catalogue.ts';
import {
  emptyPlayback,
  findAlbum,
  findTrack,
  playbackFromAlbum,
  playbackFromTrack,
  setQueueOpen,
  stepQueue,
  togglePlaying,
} from './playback.ts';

test('playing an album fills the bar from the first playable fixture track', () => {
  const library = demoLibrary();
  const album = findAlbum(library, 'demo-album-01');
  expect(album).toBeDefined();
  const snapshot = playbackFromAlbum(album!);
  expect(snapshot.playing).toStrictEqual(true);
  expect(snapshot.trackId).toStrictEqual('demo-track-01-01');
  expect(snapshot.title).toStrictEqual('Pier at Dusk');
  expect(snapshot.artistName).toStrictEqual('Mira Sol');
  expect(snapshot.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-01-02',
    'demo-track-01-03',
    'demo-track-01-04',
  ]);
  expect(snapshot.queueOpen).toStrictEqual(true);
});

test('playing skips unplayable and damaged fixture tracks when building the queue', () => {
  const library = demoLibrary();
  const album = findAlbum(library, 'demo-album-07');
  const snapshot = playbackFromAlbum(album!);
  expect(snapshot.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-07-01',
    'demo-track-07-04',
  ]);
  const fromDamaged = findTrack(library, 'demo-track-07-03');
  const rotated = playbackFromTrack(fromDamaged!.album, fromDamaged!.track);
  expect(rotated.trackId).toStrictEqual('demo-track-07-01');
  const onlyDamaged = {
    ...album!,
    tracks: album!.tracks.filter((track) => track.flag === 'damaged'),
  };
  const fallback = playbackFromTrack(onlyDamaged, onlyDamaged.tracks[0]!);
  expect(fallback.trackId).toStrictEqual('demo-track-07-03');
  expect(fallback.queue).toStrictEqual([]);
  const mid = findTrack(library, 'demo-track-02-02');
  expect(playbackFromTrack(mid!.album, mid!.track).queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-02-02',
    'demo-track-02-03',
    'demo-track-02-01',
  ]);
  expect(emptyPlayback().trackId).toStrictEqual(undefined);
  expect(playbackFromAlbum({ ...album!, tracks: [] }).trackId).toStrictEqual(undefined);
});

test('prev next and pause only step the demo-local queue snapshot', () => {
  const library = demoLibrary();
  let snapshot = playbackFromAlbum(findAlbum(library, 'demo-album-02')!);
  snapshot = stepQueue(snapshot, 1);
  expect(snapshot.trackId).toStrictEqual('demo-track-02-02');
  snapshot = stepQueue(snapshot, -1);
  expect(snapshot.trackId).toStrictEqual('demo-track-02-01');
  snapshot = togglePlaying(snapshot);
  expect(snapshot.playing).toStrictEqual(false);
  snapshot = setQueueOpen(snapshot, false);
  expect(snapshot.queueOpen).toStrictEqual(false);
  const ended = stepQueue(stepQueue(stepQueue(snapshot, 1), 1), 1);
  expect(ended.playing).toStrictEqual(false);
  expect(stepQueue(emptyPlayback(), 1).trackId).toStrictEqual(undefined);
  expect(togglePlaying(emptyPlayback()).playing).toStrictEqual(false);
  expect(findAlbum(library, 'missing')).toStrictEqual(undefined);
  expect(findTrack(library, 'missing')).toStrictEqual(undefined);
  const orphan = { ...snapshot, trackId: 'missing', queue: snapshot.queue };
  expect(stepQueue(orphan, 1).trackId).toStrictEqual('missing');
  const holeQueue = [snapshot.queue[0]!];
  holeQueue.length = 3;
  const holed = { ...snapshot, trackId: snapshot.queue[0]!.trackId, queue: holeQueue };
  expect(stepQueue(holed, 1).trackId).toStrictEqual(snapshot.queue[0]!.trackId);
});
