/**
 * The Media Session port (MUS-073, CLI-070): lock screens and hardware
 * media keys read the now-playing state and drive the same transport verbs
 * the on-screen controls use. The platform is reached only through the
 * bridge, so tests cover the wired and the unsupported paths alike.
 */

export type MediaSessionArtwork = { src: string; sizes: string; type: string };

export type MediaSessionAction =
  'play' | 'pause' | 'previoustrack' | 'nexttrack' | 'seekto' | 'seekbackward' | 'seekforward' | 'stop';

export type MediaSessionActionDetails = { seekTime?: number; seekOffset?: number };

export type MediaSessionBridge = {
  /** null clears the lock screen's metadata (an empty player). */
  setMetadata(metadata: MediaSessionMetadata | null): void;
  setPlaybackState(state: 'playing' | 'paused' | 'none'): void;
  /** Throws when the engine refuses the state (a position past the end, a zero rate). */
  setPositionState(state: { duration: number; position: number; playbackRate: number }): void;
  /** Throws for an action this engine does not offer. */
  setActionHandler(
    action: MediaSessionAction,
    handler: ((details?: MediaSessionActionDetails) => void) | undefined,
  ): void;
};

export type MediaSessionMetadata = {
  title: string;
  artist: string;
  album: string;
  artwork: readonly MediaSessionArtwork[];
};

export type MediaSessionSnapshot = {
  trackId: string | undefined;
  title: string;
  artistName: string;
  albumTitle: string | undefined;
  coverUrl: string;
  playing: boolean;
  positionMs: number;
  durationMs: number;
};

export type MediaSessionHandlers = {
  playPause(): void;
  previous(): void;
  next(): void;
  stop(): void;
  /** Jump to a position in milliseconds. */
  seekTo(positionMs: number): void;
  /** Move by a delta in milliseconds. */
  seekBy(deltaMs: number): void;
};

/** How far the media keys skip without an offset of their own, in seconds. */
const SEEK_STEP_S = 5;

/**
 * The one indirection to navigator.mediaSession. Returns undefined where
 * the API is missing, and a bridge that degrades to the methods the engine
 * actually has where the API is partial (some engines carry a mediaSession
 * without the newer methods): a missing method is a silent no-op, because
 * a lock-screen feature the browser lacks is the browser's loss, never a
 * crash in the shell's effects. The realm's constructor table is injected
 * with the navigator, so the MediaMetadata branch is testable without a
 * real browser.
 */
export function mediaSessionRef(
  nav: Navigator,
  platform: { MediaMetadata?: new (init: MediaSessionMetadata) => unknown },
): MediaSessionBridge | undefined {
  const candidate = (nav as Navigator & { mediaSession?: unknown }).mediaSession;
  if (typeof candidate !== 'object' || candidate === null) {
    return undefined;
  }
  const session = candidate as unknown as {
    metadata: unknown;
    setPlaybackState?(state: string): void;
    setPositionState?(state: { duration: number; position: number; playbackRate: number }): void;
    setActionHandler?(action: string, handler: ((details?: MediaSessionActionDetails) => void) | null): void;
  };
  return {
    setMetadata(metadata) {
      /* The IDL wants a MediaMetadata instance; engines without the
         constructor get the plain init object, which they duck-type. */
      const construct = platform.MediaMetadata;
      if (metadata === null) {
        session.metadata = null;
        return;
      }
      session.metadata = construct === undefined ? metadata : new construct(metadata);
    },
    setPlaybackState: (state) => session.setPlaybackState?.(state),
    setPositionState: (state) => session.setPositionState?.(state),
    setActionHandler: (action, handler) => session.setActionHandler?.(action, handler ?? null),
  };
}

/**
 * Publish one snapshot: the metadata and playback state, the eight
 * transport actions, and the position state. Every platform refusal is
 * contained — an unsupported action or a rejected position never breaks
 * playback (the in-app controls still do everything).
 */
function artworkType(coverUrl: string): 'image/jpeg' | 'image/png' | 'image/svg+xml' {
  if (coverUrl.endsWith('.jpg') || coverUrl.endsWith('.jpeg')) {
    return 'image/jpeg';
  }
  if (coverUrl.endsWith('.png')) {
    return 'image/png';
  }
  return 'image/svg+xml';
}

export function applyMediaSession(
  bridge: MediaSessionBridge | undefined,
  snapshot: MediaSessionSnapshot,
  handlers: MediaSessionHandlers,
  rate = 1,
): void {
  if (bridge === undefined) {
    return;
  }
  if (snapshot.trackId === undefined) {
    bridge.setMetadata(null);
    bridge.setPlaybackState('none');
    for (const action of ACTIONS) {
      wire(bridge, action, undefined);
    }
    return;
  }
  bridge.setMetadata({
    title: snapshot.title,
    artist: snapshot.artistName,
    album: snapshot.albumTitle ?? '',
    artwork:
      snapshot.coverUrl === '' ? [] : [{ src: snapshot.coverUrl, sizes: 'any', type: artworkType(snapshot.coverUrl) }],
  });
  bridge.setPlaybackState(snapshot.playing ? 'playing' : 'paused');
  wire(bridge, 'play', () => handlers.playPause());
  wire(bridge, 'pause', () => handlers.playPause());
  wire(bridge, 'previoustrack', () => handlers.previous());
  wire(bridge, 'nexttrack', () => handlers.next());
  wire(bridge, 'stop', () => handlers.stop());
  wire(bridge, 'seekto', (details) => {
    handlers.seekTo(details?.seekTime === undefined ? snapshot.positionMs : details.seekTime * 1000);
  });
  wire(bridge, 'seekbackward', (details) => {
    handlers.seekBy(-(details?.seekOffset ?? SEEK_STEP_S) * 1000);
  });
  wire(bridge, 'seekforward', (details) => {
    handlers.seekBy((details?.seekOffset ?? SEEK_STEP_S) * 1000);
  });
  /* The lock screen drifts unless the truth is pushed after every play,
     pause, seek and queue edit (CLI-070). The state is only sent when it
     is inside the platform's contract: a positive duration, a position
     within it, a non-zero rate. */
  if (snapshot.durationMs > 0 && snapshot.positionMs >= 0 && snapshot.positionMs <= snapshot.durationMs && rate > 0) {
    try {
      bridge.setPositionState({
        duration: snapshot.durationMs / 1000,
        position: snapshot.positionMs / 1000,
        playbackRate: rate,
      });
    } catch {
      // A racing state (say, a position past a shrinking duration) waits for the next sync.
    }
  }
}

const ACTIONS: readonly MediaSessionAction[] = [
  'play',
  'pause',
  'previoustrack',
  'nexttrack',
  'seekto',
  'seekbackward',
  'seekforward',
  'stop',
];

function wire(
  bridge: MediaSessionBridge,
  action: MediaSessionAction,
  handler: ((details?: MediaSessionActionDetails) => void) | undefined,
): void {
  try {
    bridge.setActionHandler(action, handler);
  } catch {
    // This engine does not offer the action; the in-app controls still do.
  }
}
