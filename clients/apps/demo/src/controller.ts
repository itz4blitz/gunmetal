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
import {
  advanceQueue,
  adoptDuration,
  carryQueueOpen,
  cycleRepeatMode,
  emptyPlayback,
  removeLine,
  seekTo,
  setQueueOpen,
  stepQueue,
  togglePlaying,
  toggleShuffleMode,
  type PlaybackSnapshot,
} from './playback.ts';
import { createAudioElement } from './browser/audio-element.ts';

/**
 * What the composition root can wire for tests and embeds: the element
 * factory, the storage for the volume, the clock's frame scheduler and a
 * ready media-session bridge. Everything defaults to the real browser.
 */
export type DemoPlaybackWiring = {
  createElement?: () => DemoAudioElement;
  volumeStore?: VolumeStore | undefined;
  scheduler?: Parameters<typeof createPositionClock>[0];
  mediaSession?: MediaSessionBridge | undefined;
};

/**
 * The demo composition root's playback controller: fixture tones through one
 * <audio> element, with the queue in memory. apps/web replaces this with the
 * real ports (core-wasm queue verbs + ServerPort sync) — the shell cannot
 * tell the difference.
 */
export function useDemoPlayback(library: ShellLibrary | undefined, wiring?: DemoPlaybackWiring): PlaybackController {
  const [state, setState] = useState<PlaybackSnapshot>(() => emptyPlayback());
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

  useEffect(() => {
    const audio = createDemoAudio(wiring?.createElement?.() ?? createAudioElement(), {
      onTime: (positionMs) => {
        setState((current) => (current.trackId === undefined ? current : { ...current, positionMs }));
      },
      onEnded: () => {
        setState((current) => advanceQueue(current));
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
    });
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
     arrives later (loadedmetadata) must not restart it. */
  useEffect(() => {
    const audio = audioRef.current;
    const track = trackRef.current;
    if (audio === undefined || track.trackId === undefined || track.mediaUrl === '') {
      return;
    }
    audio.load(track.mediaUrl, track.durationMs);
    if (track.playing) {
      audio.setPlaying(true);
    }
  }, [state.trackId, state.mediaUrl]);

  useEffect(() => {
    audioRef.current?.setPlaying(state.playing);
  }, [state.playing]);

  useEffect(() => {
    audioRef.current?.setVolume(memory.muted ? 0 : memory.volume);
  }, [memory]);

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
          setState((current) => seekTo(current, positionMs));
        },
        seekBy: (deltaMs) => {
          setState((current) =>
            seekTo(current, Math.max(0, Math.min(current.durationMs, current.positionMs + deltaMs))),
          );
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
      audioRef.current?.seek(positionMs);
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
