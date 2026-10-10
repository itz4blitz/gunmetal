import { useEffect, useMemo, useRef, useState } from 'react';
import type { ShellLibrary } from '../../../packages/ui/src/shell/library-types.ts';
import type { PlaybackController } from '../../../packages/ui/src/shell/playback-controller.ts';
import { createPositionClock, defaultScheduler } from '../../../packages/ui/src/shell/position-clock.ts';
import {
  applyMediaSession,
  mediaSessionRef,
  type MediaSessionBridge,
} from '../../../packages/ui/src/shell/media-session.ts';
import {
  parseVolume,
  serializeVolume,
  defaultVolumeMemory,
  type VolumeMemory,
  type VolumeStore,
} from '../../../packages/ui/src/shell/volume-store.ts';
import { createDemoAudio, type DemoAudio, type DemoAudioElement } from './demo-audio.ts';
import { applyAlbumQueue, applyPlayback, applyTrackQueue } from './demo-play.ts';
import type { PlaybackPrefs } from './playback-prefs.ts';
import {
  advanceQueue,
  adoptDuration,
  carryQueueOpen,
  cycleRepeatMode,
  findTrack,
  removeLine,
  seekTo,
  setQueueOpen,
  stepQueue,
  togglePlaying,
  toggleShuffleMode,
  type PlaybackSnapshot,
} from './playback.ts';
import { createAudioElement } from './browser/audio-element.ts';
import { parseSessionPlayback, serializeSessionPlayback, type SessionPlaybackStore } from './session-playback.ts';

/**
 * What the composition root can wire for tests and embeds: the element
 * factory, the storage for the volume, the clock's frame scheduler and a
 * ready media-session bridge. Everything defaults to the real browser.
 */
export type DemoPlaybackWiring = {
  createElement?: () => DemoAudioElement;
  volumeStore?: VolumeStore | undefined;
  /**
   * This tab's queue and place in the song. Session storage only: the queue
   * is Activity and must not go in localStorage (SEC-PRV-019).
   */
  sessionStore?: SessionPlaybackStore | undefined;
  scheduler?: Parameters<typeof createPositionClock>[0];
  mediaSession?: MediaSessionBridge | undefined;
  /** Levelling, crossfade and output. Absent leaves the engine's gain and sink alone. */
  prefs?: PlaybackPrefs | undefined;
};

/**
 * The demo composition root's playback controller: fixture tones through one
 * <audio> element, with the queue in memory. apps/web replaces this with the
 * real ports (core-wasm queue verbs + ServerPort sync) — the shell cannot
 * tell the difference.
 */
export function useDemoPlayback(library: ShellLibrary | undefined, wiring?: DemoPlaybackWiring): PlaybackController {
  const [state, setState] = useState<PlaybackSnapshot>(() =>
    parseSessionPlayback(wiring?.sessionStore?.read() ?? null),
  );
  const resumeMsRef = useRef(state.positionMs);
  /* A reload reports time 0 before the saved seek lands. Hold that report
     until the listener plays, seeks, or changes track. */
  const holdReports = useRef(state.positionMs > 0);
  const heldTrackId = useRef(state.trackId);
  const [fullOpen, setFullOpen] = useState(false);
  const [memory, setMemory] = useState<VolumeMemory>(() =>
    wiring?.volumeStore === undefined ? defaultVolumeMemory : parseVolume(wiring.volumeStore.read()),
  );
  const audioRef = useRef<DemoAudio | undefined>(undefined);
  const clock = useMemo(() => createPositionClock(wiring?.scheduler ?? defaultScheduler()), []);
  const trackRef = useRef({
    trackId: state.trackId,
    mediaUrl: state.mediaUrl,
    durationMs: state.durationMs,
    playing: state.playing,
  });
  trackRef.current = {
    trackId: state.trackId,
    mediaUrl: state.mediaUrl,
    durationMs: state.durationMs,
    playing: state.playing,
  };
  const shuffleSeedRef = useRef(0);
  const sessionStateRef = useRef(state);
  sessionStateRef.current = state;
  /* The track id the engine's crossfade already started on its spare
     element. The load effect skips one reload for exactly that track. */
  const handoffRef = useRef<string | undefined>(undefined);

  useEffect(() => {
    const makeElement = wiring?.createElement ?? createAudioElement;
    const primary = makeElement();
    const spare = makeElement();
    /* Distinct instances mean a real overlap; the same one means a single
       element, and the engine falls back to the window-start handoff. */
    const dual = spare !== primary;
    const audio = createDemoAudio(
      primary,
      {
        onTime: (positionMs) => {
          if (holdReports.current) {
            return;
          }
          setState((current) => (current.trackId === undefined ? current : { ...current, positionMs }));
        },
        onEnded: () => {
          const current = sessionStateRef.current;
          const stepped = advanceQueue(current);
          if (stepped.playing && stepped.trackId === current.trackId) {
            /* Repeat one, or the next line is the same track: track id and
               media url do not change, so the load effect never runs. The
               engine must restart in place or the bar sticks at the end. */
            audioRef.current?.load(stepped.mediaUrl, stepped.durationMs);
            audioRef.current?.setPlaying(true);
          }
          setState(stepped);
        },
        onBuffering: (buffering) => {
          setState((current) => (current.trackId === undefined ? current : { ...current, buffering }));
        },
        onError: (message) => {
          setState((current) => (current.trackId === undefined ? current : { ...current, playbackError: message }));
        },
        onDuration: (reportedMs) => {
          setState((current) => adoptDuration(current, reportedMs));
        },
        ...(dual
          ? {
              onCrossfade: (url: string) => {
                const current = sessionStateRef.current;
                const next = successor(current);
                if (next === undefined || next.mediaUrl !== url) {
                  return;
                }
                const stepped = advanceQueue(current);
                /* The engine is already playing this track on the element it
                   swapped in; the load effect must not restart it. */
                handoffRef.current = stepped.trackId;
                setState(stepped);
              },
            }
          : {}),
      },
      spare,
    );
    audio.setVolume(memory.muted ? 0 : memory.volume);
    audioRef.current = audio;
    return () => {
      audio.detach();
      audioRef.current = undefined;
      clock.detach();
    };
    // The element and the clock live for the hook's lifetime.
  }, []);

  /* The media element reloads only when the track changes; a duration that
     arrives later (loadedmetadata) must not restart it. A crossfade handoff
     is the one exception: the engine already moved to that track. */
  useEffect(() => {
    const audio = audioRef.current;
    const track = trackRef.current;
    if (audio === undefined || track.trackId === undefined || track.mediaUrl === '') {
      return;
    }
    if (handoffRef.current === track.trackId) {
      handoffRef.current = undefined;
      resumeMsRef.current = 0;
      return;
    }
    audio.load(track.mediaUrl, track.durationMs);
    const resumeMs = resumeMsRef.current;
    if (resumeMs > 0) {
      audio.seek(resumeMs);
    }
    resumeMsRef.current = 0;
    if (track.playing) {
      audio.setPlaying(true);
    }
  }, [state.trackId, state.mediaUrl]);

  useEffect(() => {
    if (state.playing) {
      holdReports.current = false;
    }
    audioRef.current?.setPlaying(state.playing);
  }, [state.playing]);

  useEffect(() => {
    if (state.trackId === heldTrackId.current) {
      return;
    }
    heldTrackId.current = state.trackId;
    holdReports.current = false;
  }, [state.trackId]);

  const sessionStore = wiring?.sessionStore;
  useEffect(() => {
    if (sessionStore === undefined) {
      return;
    }
    const write = () => {
      const current = sessionStateRef.current;
      const positionMs = holdReports.current ? current.positionMs : Math.round(clock.positionMs());
      sessionStore.write(serializeSessionPlayback({ ...current, positionMs }));
    };
    write();
    const onHide = () => {
      write();
    };
    globalThis.addEventListener('pagehide', onHide);
    globalThis.addEventListener('beforeunload', onHide);
    return () => {
      globalThis.removeEventListener('pagehide', onHide);
      globalThis.removeEventListener('beforeunload', onHide);
    };
  }, [
    sessionStore,
    state.trackId,
    state.albumId,
    state.playing,
    state.queue,
    state.shuffleOn,
    state.shuffleSeed,
    state.repeatMode,
    state.mediaUrl,
  ]);

  useEffect(() => {
    audioRef.current?.setVolume(memory.muted ? 0 : memory.volume);
  }, [memory]);

  /* Gain follows the playing track's tags; crossfade and sink follow the
     preference. The creation effect above has already stored the engine. */
  const prefs = wiring?.prefs;
  useEffect(() => {
    if (prefs === undefined) {
      return;
    }
    const audio = audioRef.current as DemoAudio;
    applyPlaybackPrefs(audio, prefs, library, state);
  }, [
    prefs?.levelling,
    prefs?.crossfadeSeconds,
    prefs?.sinkId,
    library,
    state.trackId,
    state.queue,
    state.repeatMode,
    state.shuffleOn,
    state.shuffleSeed,
    state.mediaUrl,
    state.durationMs,
  ]);

  /* The clock interpolates between the engine's reports so scrubbers, times
     and lyric lines can move smoothly without re-rendering the shell. */
  useEffect(() => {
    clock.sync({
      durationMs: state.durationMs,
      positionMs: state.positionMs,
      playing: state.playing,
    });
  }, [state, clock]);

  /* The lock screen follows the same truth: metadata, state, position and
     the transport verbs, pushed on every change (CLI-070). */
  const seekEngine = (positionMs: number) => {
    holdReports.current = false;
    audioRef.current?.seek(positionMs);
  };
  useEffect(() => {
    const bridge =
      wiring?.mediaSession !== undefined
        ? wiring.mediaSession
        : mediaSessionRef(
            globalThis.navigator,
            globalThis as typeof globalThis & Parameters<typeof mediaSessionRef>[1],
          );
    applyMediaSession(
      bridge,
      {
        trackId: state.trackId,
        title: state.title,
        artistName: state.artistName,
        albumTitle: state.albumId === undefined ? undefined : albumTitleOf(library, state.albumId),
        coverUrl: state.coverUrl,
        playing: state.playing,
        positionMs: state.positionMs,
        durationMs: state.durationMs,
      },
      {
        playPause: () => {
          setState((current) => togglePlaying(current));
        },
        previous: () => {
          setState((current) => stepQueue(current, -1));
        },
        next: () => {
          setState((current) => stepQueue(current, 1));
        },
        stop: () => {
          setState((current) => (current.trackId === undefined ? current : { ...current, playing: false }));
        },
        seekTo: (positionMs) => {
          seekEngine(positionMs);
          setState((current) => seekTo(current, positionMs));
        },
        seekBy: (deltaMs) => {
          const current = sessionStateRef.current;
          const target = Math.max(0, Math.min(current.durationMs, current.positionMs + deltaMs));
          seekEngine(target);
          setState((snapshot) => seekTo(snapshot, target));
        },
      },
    );
  }, [state, library, wiring?.mediaSession]);

  const volumeStore = wiring?.volumeStore;
  const storedOnce = useRef(false);
  useEffect(() => {
    /* The level just read back is not written again; only changes are. */
    if (!storedOnce.current) {
      storedOnce.current = true;
      return;
    }
    volumeStore?.write(serializeVolume(memory));
  }, [memory, volumeStore]);

  return {
    state,
    fullOpen,
    volume: memory.volume,
    muted: memory.muted,
    clock,
    setVolume: (volume) => {
      setMemory((current) => ({ ...current, volume: Math.max(0, Math.min(1, volume)) }));
    },
    setMuted: (muted) => {
      setMemory((current) => ({ ...current, muted }));
    },
    toggleShuffle: () => {
      shuffleSeedRef.current += 1;
      const seed = shuffleSeedRef.current;
      setState((current) => toggleShuffleMode(current, seed));
    },
    cycleRepeat: () => {
      setState((current) => cycleRepeatMode(current));
    },
    removeQueueLine: (trackId) => {
      setState((current) => removeLine(current, trackId));
    },
    playAlbum: (nextAlbumId) => {
      setState((current) => carryQueueOpen(current, applyPlayback(library, nextAlbumId, undefined)));
    },
    playTrack: (nextAlbumId, track) => {
      setState((current) => carryQueueOpen(current, applyPlayback(library, nextAlbumId, track)));
    },
    playNextAlbum: (nextAlbumId) => {
      setState((current) => applyAlbumQueue(library, current, nextAlbumId, 'next'));
    },
    addAlbumToQueue: (nextAlbumId) => {
      setState((current) => applyAlbumQueue(library, current, nextAlbumId, 'append'));
    },
    playNextTrack: (nextAlbumId, track) => {
      setState((current) => applyTrackQueue(library, current, nextAlbumId, track, 'next'));
    },
    addTrackToQueue: (nextAlbumId, track) => {
      setState((current) => applyTrackQueue(library, current, nextAlbumId, track, 'append'));
    },
    playPause: () => {
      setState((current) => togglePlaying(current));
    },
    previous: () => {
      setState((current) => stepQueue(current, -1));
    },
    next: () => {
      setState((current) => stepQueue(current, 1));
    },
    seek: (positionMs) => {
      seekEngine(positionMs);
      setState((current) => seekTo(current, positionMs));
    },
    toggleQueue: () => {
      setState((current) => setQueueOpen(current, !current.queueOpen));
    },
    closeQueue: () => {
      setState((current) => setQueueOpen(current, false));
    },
    openFull: () => {
      setFullOpen(true);
    },
    closeFull: () => {
      setFullOpen(false);
    },
  };
}

function albumTitleOf(library: ShellLibrary | undefined, albumId: string): string | undefined {
  return library?.albums.find((album) => album.id === albumId)?.title;
}

/**
 * The decibels `setGainDb` should apply. Off clears the gain. Track and
 * album pass that tag, including when the file has none.
 */
function appliedGain(
  levelling: PlaybackPrefs['levelling'],
  library: ShellLibrary | undefined,
  trackId: string | undefined,
): number | undefined {
  if (levelling === 'off' || library === undefined || trackId === undefined) {
    return undefined;
  }
  const found = findTrack(library, trackId);
  if (found === undefined) {
    return undefined;
  }
  if (levelling === 'track') {
    return found.track.trackGainDb;
  }
  return found.track.albumGainDb;
}

/** The line that will play next, or nothing when the queue ends. */
function successor(snapshot: PlaybackSnapshot): { mediaUrl: string; durationMs: number } | undefined {
  if (snapshot.trackId === undefined) {
    return undefined;
  }
  if (snapshot.repeatMode === 'one') {
    return { mediaUrl: snapshot.mediaUrl, durationMs: snapshot.durationMs };
  }
  const stepped = stepQueue(snapshot, 1);
  if (stepped.playing === false) {
    return undefined;
  }
  return { mediaUrl: stepped.mediaUrl, durationMs: stepped.durationMs };
}

function applyPlaybackPrefs(
  audio: DemoAudio,
  prefs: PlaybackPrefs,
  library: ShellLibrary | undefined,
  snapshot: PlaybackSnapshot,
): void {
  audio.setGainDb(appliedGain(prefs.levelling, library, snapshot.trackId));
  audio.setCrossfadeMs(prefs.crossfadeSeconds * 1000);
  audio.setSinkId(prefs.sinkId);
  const next = successor(snapshot);
  if (next !== undefined) {
    audio.armNext(next.mediaUrl, next.durationMs);
  }
}
