import { useEffect, useRef, useState } from 'react';
import type { ShellLibrary } from '../../../packages/ui/src/shell/library-types.ts';
import type { PlaybackController } from '../../../packages/ui/src/shell/playback-controller.ts';
import { createDemoAudio, type DemoAudio } from './demo-audio.ts';
import { applyAlbumQueue, applyPlayback, applyTrackQueue } from './demo-play.ts';
import { createAudioElement } from './browser/audio-element.ts';
import { emptyPlayback, seekTo, setQueueOpen, stepQueue, togglePlaying, type PlaybackSnapshot } from './playback.ts';

/**
 * The demo composition root's playback controller: fixture tones through one
 * <audio> element, with the queue in memory. apps/web replaces this with the
 * real ports (core-wasm queue verbs + ServerPort sync) — the shell cannot
 * tell the difference.
 */
export function useDemoPlayback(library: ShellLibrary | undefined): PlaybackController {
  const [state, setState] = useState<PlaybackSnapshot>(() => emptyPlayback());
  const [fullOpen, setFullOpen] = useState(false);
  const [volume, setVolume] = useState(0.8);
  const audioRef = useRef<DemoAudio | undefined>(undefined);
  const volumeRef = useRef(volume);
  volumeRef.current = volume;

  useEffect(() => {
    const audio = createDemoAudio(createAudioElement(), {
      onTime: (positionMs) => {
        setState((current) => (current.trackId === undefined ? current : { ...current, positionMs }));
      },
      onEnded: () => {
        setState((current) => stepQueue(current, 1));
      },
    });
    audio.setVolume(volumeRef.current);
    audioRef.current = audio;
    return () => {
      audio.detach();
      audioRef.current = undefined;
    };
  }, []);

  const trackId = state.trackId;
  const mediaUrl = state.mediaUrl;
  const playing = state.playing;
  const durationMs = state.durationMs;
  useEffect(() => {
    const audio = audioRef.current;
    if (audio === undefined || trackId === undefined || mediaUrl === '') {
      return;
    }
    audio.load(mediaUrl, durationMs);
    if (playing) {
      audio.setPlaying(true);
    }
    // durationMs changes only together with the track (same state update).
  }, [trackId, mediaUrl, durationMs]);

  useEffect(() => {
    audioRef.current?.setPlaying(playing);
  }, [playing]);

  useEffect(() => {
    audioRef.current?.setVolume(volume);
  }, [volume]);

  return {
    state,
    fullOpen,
    volume,
    setVolume,
    playAlbum: (albumId) => {
      setState(() => applyPlayback(library, albumId, undefined));
    },
    playTrack: (albumId, track) => {
      setState(() => applyPlayback(library, albumId, track));
    },
    playNextAlbum: (albumId) => {
      setState((current) => applyAlbumQueue(library, current, albumId, 'next'));
    },
    addAlbumToQueue: (albumId) => {
      setState((current) => applyAlbumQueue(library, current, albumId, 'append'));
    },
    playNextTrack: (albumId, track) => {
      setState((current) => applyTrackQueue(library, current, albumId, track, 'next'));
    },
    addTrackToQueue: (albumId, track) => {
      setState((current) => applyTrackQueue(library, current, albumId, track, 'append'));
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
