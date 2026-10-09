import { outputVolume } from './gain.ts';

// Demo audio engine until CP-019 wires the real engine and the core's
// decision engine. All logic lives here so it runs in Vitest against a
// scripted element; the DOM element itself comes from the one dumb adapter
// in src/browser/audio-element.ts.
//
// A fixture tone is a short loop under a longer catalogue duration. A file
// whose length matches that row plays once, and the element's own end
// hands the track over. CP-019/WP-030 replace this clock.

/** The element events the engine mirrors into honest playback state. */
export type DemoAudioEvent = 'timeupdate' | 'ended' | 'waiting' | 'stalled' | 'playing' | 'loadedmetadata' | 'error';

export type DemoAudioElement = {
  src: string;
  loop: boolean;
  volume: number;
  currentTime: number;
  duration: number;
  /** The engine reads the reason off a failed element (MediaError or null). */
  error: { message?: string } | null;
  play(): void | Promise<void>;
  pause(): void;
  addEventListener(type: DemoAudioEvent, listener: () => void): void;
  removeEventListener(type: DemoAudioEvent, listener: () => void): void;
  /** Present when the element can move playback to another device. */
  setSinkId?(id: string): Promise<void>;
};

export type DemoAudioHooks = {
  /** Fired on the element's timeupdate with the virtual position in ms. */
  onTime: (positionMs: number, durationMs: number) => void;
  /** Fired once when the virtual clock reaches the track's duration. */
  onEnded: () => void;
  /** Fired when the media is, or stops being, not ready to play through. */
  onBuffering: (buffering: boolean) => void;
  /** Fired on the element's error event with the engine's reason ('' if it gave none). */
  onError: (message: string) => void;
  /**
   * Fired from loadedmetadata with the track duration in ms — only when
   * the engine had to adopt the element's length because the catalogue
   * gave none. A catalogue duration stays the clock's scale (the demo
   * fixtures play at that scale on purpose).
   */
  onDuration: (durationMs: number) => void;
  /**
   * Fired once per track when the virtual clock enters the crossfade
   * window and a next url is armed. The owner starts that url; this
   * element is the one fading out.
   */
  onCrossfade?: (url: string) => void;
};

export type DemoAudio = {
  /** Point the element at a fixture URL with its catalogue duration. */
  load: (url: string, durationMs: number) => void;
  /** Mirror the transport state onto the element. */
  setPlaying: (playing: boolean) => void;
  /** Move the virtual clock to a position in milliseconds. */
  seek: (positionMs: number) => void;
  /** Set the output volume, 0..1. */
  setVolume: (volume: number) => void;
  /** Multiply a track gain, in dB, into the user volume. Undefined clears it. */
  setGainDb: (db: number | undefined) => void;
  /** How long before the end the next track may start. Zero disables it. */
  setCrossfadeMs: (ms: number) => void;
  /** Remember the url to hand to onCrossfade, with its catalogue length. */
  armNext: (url: string, durationMs: number) => void;
  /** Ask the element to play through this device, when it knows how. */
  setSinkId: (id: string) => void;
  /** Stop mirroring events (effect cleanup). */
  detach: () => void;
};

function clampVolume(volume: number): number {
  return Math.max(0, Math.min(1, volume));
}

/**
 * A fixture tone is much shorter than the catalogue row it stands in for,
 * so the element has to loop. A real file is the row's length (or the row
 * had no length and will adopt the file), so it plays once.
 */
export function loopsFixtureTone(catalogueMs: number, fileSeconds: number): boolean {
  if (!Number.isFinite(fileSeconds) || fileSeconds <= 0) {
    return true;
  }
  if (!(catalogueMs > 0)) {
    return false;
  }
  return fileSeconds * 1000 + 1_500 < catalogueMs;
}

export function createDemoAudio(element: DemoAudioElement, hooks: DemoAudioHooks): DemoAudio {
  element.loop = true;
  let trackDurationMs = 0;
  let virtualMs = 0;
  let lastElementSeconds = 0;
  let endedFired = false;
  let pendingSeekMs: number | undefined;
  let userVolume = 1;
  let gainDb: number | undefined;
  let crossfadeMs = 0;
  let armed: { url: string; durationMs: number } | undefined;
  let crossfadeFired = false;

  const writeVolume = () => {
    element.volume = outputVolume(userVolume, 'track', gainDb, undefined);
  };

  // The handoff belongs to this track only. A fixture tone has to keep
  // looping until then, or the virtual clock stops; once the next url is
  // due, wrapping would play the outgoing tone again under the fade.
  const maybeCrossfade = () => {
    const next = armed;
    if (
      crossfadeFired ||
      !(crossfadeMs > 0) ||
      next === undefined ||
      !(trackDurationMs > 0) ||
      virtualMs < trackDurationMs - crossfadeMs
    ) {
      return;
    }
    crossfadeFired = true;
    element.loop = false;
    hooks.onCrossfade?.(next.url);
  };

  const advance = () => {
    const now = element.currentTime;
    let delta = now - lastElementSeconds;
    if (delta < 0) {
      // A loop seam is a wrap near the element's end. A reload that resets
      // the element to 0 is not a seam, and must not jump the clock.
      const length = element.duration > 0 ? element.duration : 0;
      const seam = element.loop && length > 0 && lastElementSeconds > length - 1 && now < 1;
      delta = seam ? delta + length : 0;
    }
    lastElementSeconds = now;
    if (delta > 0) {
      virtualMs += delta * 1000;
    }
    maybeCrossfade();
    if (trackDurationMs > 0 && virtualMs >= trackDurationMs && !endedFired) {
      endedFired = true;
      hooks.onEnded();
      return;
    }
    hooks.onTime(Math.min(virtualMs, trackDurationMs), trackDurationMs);
  };

  const onTime = () => {
    advance();
  };
  const onWaiting = () => {
    hooks.onBuffering(true);
  };
  const onStalled = () => {
    hooks.onBuffering(true);
  };
  const onPlaying = () => {
    hooks.onBuffering(false);
  };
  const applyElementTime = (positionMs: number) => {
    const elementSeconds = element.duration > 0 ? element.duration : 0;
    const withinLoop = elementSeconds > 0 ? (positionMs / 1000) % elementSeconds : 0;
    element.currentTime = withinLoop;
    lastElementSeconds = withinLoop;
    if (elementSeconds > 0) {
      pendingSeekMs = undefined;
    }
  };
  const onLoadedMetadata = () => {
    if (pendingSeekMs !== undefined && element.duration > 0) {
      applyElementTime(pendingSeekMs);
    }
    hooks.onBuffering(false);
    /* The element's own length is the truth about the track when the
       catalogue had none to give. A real file's metadata, once adopted,
       becomes the clock's scale too. */
    const seconds = element.duration;
    element.loop = !crossfadeFired && loopsFixtureTone(trackDurationMs, seconds);
    if (trackDurationMs === 0 && Number.isFinite(seconds) && seconds > 0) {
      trackDurationMs = seconds * 1000;
      hooks.onDuration(trackDurationMs);
    }
  };
  const onElementEnded = () => {
    if (endedFired) {
      return;
    }
    endedFired = true;
    virtualMs = trackDurationMs > 0 ? trackDurationMs : virtualMs;
    hooks.onEnded();
  };
  const onError = () => {
    hooks.onError(element.error?.message ?? '');
  };
  const listeners: ReadonlyArray<[DemoAudioEvent, () => void]> = [
    ['timeupdate', onTime],
    ['waiting', onWaiting],
    ['stalled', onStalled],
    ['playing', onPlaying],
    ['loadedmetadata', onLoadedMetadata],
    ['ended', onElementEnded],
    ['error', onError],
  ];
  for (const [type, listener] of listeners) {
    element.addEventListener(type, listener);
  }

  return {
    load(url: string, durationMs: number) {
      trackDurationMs = durationMs;
      virtualMs = 0;
      endedFired = false;
      // The armed successor belonged to the track just replaced.
      crossfadeFired = false;
      armed = undefined;
      element.loop = true;
      element.src = url;
      element.currentTime = 0;
      lastElementSeconds = 0;
      pendingSeekMs = undefined;
    },
    setPlaying(playing: boolean) {
      if (playing) {
        void element.play();
        return;
      }
      element.pause();
    },
    seek(positionMs: number) {
      const cap = trackDurationMs > 0 ? trackDurationMs : positionMs;
      const clamped = Math.max(0, Math.min(cap, positionMs));
      virtualMs = clamped;
      endedFired = false;
      pendingSeekMs = clamped;
      applyElementTime(clamped);
      maybeCrossfade();
      hooks.onTime(clamped, trackDurationMs);
    },
    setVolume(volume: number) {
      userVolume = clampVolume(volume);
      writeVolume();
    },
    setGainDb(db: number | undefined) {
      gainDb = db;
      writeVolume();
    },
    setCrossfadeMs(ms: number) {
      crossfadeMs = ms;
    },
    armNext(url: string, durationMs: number) {
      armed = { url, durationMs };
    },
    setSinkId(id: string) {
      const sink = element.setSinkId;
      if (sink === undefined) {
        return;
      }
      // A device the element refuses must not become an unhandled rejection.
      // Playback keeps the output it already has.
      void sink.call(element, id).catch(() => undefined);
    },
    detach() {
      for (const [type, listener] of listeners) {
        element.removeEventListener(type, listener);
      }
      element.pause();
    },
  };
}
