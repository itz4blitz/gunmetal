import { expect, test } from 'vitest';
import { demoLibrary } from '../../../packages/fake-server/src/catalogue.ts';
import {
  applyAlbumQueue,
  applyPlayback,
  applyTrackQueue,
  resolveAlbumPlayback,
  resolveTrackPlayback,
} from './demo-play.ts';
import { emptyPlayback, playbackFromAlbum, findAlbum } from './playback.ts';

test('resolve playback helpers refuse missing library albums and mismatched tracks', () => {
  const library = demoLibrary();
  expect(resolveAlbumPlayback(undefined, 'demo-album-01')).toStrictEqual(undefined);
  expect(resolveAlbumPlayback(library, 'missing')).toStrictEqual(undefined);
  const album = resolveAlbumPlayback(library, 'demo-album-01');
  expect(album?.trackId).toStrictEqual('demo-track-01-01');
  expect(resolveTrackPlayback(undefined, 'demo-album-01', 'demo-track-01-01')).toStrictEqual(undefined);
  expect(resolveTrackPlayback(library, 'demo-album-01', 'missing')).toStrictEqual(undefined);
  expect(resolveTrackPlayback(library, 'wrong-album', 'demo-track-01-01')).toStrictEqual(undefined);
  expect(resolveTrackPlayback(library, 'demo-album-01', 'demo-track-01-02')?.trackId).toStrictEqual('demo-track-01-02');
  expect(applyPlayback(library, 'demo-album-02', undefined).title).toStrictEqual('Clock In');
  expect(applyPlayback(library, 'demo-album-02', 'demo-track-02-02').title).toStrictEqual('Freight Elevator');
  expect(applyPlayback(undefined, 'demo-album-01', undefined).trackId).toStrictEqual(undefined);
  expect(applyPlayback(library, 'missing', 'missing').trackId).toStrictEqual(undefined);
});

test('demo-local queue helpers insert or append without shuffling', () => {
  const library = demoLibrary();
  const idle = emptyPlayback();
  expect(applyAlbumQueue(undefined, idle, 'demo-album-01', 'next').trackId).toStrictEqual(undefined);
  expect(applyAlbumQueue(library, idle, 'missing', 'append').trackId).toStrictEqual(undefined);
  const started = applyAlbumQueue(library, idle, 'demo-album-01', 'next');
  expect(started.trackId).toStrictEqual('demo-track-01-01');
  const afterNight = applyAlbumQueue(library, started, 'demo-album-02', 'next');
  expect(afterNight.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-02-01',
    'demo-track-02-02',
    'demo-track-02-03',
    'demo-track-01-02',
    'demo-track-01-03',
    'demo-track-01-04',
  ]);
  const appended = applyAlbumQueue(library, started, 'demo-album-02', 'append');
  expect(appended.queue.map((line) => line.trackId).slice(-3)).toStrictEqual([
    'demo-track-02-01',
    'demo-track-02-02',
    'demo-track-02-03',
  ]);
  expect(applyTrackQueue(undefined, started, 'demo-album-02', 'demo-track-02-02', 'next').queue).toStrictEqual(
    started.queue,
  );
  expect(applyTrackQueue(library, started, 'demo-album-01', 'missing', 'next').queue).toStrictEqual(started.queue);
  expect(applyTrackQueue(library, started, 'wrong-album', 'demo-track-02-02', 'append').queue).toStrictEqual(
    started.queue,
  );
  const nextTrack = applyTrackQueue(library, started, 'demo-album-02', 'demo-track-02-02', 'next');
  expect(nextTrack.queue.map((line) => line.trackId)[1]).toStrictEqual('demo-track-02-02');
  const appendTrack = applyTrackQueue(library, started, 'demo-album-02', 'demo-track-02-02', 'append');
  expect(appendTrack.queue.map((line) => line.trackId).at(-1)).toStrictEqual('demo-track-02-02');
  expect(playbackFromAlbum(findAlbum(library, 'demo-album-01')!).trackId).toStrictEqual('demo-track-01-01');
});
