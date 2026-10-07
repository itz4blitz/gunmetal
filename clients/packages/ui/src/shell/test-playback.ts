import type { PlaybackController } from './playback-controller.ts';
import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';

export type PlaybackStub = {
  controller: PlaybackController;
  /** The action names a test drove through the stub, in order. */
  calls: readonly string[];
};

/**
 * A scripted playback controller for component tests: records the calls a
 * test names and holds the snapshot the test wrote down. It implements no
 * rule — that is the point (component tests prove what a screen shows for a
 * given state; the rules live in the composition root and the core).
 */
export function stubPlayback(state?: PlayerSnapshot): PlaybackStub {
  const calls: string[] = [];
  const record = (name: string) => () => {
    calls.push(name);
  };
  const controller: PlaybackController = {
    state: state ?? {
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
    },
    fullOpen: false,
    volume: 0.8,
    muted: false,
    setVolume: record('setVolume'),
    setMuted: record('setMuted'),
    toggleShuffle: record('toggleShuffle'),
    cycleRepeat: record('cycleRepeat'),
    removeQueueLine: (trackId) => {
      calls.push(`removeQueueLine ${trackId}`);
    },
    playAlbum: record('playAlbum'),
    playTrack: record('playTrack'),
    playNextAlbum: record('playNextAlbum'),
    addAlbumToQueue: record('addAlbumToQueue'),
    playNextTrack: record('playNextTrack'),
    addTrackToQueue: record('addTrackToQueue'),
    playPause: record('playPause'),
    previous: record('previous'),
    next: record('next'),
    seek: record('seek'),
    toggleQueue: record('toggleQueue'),
    closeQueue: record('closeQueue'),
    openFull: record('openFull'),
    closeFull: record('closeFull'),
  };
  return { controller, calls };
}

/** Literal empty snapshot for component tests. */
export function emptySnapshot(): PlayerSnapshot {
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

const FIXTURE_LINES: readonly PlayerSnapshot['queue'][number][] = [
  {
    trackId: 'demo-track-01-01',
    albumId: 'demo-album-01',
    title: 'Pier at Dusk',
    artistName: 'Mira Sol',
    coverTone: '01',
    coverUrl: '/media/covers/fixture.svg',
    mediaUrl: '/media/audio/fixtures.wav',
    durationMs: 214_000,
    lyricsKind: 'none',
  },
  {
    trackId: 'demo-track-01-02',
    albumId: 'demo-album-01',
    title: 'Salt Window',
    artistName: 'Mira Sol',
    coverTone: '01',
    coverUrl: '/media/covers/fixture.svg',
    mediaUrl: '/media/audio/fixtures.wav',
    durationMs: 198_000,
    lyricsKind: 'none',
  },
  {
    trackId: 'demo-track-01-03',
    albumId: 'demo-album-01',
    title: 'Low Tide Letter',
    artistName: 'Mira Sol',
    coverTone: '01',
    coverUrl: '/media/covers/fixture.svg',
    mediaUrl: '/media/audio/fixtures.wav',
    durationMs: 241_000,
    lyricsKind: 'plain',
  },
  {
    trackId: 'demo-track-01-04',
    albumId: 'demo-album-01',
    title: 'Beacon',
    artistName: 'Mira Sol',
    coverTone: '01',
    coverUrl: '/media/covers/fixture.svg',
    mediaUrl: '/media/audio/fixtures.wav',
    durationMs: 187_000,
    lyricsKind: 'none',
  },
];

/** Literal queued-and-playing snapshot (demo album 01's four tracks). */
export function queuedSnapshot(currentIndex = 0): PlayerSnapshot {
  const line = FIXTURE_LINES[currentIndex];
  if (line === undefined) {
    return emptySnapshot();
  }
  return {
    trackId: line.trackId,
    albumId: line.albumId,
    title: line.title,
    artistName: line.artistName,
    coverTone: line.coverTone,
    coverUrl: line.coverUrl,
    mediaUrl: line.mediaUrl,
    playing: true,
    positionMs: 45_000,
    durationMs: line.durationMs,
    lyricsKind: line.lyricsKind,
    queue: FIXTURE_LINES,
    queueOpen: true,
  };
}

/**
 * The queued snapshot carrying the transport state the 2026 player pass
 * added: shuffle on with a seed, repeat all, the engine buffering. Tests
 * that need a different shape spread over this one.
 */
export function transportSnapshot(): PlayerSnapshot {
  return {
    ...queuedSnapshot(),
    shuffleOn: true,
    shuffleSeed: 42,
    repeatMode: 'all',
    buffering: false,
  };
}
