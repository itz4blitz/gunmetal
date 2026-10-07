import { expect, test } from 'vitest';
import { demoLibrary } from '../../../packages/fake-server/src/catalogue.ts';
import {
  albumsForArtist,
  appendAlbum,
  appendQueue,
  carryQueueOpen,
  emptyPlayback,
  findAlbum,
  findArtist,
  findTrack,
  insertAlbumNext,
  insertPlayNext,
  playbackFromAlbum,
  playbackFromTrack,
  playFromLine,
  queueLineFrom,
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
  expect(snapshot.lyricsKind).toStrictEqual('none');
  expect(snapshot.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-01-02',
    'demo-track-01-03',
    'demo-track-01-04',
  ]);
  // Playing does not put the queue on screen; only the queue control does.
  expect(snapshot.queueOpen).toStrictEqual(false);
});

test('playing skips unplayable and damaged fixture tracks when building the queue', () => {
  const library = demoLibrary();
  const album = findAlbum(library, 'demo-album-07');
  const snapshot = playbackFromAlbum(album!);
  expect(snapshot.queue.map((line) => line.trackId)).toStrictEqual(['demo-track-07-01', 'demo-track-07-04']);
  expect(snapshot.lyricsKind).toStrictEqual('synced');
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
  expect(playbackFromTrack(mid!.album, mid!.track).lyricsKind).toStrictEqual('synced');
  expect(emptyPlayback().trackId).toStrictEqual(undefined);
  expect(emptyPlayback().lyricsKind).toStrictEqual('none');
  expect(playbackFromAlbum({ ...album!, tracks: [] }).trackId).toStrictEqual(undefined);
});

test('prev next and pause only step the demo-local queue snapshot', () => {
  const library = demoLibrary();
  let snapshot = playbackFromAlbum(findAlbum(library, 'demo-album-02')!);
  snapshot = stepQueue(snapshot, 1);
  expect(snapshot.trackId).toStrictEqual('demo-track-02-02');
  expect(snapshot.lyricsKind).toStrictEqual('synced');
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
  expect(findArtist(library, 'mira-sol')?.name).toStrictEqual('Mira Sol');
  expect(findArtist(library, 'missing')).toStrictEqual(undefined);
  expect(albumsForArtist(library, findArtist(library, 'mira-sol')!).map((album) => album.id)).toStrictEqual([
    'demo-album-01',
    'demo-album-02',
  ]);
  expect(
    albumsForArtist(library, { key: 'ghost', name: 'Ghost', albumIds: ['missing', 'demo-album-01'] }).map(
      (album) => album.id,
    ),
  ).toStrictEqual(['demo-album-01']);
  expect(findTrack(library, 'missing')).toStrictEqual(undefined);
  const orphan = { ...snapshot, trackId: 'missing', queue: snapshot.queue };
  expect(stepQueue(orphan, 1).trackId).toStrictEqual('missing');
  const holeQueue = [snapshot.queue[0]!];
  holeQueue.length = 3;
  const holed = { ...snapshot, trackId: snapshot.queue[0]!.trackId, queue: holeQueue };
  expect(stepQueue(holed, 1).trackId).toStrictEqual(snapshot.queue[0]!.trackId);
});

test('play next inserts after the current row and add to queue only appends', () => {
  const library = demoLibrary();
  const harbour = findAlbum(library, 'demo-album-01')!;
  const night = findAlbum(library, 'demo-album-02')!;
  const elevator = findTrack(library, 'demo-track-02-02')!;
  expect(queueLineFrom(elevator.album, elevator.track)).toStrictEqual({
    trackId: 'demo-track-02-02',
    albumId: 'demo-album-02',
    title: 'Freight Elevator',
    artistName: 'Mira Sol',
    coverTone: '02',
    coverUrl: '/media/covers/demo-album-02.svg',
    mediaUrl: '/media/audio/demo-album-02.wav',
    durationMs: 232_000,
    lyricsKind: 'synced',
  });

  let snapshot = playbackFromAlbum(harbour);
  snapshot = insertAlbumNext(snapshot, night);
  expect(snapshot.trackId).toStrictEqual('demo-track-01-01');
  expect(snapshot.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-02-01',
    'demo-track-02-02',
    'demo-track-02-03',
    'demo-track-01-02',
    'demo-track-01-03',
    'demo-track-01-04',
  ]);
  snapshot = appendQueue(snapshot, queueLineFrom(elevator.album, elevator.track));
  expect(snapshot.queue.map((line) => line.trackId).at(-1)).toStrictEqual('demo-track-02-02');
  expect(snapshot.trackId).toStrictEqual('demo-track-01-01');

  const emptyAlbum = { ...harbour, tracks: harbour.tracks.filter((track) => track.flag === 'unplayable') };
  expect(insertAlbumNext(snapshot, emptyAlbum).queue).toStrictEqual(snapshot.queue);
  expect(appendAlbum(snapshot, emptyAlbum).queue).toStrictEqual(snapshot.queue);

  const started = insertAlbumNext(emptyPlayback(), harbour);
  expect(started.trackId).toStrictEqual('demo-track-01-01');
  expect(started.playing).toStrictEqual(true);
  const appendedIdle = appendAlbum(emptyPlayback(), night);
  expect(appendedIdle.trackId).toStrictEqual('demo-track-02-01');
  expect(appendedIdle.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-02-01',
    'demo-track-02-02',
    'demo-track-02-03',
  ]);

  const afterAppend = appendAlbum(snapshot, night);
  expect(afterAppend.queue.map((line) => line.trackId).slice(-3)).toStrictEqual([
    'demo-track-02-01',
    'demo-track-02-02',
    'demo-track-02-03',
  ]);

  const orphan = { ...snapshot, trackId: 'missing' };
  const afterOrphan = insertPlayNext(orphan, queueLineFrom(harbour, harbour.tracks[0]!));
  expect(afterOrphan.queue.map((line) => line.trackId).at(-1)).toStrictEqual('demo-track-01-01');
  // Queuing leaves the sheet as it was: shut stays shut, open stays open.
  expect(afterOrphan.queueOpen).toStrictEqual(false);
  const shown = { ...snapshot, queueOpen: true };
  const line = snapshot.queue.find((entry) => entry.trackId === 'demo-track-01-02');
  if (line === undefined) {
    throw new Error('the fixture queue lost its second line');
  }
  expect(insertPlayNext(shown, line).queueOpen).toStrictEqual(true);
  expect(insertPlayNext({ ...shown, trackId: 'missing' }, line).queueOpen).toStrictEqual(true);
  expect(appendQueue(shown, line).queueOpen).toStrictEqual(true);
  expect(insertPlayNext(snapshot, line).queueOpen).toStrictEqual(false);
  expect(appendQueue(snapshot, line).queueOpen).toStrictEqual(false);

  const idleLine = playFromLine(queueLineFrom(harbour, harbour.tracks[2]!));
  expect(idleLine.trackId).toStrictEqual('demo-track-01-03');
  expect(idleLine.lyricsKind).toStrictEqual('plain');
  expect(insertPlayNext(emptyPlayback(), idleLine.queue[0]!).trackId).toStrictEqual('demo-track-01-03');
  expect(appendQueue(emptyPlayback(), idleLine.queue[0]!).queue).toStrictEqual(idleLine.queue);
});

test('a new play keeps the queue sheet as the listener left it', () => {
  const library = demoLibrary();
  const harbour = findAlbum(library, 'demo-album-01');
  const night = findAlbum(library, 'demo-album-02');
  if (harbour === undefined || night === undefined) {
    throw new Error('the fixture library lost an album');
  }
  const playing = playbackFromAlbum(harbour);
  const next = playbackFromAlbum(night);
  // Shut stays shut.
  const stillShut = carryQueueOpen(playing, next);
  expect(stillShut.queueOpen).toStrictEqual(false);
  expect(stillShut.trackId).toStrictEqual('demo-track-02-01');
  // Open stays open, and nothing else is carried over from the old snapshot.
  const stillOpen = carryQueueOpen(setQueueOpen(playing, true), next);
  expect(stillOpen).toStrictEqual({ ...next, queueOpen: true });
  expect(stillOpen.albumId).toStrictEqual('demo-album-02');
  // The inputs are not changed.
  expect(next.queueOpen).toStrictEqual(false);
});
