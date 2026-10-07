import { expect, test } from 'vitest';
import {
  applyMediaSession,
  mediaSessionRef,
  type MediaSessionAction,
  type MediaSessionBridge,
} from './media-session.ts';

/** A recording media session, the way tests hand one to the module. */
function recordingSession() {
  const metadata: unknown[] = [];
  const states: Array<Record<string, number>> = [];
  const playbackStates: string[] = [];
  const handlers = new Map<MediaSessionAction, unknown>();
  const failures = new Set<MediaSessionAction>();
  let rejectPositionState = false;
  const session = {
    set metadata(value: unknown) {
      metadata.push(value);
    },
    setPlaybackState(value: string) {
      playbackStates.push(value);
    },
    setPositionState(state: Record<string, number>) {
      if (rejectPositionState) {
        throw new TypeError('position out of range');
      }
      states.push(state);
    },
    setActionHandler(action: MediaSessionAction, handler: unknown) {
      if (failures.has(action)) {
        throw new TypeError('unsupported action');
      }
      handlers.set(action, handler);
    },
  };
  const bridge: MediaSessionBridge & { session: typeof session; rejectPosition(v: boolean): void } = {
    session,
    rejectPosition: (value) => {
      rejectPositionState = value;
    },
    setMetadata: (value) => {
      session.metadata = value;
    },
    setPlaybackState: (value) => session.setPlaybackState(value),
    setPositionState: (state) => session.setPositionState(state),
    setActionHandler: (action, handler) => session.setActionHandler(action, handler),
  };
  return { bridge, metadata, states, playbackStates, handlers, failures };
}

const PLAYING = {
  trackId: 'demo-track-01-01' as string | undefined,
  title: 'Pier at Dusk',
  artistName: 'Mira Sol',
  albumTitle: 'Harbour Lights' as string | undefined,
  coverUrl: '/media/covers/demo-album-01.svg',
  playing: true,
  positionMs: 45_000,
  durationMs: 180_000,
};

const HANDLERS = {
  playPause: () => {},
  previous: () => {},
  next: () => {},
  stop: () => {},
  seekTo: () => {},
  seekBy: () => {},
};

function wiredHandlers(bridge: MediaSessionBridge, snapshot: typeof PLAYING) {
  const calls: string[] = [];
  applyMediaSession(bridge, snapshot, {
    playPause: () => calls.push('playPause'),
    previous: () => calls.push('previous'),
    next: () => calls.push('next'),
    stop: () => calls.push('stop'),
    seekTo: (ms) => calls.push(`seekTo ${ms}`),
    seekBy: (ms) => calls.push(`seekBy ${ms}`),
  });
  return calls;
}

test('a navigator without a media session reports no bridge and syncing is a no-op', () => {
  expect(mediaSessionRef({} as Navigator, {})).toStrictEqual(undefined);
  expect(mediaSessionRef({ mediaSession: null } as unknown as Navigator, {})).toStrictEqual(undefined);
  expect(() => applyMediaSession(undefined, PLAYING, HANDLERS)).not.toThrow();
});

test('the bridge wires the eight transport actions through the session', () => {
  const { bridge, handlers } = recordingSession();
  const calls: string[] = [];
  applyMediaSession(bridge, PLAYING, {
    playPause: () => calls.push('playPause'),
    previous: () => calls.push('previous'),
    next: () => calls.push('next'),
    stop: () => calls.push('stop'),
    seekTo: (ms) => calls.push(`seekTo ${ms}`),
    seekBy: (ms) => calls.push(`seekBy ${ms}`),
  });
  expect([...handlers.keys()].sort()).toStrictEqual(
    ['nexttrack', 'pause', 'play', 'previoustrack', 'seekbackward', 'seekforward', 'seekto', 'stop'].sort(),
  );
  const fire = (action: MediaSessionAction, details?: { seekTime?: number; seekOffset?: number }) =>
    (handlers.get(action) as (input?: typeof details) => void)(details);
  fire('play');
  fire('pause');
  fire('previoustrack');
  fire('nexttrack');
  fire('stop');
  expect(calls).toStrictEqual(['playPause', 'playPause', 'previous', 'next', 'stop']);
  calls.length = 0;
  fire('seekto', { seekTime: 12.5 });
  fire('seekto', {});
  fire('seekbackward', { seekOffset: 30 });
  fire('seekbackward');
  fire('seekforward', { seekOffset: 15 });
  fire('seekforward');
  expect(calls).toStrictEqual([
    'seekTo 12500',
    'seekTo 45000',
    'seekBy -30000',
    'seekBy -5000',
    'seekBy 15000',
    'seekBy 5000',
  ]);
});

test('the bridge publishes metadata with artwork and the playback state', () => {
  const { bridge, metadata, playbackStates } = recordingSession();
  applyMediaSession(bridge, PLAYING, HANDLERS);
  expect(metadata).toStrictEqual([
    {
      title: 'Pier at Dusk',
      artist: 'Mira Sol',
      album: 'Harbour Lights',
      artwork: [{ src: '/media/covers/demo-album-01.svg', sizes: 'any', type: 'image/svg+xml' }],
    },
  ]);
  expect(playbackStates).toStrictEqual(['playing']);
  metadata.length = 0;
  playbackStates.length = 0;
  applyMediaSession(bridge, { ...PLAYING, playing: false, albumTitle: undefined, coverUrl: '' }, HANDLERS);
  expect(metadata).toStrictEqual([{ title: 'Pier at Dusk', artist: 'Mira Sol', album: '', artwork: [] }]);
  expect(playbackStates).toStrictEqual(['paused']);
  metadata.length = 0;
  applyMediaSession(bridge, { ...PLAYING, coverUrl: '/media/library/covers/aaaaaaaaaaaaaaaa.jpg' }, HANDLERS);
  expect(metadata).toStrictEqual([
    {
      title: 'Pier at Dusk',
      artist: 'Mira Sol',
      album: 'Harbour Lights',
      artwork: [{ src: '/media/library/covers/aaaaaaaaaaaaaaaa.jpg', sizes: 'any', type: 'image/jpeg' }],
    },
  ]);
  metadata.length = 0;
  applyMediaSession(bridge, { ...PLAYING, coverUrl: '/media/library/covers/aaaaaaaaaaaaaaaa.jpeg' }, HANDLERS);
  expect(metadata).toStrictEqual([
    {
      title: 'Pier at Dusk',
      artist: 'Mira Sol',
      album: 'Harbour Lights',
      artwork: [{ src: '/media/library/covers/aaaaaaaaaaaaaaaa.jpeg', sizes: 'any', type: 'image/jpeg' }],
    },
  ]);
  metadata.length = 0;
  applyMediaSession(bridge, { ...PLAYING, coverUrl: '/media/library/covers/aaaaaaaaaaaaaaaa.png' }, HANDLERS);
  expect(metadata).toStrictEqual([
    {
      title: 'Pier at Dusk',
      artist: 'Mira Sol',
      album: 'Harbour Lights',
      artwork: [{ src: '/media/library/covers/aaaaaaaaaaaaaaaa.png', sizes: 'any', type: 'image/png' }],
    },
  ]);
});

test('the bridge prefers the injected MediaMetadata constructor over the plain init', () => {
  class MediaMetadata {
    init: unknown;
    constructor(init: unknown) {
      this.init = init;
    }
  }
  const recorded: unknown[] = [];
  const states: Array<Record<string, number>> = [];
  const playbackStates: string[] = [];
  const handlers = new Map<MediaSessionAction, unknown>();
  const session = {
    set metadata(value: unknown) {
      recorded.push(value);
    },
    setPlaybackState: (value: string) => {
      playbackStates.push(value);
    },
    setPositionState: (state: Record<string, number>) => {
      states.push(state);
    },
    setActionHandler: (action: MediaSessionAction, handler: unknown) => {
      handlers.set(action, handler);
    },
  };
  const bridge = mediaSessionRef({ mediaSession: session } as unknown as Navigator, { MediaMetadata });
  if (bridge === undefined) {
    throw new Error('a session the navigator carries must produce a bridge');
  }
  applyMediaSession(bridge, PLAYING, HANDLERS);
  const built = recorded[0] as MediaMetadata;
  expect(built).toBeInstanceOf(MediaMetadata);
  expect(built.init).toStrictEqual({
    title: 'Pier at Dusk',
    artist: 'Mira Sol',
    album: 'Harbour Lights',
    artwork: [{ src: '/media/covers/demo-album-01.svg', sizes: 'any', type: 'image/svg+xml' }],
  });
  // The same bridge carries the state, the playback state and the actions.
  expect(states).toStrictEqual([{ duration: 180, position: 45, playbackRate: 1 }]);
  expect(playbackStates).toStrictEqual(['playing']);
  expect(handlers.size).toStrictEqual(8);
  // An empty player clears through the same bridge: metadata null.
  applyMediaSession(bridge, { ...PLAYING, trackId: undefined }, HANDLERS);
  expect(recorded.at(-1)).toStrictEqual(null);
  // Without the constructor the plain init object is what the session gets.
  const plain: unknown[] = [];
  const bareSession = {
    set metadata(value: unknown) {
      plain.push(value);
    },
    setPlaybackState: () => {},
    setPositionState: () => {},
    setActionHandler: () => {},
  };
  const bare = mediaSessionRef({ mediaSession: bareSession } as unknown as Navigator, {});
  applyMediaSession(bare, PLAYING, HANDLERS);
  expect(plain).toStrictEqual([
    {
      title: 'Pier at Dusk',
      artist: 'Mira Sol',
      album: 'Harbour Lights',
      artwork: [{ src: '/media/covers/demo-album-01.svg', sizes: 'any', type: 'image/svg+xml' }],
    },
  ]);
});

test('an empty player clears the metadata, every handler and the playback state', () => {
  const { bridge, metadata, handlers, playbackStates } = recordingSession();
  wiredHandlers(bridge, PLAYING);
  expect(handlers.size).toStrictEqual(8);
  metadata.length = 0;
  playbackStates.length = 0;
  applyMediaSession(bridge, { ...PLAYING, trackId: undefined }, HANDLERS);
  expect(metadata).toStrictEqual([null]);
  expect(playbackStates).toStrictEqual(['none']);
  expect([...handlers.values()].filter((handler) => handler !== undefined)).toHaveLength(0);
});

test('the position state is pushed only for honest values and survives a rejecting engine', () => {
  const { bridge, states } = recordingSession();
  applyMediaSession(bridge, PLAYING, HANDLERS);
  expect(states).toStrictEqual([{ duration: 180, position: 45, playbackRate: 1 }]);
  // A position past the duration is the classic gapless-handoff crash; it is guarded.
  expect(() => applyMediaSession(bridge, { ...PLAYING, positionMs: 190_000 }, HANDLERS)).not.toThrow();
  expect(states).toHaveLength(1);
  // A zero rate is refused by the platform, so it is never sent.
  applyMediaSession(bridge, PLAYING, HANDLERS, 0);
  expect(states).toHaveLength(1);
  // A track with no known length has nothing honest to report.
  applyMediaSession(bridge, { ...PLAYING, durationMs: 0 }, HANDLERS);
  expect(states).toHaveLength(1);
  // A rejecting engine (a racing state) is caught, not fatal.
  bridge.rejectPosition(true);
  expect(() => applyMediaSession(bridge, PLAYING, HANDLERS)).not.toThrow();
  bridge.rejectPosition(false);
  applyMediaSession(bridge, PLAYING, HANDLERS, 1.5);
  expect(states).toStrictEqual([
    { duration: 180, position: 45, playbackRate: 1 },
    { duration: 180, position: 45, playbackRate: 1.5 },
  ]);
});

test('an action the engine refuses is skipped without dropping the rest', () => {
  const { bridge, handlers, failures } = recordingSession();
  failures.add('seekbackward');
  wiredHandlers(bridge, PLAYING);
  expect(handlers.has('seekbackward')).toStrictEqual(false);
  expect(handlers.size).toStrictEqual(7);
});

test('a session without setPlaybackState still syncs everything else', () => {
  /* Real engines differ: some carry a mediaSession without the newer
     methods. The bridge must degrade to the methods the engine has, never
     throw inside the shell's effects. */
  const recorded: unknown[] = [];
  const handlers = new Map<MediaSessionAction, unknown>();
  const partial = {
    set metadata(value: unknown) {
      recorded.push(value);
    },
    setActionHandler: (action: MediaSessionAction, handler: unknown) => {
      handlers.set(action, handler);
    },
  };
  const bridge = mediaSessionRef({ mediaSession: partial } as unknown as Navigator, {});
  expect(bridge).not.toStrictEqual(undefined);
  expect(() => applyMediaSession(bridge, PLAYING, HANDLERS)).not.toThrow();
  expect(() => applyMediaSession(bridge, { ...PLAYING, trackId: undefined }, HANDLERS)).not.toThrow();
  expect(recorded).toStrictEqual([
    {
      title: 'Pier at Dusk',
      artist: 'Mira Sol',
      album: 'Harbour Lights',
      artwork: [{ src: '/media/covers/demo-album-01.svg', sizes: 'any', type: 'image/svg+xml' }],
    },
    null,
  ]);
  expect(handlers.size).toStrictEqual(8);
});

test('a session without setPositionState or without setActionHandler also degrades', () => {
  const playbackStates: string[] = [];
  const noPosition = {
    set metadata(_value: unknown) {},
    setPlaybackState: (value: string) => {
      playbackStates.push(value);
    },
    setActionHandler: () => {},
  };
  const positionless = mediaSessionRef({ mediaSession: noPosition } as unknown as Navigator, {});
  expect(() => applyMediaSession(positionless, PLAYING, HANDLERS)).not.toThrow();
  expect(playbackStates).toStrictEqual(['playing']);

  const actionless = {
    set metadata(_value: unknown) {},
    setPlaybackState: () => {},
    setPositionState: () => {},
  };
  const handlerless = mediaSessionRef({ mediaSession: actionless } as unknown as Navigator, {});
  expect(() => applyMediaSession(handlerless, PLAYING, HANDLERS)).not.toThrow();
});
