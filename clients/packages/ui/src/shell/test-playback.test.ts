import { expect, test } from 'vitest';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';
import { emptySnapshot, queuedSnapshot, stubPlayback, transportSnapshot } from './test-playback.ts';

test('the stub records every verb in the order the test drove it', () => {
  const { controller, calls } = stubPlayback();
  controller.setVolume(0.5);
  controller.setMuted(true);
  controller.toggleShuffle();
  controller.cycleRepeat();
  controller.removeQueueLine('demo-track-01-02');
  controller.playAlbum('demo-album-01');
  controller.playTrack('demo-album-01', 'demo-track-01-02');
  controller.playNextAlbum('demo-album-02');
  controller.addAlbumToQueue('demo-album-02');
  controller.playNextTrack('demo-album-01', 'demo-track-01-03');
  controller.addTrackToQueue('demo-album-01', 'demo-track-01-04');
  controller.playPause();
  controller.previous();
  controller.next();
  controller.seek(45_000);
  controller.toggleQueue();
  controller.closeQueue();
  controller.openFull();
  controller.closeFull();
  expect(calls).toStrictEqual([
    'setVolume',
    'setMuted',
    'toggleShuffle',
    'cycleRepeat',
    'removeQueueLine demo-track-01-02',
    'playAlbum',
    'playTrack',
    'playNextAlbum',
    'addAlbumToQueue',
    'playNextTrack',
    'addTrackToQueue',
    'playPause',
    'previous',
    'next',
    'seek',
    'toggleQueue',
    'closeQueue',
    'openFull',
    'closeFull',
  ]);
});

test('the stub holds the snapshot the test wrote down, or the empty one', () => {
  const idle = stubPlayback();
  expect(idle.controller.state.trackId).toStrictEqual(undefined);
  expect(idle.controller.fullOpen).toStrictEqual(false);
  expect(idle.controller.volume).toStrictEqual(0.8);
  expect(idle.controller.muted).toStrictEqual(false);
  expect(idle.controller.clock).toStrictEqual(undefined);
  const snapshot = queuedSnapshot();
  const wired = stubPlayback(snapshot);
  expect(wired.controller.state).toStrictEqual(snapshot);
});

test('the empty snapshot is a literal idle player', () => {
  expect(emptySnapshot()).toStrictEqual({
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
  });
});

test('the queued snapshot is the fixture album at an index, or idle past its end', () => {
  const snapshot = queuedSnapshot(1);
  expect(snapshot.trackId).toStrictEqual('demo-track-01-02');
  expect(snapshot.title).toStrictEqual('Salt Window');
  expect(snapshot.playing).toStrictEqual(true);
  expect(snapshot.positionMs).toStrictEqual(45_000);
  expect(snapshot.durationMs).toStrictEqual(198_000);
  expect(snapshot.queueOpen).toStrictEqual(true);
  expect(snapshot.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-01-02',
    'demo-track-01-03',
    'demo-track-01-04',
  ]);
  expect(queuedSnapshot().trackId).toStrictEqual('demo-track-01-01');
  expect(queuedSnapshot(9)).toStrictEqual(emptySnapshot());
});

test('the transport snapshot carries the shuffle, repeat and engine state', () => {
  const snapshot = transportSnapshot();
  expect(snapshot.shuffleOn).toStrictEqual(true);
  expect(snapshot.shuffleSeed).toStrictEqual(42);
  expect(snapshot.repeatMode).toStrictEqual('all');
  expect(snapshot.buffering).toStrictEqual(false);
  expect(snapshot.playbackError).toStrictEqual(undefined);
  // Everything else is the queued snapshot it grew from.
  const queued: PlayerSnapshot = queuedSnapshot();
  expect({
    ...snapshot,
    shuffleOn: undefined,
    shuffleSeed: undefined,
    repeatMode: undefined,
    buffering: undefined,
  }).toStrictEqual({
    ...queued,
    shuffleOn: undefined,
    shuffleSeed: undefined,
    repeatMode: undefined,
    buffering: undefined,
  });
});
