import { expect, test } from 'vitest';
import { demoLibrary } from '../../../packages/fake-server/src/catalogue.ts';

/** The fixture value a test names, or a loud failure — never an asserted maybe. */
function present<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) {
    throw new Error(`${what} is missing from the fixture library`);
  }
  return value;
}
import {
  albumsForArtist,
  advanceQueue,
  adoptDuration,
  appendAlbum,
  appendQueue,
  carryQueueOpen,
  cycleRepeatMode,
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
  removeLine,
  setQueueOpen,
  shuffledIndices,
  stepQueue,
  togglePlaying,
  toggleShuffleMode,
} from './playback.ts';

test('playing an album fills the bar from the first playable fixture track', () => {
  const library = demoLibrary();
  const album = present(findAlbum(library, 'demo-album-01'), 'demo-album-01');
  const snapshot = playbackFromAlbum(album);
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
  const album = present(findAlbum(library, 'demo-album-07'), 'demo-album-07');
  const snapshot = playbackFromAlbum(album);
  expect(snapshot.queue.map((line) => line.trackId)).toStrictEqual(['demo-track-07-01', 'demo-track-07-04']);
  expect(snapshot.lyricsKind).toStrictEqual('synced');
  const fromDamaged = findTrack(library, 'demo-track-07-03');
  const damaged = present(fromDamaged, 'demo-track-07-03');
  const rotated = playbackFromTrack(damaged.album, damaged.track);
  expect(rotated.trackId).toStrictEqual('demo-track-07-01');
  const onlyDamaged = {
    ...album,
    tracks: album.tracks.filter((track) => track.flag === 'damaged'),
  };
  const fallback = playbackFromTrack(onlyDamaged, present(onlyDamaged.tracks[0], 'the damaged track'));
  expect(fallback.trackId).toStrictEqual('demo-track-07-03');
  expect(fallback.queue).toStrictEqual([]);
  const mid = findTrack(library, 'demo-track-02-02');
  const midnight = present(mid, 'demo-track-02-02');
  expect(playbackFromTrack(midnight.album, midnight.track).queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-02-02',
    'demo-track-02-03',
    'demo-track-02-01',
  ]);
  expect(playbackFromTrack(midnight.album, midnight.track).lyricsKind).toStrictEqual('synced');
  expect(emptyPlayback().trackId).toStrictEqual(undefined);
  expect(emptyPlayback().lyricsKind).toStrictEqual('none');
  expect(playbackFromAlbum({ ...album, tracks: [] }).trackId).toStrictEqual(undefined);
});

test('prev next and pause only step the demo-local queue snapshot', () => {
  const library = demoLibrary();
  let snapshot = playbackFromAlbum(present(findAlbum(library, 'demo-album-02'), 'demo-album-02'));
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
  expect(
    albumsForArtist(library, present(findArtist(library, 'mira-sol'), 'mira-sol')).map((album) => album.id),
  ).toStrictEqual(['demo-album-01', 'demo-album-02']);
  expect(
    albumsForArtist(library, { key: 'ghost', name: 'Ghost', albumIds: ['missing', 'demo-album-01'] }).map(
      (album) => album.id,
    ),
  ).toStrictEqual(['demo-album-01']);
  expect(findTrack(library, 'missing')).toStrictEqual(undefined);
  const orphan = { ...snapshot, trackId: 'missing', queue: snapshot.queue };
  expect(stepQueue(orphan, 1).trackId).toStrictEqual('missing');
  const holeQueue = [present(snapshot.queue[0], 'the holed queue head')];
  holeQueue.length = 3;
  const holed = { ...snapshot, trackId: present(snapshot.queue[0], 'the holed queue head').trackId, queue: holeQueue };
  expect(stepQueue(holed, 1).trackId).toStrictEqual(holed.trackId);
});

test('play next inserts after the current row and add to queue only appends', () => {
  const library = demoLibrary();
  const harbour = present(findAlbum(library, 'demo-album-01'), 'demo-album-01');
  const night = present(findAlbum(library, 'demo-album-02'), 'demo-album-02');
  const elevator = present(findTrack(library, 'demo-track-02-02'), 'demo-track-02-02');
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
  const afterOrphan = insertPlayNext(
    orphan,
    queueLineFrom(harbour, present(harbour.tracks[0], 'the harbour opening track')),
  );
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

  const idleLine = playFromLine(queueLineFrom(harbour, present(harbour.tracks[2], 'the harbour third track')));
  expect(idleLine.trackId).toStrictEqual('demo-track-01-03');
  expect(idleLine.lyricsKind).toStrictEqual('plain');
  expect(insertPlayNext(emptyPlayback(), present(idleLine.queue[0], 'the single line')).trackId).toStrictEqual(
    'demo-track-01-03',
  );
  expect(appendQueue(emptyPlayback(), present(idleLine.queue[0], 'the single line')).queue).toStrictEqual(
    idleLine.queue,
  );
});

test('a new play keeps the queue sheet as the listener left it, and the transport choices too', () => {
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
  // Open stays open, shuffle and repeat survive, and nothing else is carried
  // over from the old snapshot.
  const choices = { ...setQueueOpen(playing, true), shuffleOn: true, shuffleSeed: 7, repeatMode: 'one' as const };
  const stillOpen = carryQueueOpen(choices, next);
  expect(stillOpen).toStrictEqual({
    ...next,
    queueOpen: true,
    shuffleOn: true,
    shuffleSeed: 7,
    repeatMode: 'one',
  });
  expect(stillOpen.albumId).toStrictEqual('demo-album-02');
  // A fresh play without choices carries none.
  expect(carryQueueOpen(playing, next)).toStrictEqual({
    ...next,
    queueOpen: false,
    shuffleOn: undefined,
    shuffleSeed: undefined,
    repeatMode: undefined,
  });
  // The inputs are not changed.
  expect(next.queueOpen).toStrictEqual(false);
});

test('shuffledIndices is a seeded permutation: same seed, same order', () => {
  expect(shuffledIndices(4, 1)).toStrictEqual([1, 2, 0, 3]);
  expect(shuffledIndices(4, 1)).toStrictEqual(shuffledIndices(4, 1));
  expect(shuffledIndices(4, 42)).toStrictEqual([3, 1, 0, 2]);
  expect(shuffledIndices(1, 7)).toStrictEqual([0]);
  expect(shuffledIndices(0, 7)).toStrictEqual([]);
  // Seeds normalise into the LCG's range instead of breaking it.
  expect(shuffledIndices(4, 0)).toStrictEqual([3, 2, 1, 0]);
  expect(shuffledIndices(4, 2147483647)).toStrictEqual([3, 2, 1, 0]);
  expect(shuffledIndices(4, -3)).toStrictEqual([0, 2, 1, 3]);
  // A permutation: every index exactly once.
  expect([...shuffledIndices(8, 5)].sort((a, b) => a - b)).toStrictEqual([0, 1, 2, 3, 4, 5, 6, 7]);
});

test('toggling shuffle stores the seed on and drops the order off', () => {
  const library = demoLibrary();
  const snapshot = playbackFromAlbum(present(findAlbum(library, 'demo-album-01'), 'demo-album-01'));
  expect(snapshot.shuffleOn).toStrictEqual(undefined);
  const on = toggleShuffleMode(snapshot, 42);
  expect(on.shuffleOn).toStrictEqual(true);
  expect(on.shuffleSeed).toStrictEqual(42);
  expect(on.queue).toStrictEqual(snapshot.queue);
  // Toggling again forgets the order but keeps the last seed for the readout.
  const off = toggleShuffleMode(on, 7);
  expect(off.shuffleOn).toStrictEqual(false);
  expect(off.shuffleSeed).toStrictEqual(7);
  expect(off.queue).toStrictEqual(snapshot.queue);
});

test('cycleRepeat walks off, all, one and back to off', () => {
  const library = demoLibrary();
  const snapshot = playbackFromAlbum(present(findAlbum(library, 'demo-album-01'), 'demo-album-01'));
  expect(cycleRepeatMode(snapshot).repeatMode).toStrictEqual('all');
  expect(cycleRepeatMode(cycleRepeatMode(snapshot)).repeatMode).toStrictEqual('one');
  expect(cycleRepeatMode(cycleRepeatMode(cycleRepeatMode(snapshot))).repeatMode).toStrictEqual('off');
  // Nothing else moves.
  expect(cycleRepeatMode(snapshot)).toStrictEqual({ ...snapshot, repeatMode: 'all' });
});

test('with shuffle on, next and previous follow the seeded order', () => {
  const library = demoLibrary();
  const album = present(findAlbum(library, 'demo-album-01'), 'demo-album-01');
  const shuffled = { ...playbackFromAlbum(album), shuffleOn: true, shuffleSeed: 1 };
  // seed 1 over four lines orders [1, 2, 0, 3]: Salt Window, Low Tide,
  // Pier, Beacon. From Pier the next in the order is Beacon, the one
  // before it is Low Tide Letter.
  expect(stepQueue(shuffled, 1).trackId).toStrictEqual('demo-track-01-04');
  expect(stepQueue(shuffled, -1).trackId).toStrictEqual('demo-track-01-03');
  const fromSalt = { ...shuffled, trackId: 'demo-track-01-02' };
  expect(stepQueue(fromSalt, 1).trackId).toStrictEqual('demo-track-01-03');
  // The head of the shuffle order still stops when repeat is off.
  expect(stepQueue(fromSalt, -1).playing).toStrictEqual(false);
  expect(stepQueue(fromSalt, -1).trackId).toStrictEqual('demo-track-01-02');
  // Position, duration and lyrics ride along with the line.
  const next = stepQueue(shuffled, 1);
  expect(next.positionMs).toStrictEqual(0);
  expect(next.durationMs).toStrictEqual(187_000);
  // Shuffle off (the default): the queue's own order, as before.
  const ordered = stepQueue(playbackFromAlbum(album), 1);
  expect(ordered.trackId).toStrictEqual('demo-track-01-02');
  // A shuffle with no stored seed walks the seed-0 order.
  const unseeded = { ...playbackFromAlbum(album), shuffleOn: true };
  expect(stepQueue(unseeded, -1).trackId).toStrictEqual('demo-track-01-02');
  expect(stepQueue(unseeded, 1).playing).toStrictEqual(false);
});

test('shuffle keeps its order as the queue plays through and repeats all', () => {
  const library = demoLibrary();
  const album = present(findAlbum(library, 'demo-album-01'), 'demo-album-01');
  let snapshot: ReturnType<typeof playbackFromAlbum> = {
    ...playbackFromAlbum(album),
    shuffleOn: true,
    shuffleSeed: 1,
    repeatMode: 'all',
  };
  const seen: string[] = [present(snapshot.trackId, 'the walking track id')];
  for (let step = 0; step < 5; step += 1) {
    snapshot = stepQueue(snapshot, 1);
    seen.push(present(snapshot.trackId, 'the walking track id'));
  }
  // [1,2,0,3] plays Salt, Low Tide, Pier, Beacon, then wraps to Salt again.
  expect(seen).toStrictEqual([
    'demo-track-01-01',
    'demo-track-01-04',
    'demo-track-01-02',
    'demo-track-01-03',
    'demo-track-01-01',
    'demo-track-01-04',
  ]);
});

test('repeat all wraps next and previous past the queue ends', () => {
  const library = demoLibrary();
  const album = present(findAlbum(library, 'demo-album-01'), 'demo-album-01');
  const all = { ...playbackFromAlbum(album), repeatMode: 'all' as const };
  const last = { ...all, trackId: 'demo-track-01-04' };
  expect(stepQueue(last, 1).trackId).toStrictEqual('demo-track-01-01');
  expect(stepQueue(all, -1).trackId).toStrictEqual('demo-track-01-04');
  // Without repeat the ends stop, as they always did.
  expect(stepQueue({ ...playbackFromAlbum(album), trackId: 'demo-track-01-04' }, 1).playing).toStrictEqual(false);
  expect(stepQueue({ ...playbackFromAlbum(album), trackId: 'demo-track-01-01' }, -1).playing).toStrictEqual(false);
});

test('the natural end replays one, wraps all and stops off', () => {
  const library = demoLibrary();
  const album = present(findAlbum(library, 'demo-album-01'), 'demo-album-01');
  const last = { ...playbackFromAlbum(album), trackId: 'demo-track-01-04' };
  // Off: the queue ends, playback stops, the track stays.
  const stopped = advanceQueue(last);
  expect(stopped.playing).toStrictEqual(false);
  expect(stopped.trackId).toStrictEqual('demo-track-01-04');
  expect(stopped.positionMs).toStrictEqual(last.positionMs);
  // All: wrap to the head and keep playing from its start.
  const wrapped = advanceQueue({ ...last, repeatMode: 'all' });
  expect(wrapped.trackId).toStrictEqual('demo-track-01-01');
  expect(wrapped.playing).toStrictEqual(true);
  expect(wrapped.positionMs).toStrictEqual(0);
  // One: the same track from the top.
  const again = advanceQueue({ ...last, repeatMode: 'one' });
  expect(again.trackId).toStrictEqual('demo-track-01-04');
  expect(again.playing).toStrictEqual(true);
  expect(again.positionMs).toStrictEqual(0);
  // Shuffle rides along at the natural end too: from Beacon, the seeded
  // order wraps to its head, Salt Window.
  const shuffledEnd = { ...last, shuffleOn: true, shuffleSeed: 1, repeatMode: 'all' as const };
  expect(advanceQueue(shuffledEnd).trackId).toStrictEqual('demo-track-01-02');
  // Mid-queue ends step on in the queue's order.
  expect(advanceQueue(playbackFromAlbum(album)).trackId).toStrictEqual('demo-track-01-02');
  expect(advanceQueue(emptyPlayback())).toStrictEqual(emptyPlayback());
});

test('removing a queue line drops it and keeps the rest in order', () => {
  const library = demoLibrary();
  const snapshot = playbackFromAlbum(present(findAlbum(library, 'demo-album-01'), 'demo-album-01'));
  // A later line goes; the current one and the order stay.
  const withoutSalt = removeLine(snapshot, 'demo-track-01-02');
  expect(withoutSalt.trackId).toStrictEqual('demo-track-01-01');
  expect(withoutSalt.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-01-03',
    'demo-track-01-04',
  ]);
  expect(withoutSalt.playing).toStrictEqual(true);
  expect(withoutSalt.queueOpen).toStrictEqual(false);
  // Removing the current line promotes the line after it.
  const withoutCurrent = removeLine(snapshot, 'demo-track-01-01');
  expect(withoutCurrent.trackId).toStrictEqual('demo-track-01-02');
  expect(withoutCurrent.title).toStrictEqual('Salt Window');
  expect(withoutCurrent.positionMs).toStrictEqual(0);
  expect(withoutCurrent.durationMs).toStrictEqual(198_000);
  expect(withoutCurrent.playing).toStrictEqual(true);
  // Removing the last line while it plays lands on the new last line.
  const last = { ...snapshot, trackId: 'demo-track-01-04' };
  const withoutLast = removeLine(last, 'demo-track-01-04');
  expect(withoutLast.trackId).toStrictEqual('demo-track-01-03');
  expect(withoutLast.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-01-02',
    'demo-track-01-03',
  ]);
  // The open sheet survives the edit, in both directions.
  expect(removeLine(setQueueOpen(snapshot, true), 'demo-track-01-02').queueOpen).toStrictEqual(true);
  // Removing an unknown line changes nothing.
  expect(removeLine(snapshot, 'missing')).toStrictEqual(snapshot);
  // Removing the only line empties the player honestly.
  const single = playFromLine(present(snapshot.queue[0], 'the only line'));
  const emptied = removeLine(single, 'demo-track-01-01');
  expect(emptied.trackId).toStrictEqual(undefined);
  expect(emptied.playing).toStrictEqual(false);
  expect(emptied.queue).toStrictEqual([]);
  expect(removeLine(emptyPlayback(), 'demo-track-01-01')).toStrictEqual(emptyPlayback());
});

test('a reported duration is adopted only when the catalogue had none', () => {
  const library = demoLibrary();
  const snapshot = playbackFromAlbum(present(findAlbum(library, 'demo-album-01'), 'demo-album-01'));
  // The catalogue knows the track: the report changes nothing.
  expect(adoptDuration(snapshot, 8_000)).toStrictEqual(snapshot);
  expect(adoptDuration(snapshot, 0)).toStrictEqual(snapshot);
  expect(adoptDuration(snapshot, Number.NaN)).toStrictEqual(snapshot);
  expect(adoptDuration(emptyPlayback(), 8_000)).toStrictEqual(emptyPlayback());
  // No catalogue duration (a hand-built queue): the file's length wins.
  const unknown = { ...playFromLine(present(snapshot.queue[0], 'the only line')), durationMs: 0 };
  expect(adoptDuration(unknown, 8_400.4)).toStrictEqual({ ...unknown, durationMs: 8_400 });
});
