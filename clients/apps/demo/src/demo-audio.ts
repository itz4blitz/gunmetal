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

  const advance = () => {
    const now = element.currentTime;
    let delta = now - lastElementSeconds;
    if (delta < 0) {
      // The element looped; carry the clock across the seam.
      delta += element.duration > 0 ? element.duration : 0;
    }
    lastElementSeconds = now;
    if (delta > 0) {
      virtualMs += delta * 1000;
    }
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
  const onLoadedMetadata = () => {
    hooks.onBuffering(false);
    /* The element's own length is the truth about the track when the
       catalogue had none to give. A real file's metadata, once adopted,
       becomes the clock's scale too. */
    const seconds = element.duration;
    element.loop = loopsFixtureTone(trackDurationMs, seconds);
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
      element.loop = true;
      element.src = url;
      element.currentTime = 0;
      lastElementSeconds = 0;
    },
    setPlaying(playing: boolean) {
      if (playing) {
        void element.play();
        return;
      }
      element.pause();
    },
    seek(positionMs: number) {
      const clamped = Math.max(0, Math.min(trackDurationMs, positionMs));
      virtualMs = clamped;
      endedFired = false;
      const elementSeconds = element.duration > 0 ? element.duration : 0;
      const withinLoop = elementSeconds > 0 ? (clamped / 1000) % elementSeconds : 0;
      element.currentTime = withinLoop;
      lastElementSeconds = withinLoop;
      hooks.onTime(clamped, trackDurationMs);
    },
    setVolume(volume: number) {
      element.volume = clampVolume(volume);
    },
    detach() {
      for (const [type, listener] of listeners) {
        element.removeEventListener(type, listener);
      }
      element.pause();
    },
  };
}
