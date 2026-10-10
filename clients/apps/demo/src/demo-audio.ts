import { outputVolume } from './gain.ts';

// Demo audio engine until CP-019 wires the real engine and the core's
// decision engine. All logic lives here so it runs in Vitest against a
// scripted element; the DOM element itself comes from the one dumb adapter
// in src/browser/audio-element.ts.
//
// A fixture tone is a short loop under a longer catalogue duration. A file
// whose length matches that row plays once, and the element's own end
// hands the track over. CP-019/WP-030 replace this clock.
//
// Crossfade: with a second element the engine performs a real overlap —
// the next track starts on the spare element at zero volume and the two
// ramp across the window, then the spare becomes the active one and the
// engine tells its owner (`onCrossfade`). With one element only (tests, or
// a caller that supplies the same instance twice) the hook fires at the
// window's start, which is the one-element handoff the owner must make.

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
   * With a second element: fired once when the overlap completes and the
   * successor is the active element — the owner moves its state to that
   * track. With one element: fired once per track when the virtual clock
   * enters the crossfade window and a next url is armed; the owner starts
   * that url, and this element is the one fading out.
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
  /** The url the active element is pointed at, or undefined when none is. */
  playingUrl: () => string | undefined;
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

export function createDemoAudio(
  element: DemoAudioElement,
  hooks: DemoAudioHooks,
  nextElement?: DemoAudioElement,
): DemoAudio {
  /* The same instance means the caller has only one element; the engine
     then falls back to the one-element handoff. */
  const spare = nextElement !== undefined && nextElement !== element ? nextElement : undefined;
  let activeIndex = 0;
  const act = (): DemoAudioElement => (activeIndex === 0 ? element : (spare as DemoAudioElement));
  const other = (): DemoAudioElement | undefined =>
    spare === undefined ? undefined : activeIndex === 0 ? spare : element;
  act().loop = true;

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
  /* A real overlap in progress: when it began on the virtual clock and how
     far the ramp has come. */
  let fading = false;
  let fadeStartedMs = 0;
  let fadeProgress = 0;
  /* The successor the in-progress fade is playing; set with `fading`. */
  let fadeNext: { url: string; durationMs: number } | undefined;

  const baseVolume = () => outputVolume(userVolume, 'track', gainDb, undefined);

  /* During a fade the active element ramps down and the spare ramps up from
     the same base; otherwise the active element takes the base and the spare
     rests silent. */
  const applyVolumes = () => {
    const base = baseVolume();
    act().volume = fading ? base * (1 - fadeProgress) : base;
    const o = other();
    if (o !== undefined) {
      o.volume = fading ? base * fadeProgress : 0;
    }
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
    act().loop = false;
    const o = other();
    if (o === undefined) {
      hooks.onCrossfade?.(next.url);
      return;
    }
    fading = true;
    fadeStartedMs = virtualMs;
    fadeProgress = 0;
    fadeNext = next;
    o.src = next.url;
    o.loop = false;
    o.currentTime = 0;
    o.volume = 0;
    void o.play();
    applyVolumes();
  };

  const completeFade = () => {
    const next = fadeNext as { url: string; durationMs: number };
    act().pause();
    activeIndex = activeIndex === 0 ? 1 : 0;
    fading = false;
    fadeProgress = 0;
    fadeNext = undefined;
    trackDurationMs = next.durationMs;
    const fresh = act();
    virtualMs = fresh.currentTime * 1000;
    lastElementSeconds = fresh.currentTime;
    endedFired = false;
    /* The new track gets its own handoff when its window arrives. */
    crossfadeFired = false;
    armed = undefined;
    fresh.loop = loopsFixtureTone(trackDurationMs, fresh.duration);
    applyVolumes();
    hooks.onCrossfade?.(next.url);
  };

  const advance = () => {
    const now = act().currentTime;
    let delta = now - lastElementSeconds;
    if (delta < 0) {
      // A loop seam is a wrap near the element's end. A reload that resets
      // the element to 0 is not a seam, and must not jump the clock.
      const length = act().duration > 0 ? act().duration : 0;
      const seam = act().loop && length > 0 && lastElementSeconds > length - 1 && now < 1;
      delta = seam ? delta + length : 0;
    }
    lastElementSeconds = now;
    if (delta > 0) {
      virtualMs += delta * 1000;
    }
    if (fading) {
      fadeProgress = Math.max(0, Math.min(1, (virtualMs - fadeStartedMs) / crossfadeMs));
      applyVolumes();
      if (fadeProgress >= 1) {
        completeFade();
      }
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
    const elementSeconds = act().duration > 0 ? act().duration : 0;
    const withinLoop = elementSeconds > 0 ? (positionMs / 1000) % elementSeconds : 0;
    act().currentTime = withinLoop;
    lastElementSeconds = withinLoop;
    if (elementSeconds > 0) {
      pendingSeekMs = undefined;
    }
  };
  const onLoadedMetadata = () => {
    if (pendingSeekMs !== undefined && act().duration > 0) {
      applyElementTime(pendingSeekMs);
    }
    hooks.onBuffering(false);
    /* The element's own length is the truth about the track when the
       catalogue had none to give. A real file's metadata, once adopted,
       becomes the clock's scale too. */
    const seconds = act().duration;
    act().loop = !crossfadeFired && loopsFixtureTone(trackDurationMs, seconds);
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
    hooks.onError(act().error?.message ?? '');
  };

  /* Both elements are bound, but only the active one's events move the
     engine: the spare's ticks during a fade belong to the next track. */
  const bind = (el: DemoAudioElement): ReadonlyArray<[DemoAudioEvent, () => void]> => {
    const listeners: ReadonlyArray<[DemoAudioEvent, () => void]> = [
      [
        'timeupdate',
        () => {
          if (el !== act()) {
            return;
          }
          onTime();
        },
      ],
      [
        'waiting',
        () => {
          if (el !== act()) {
            return;
          }
          onWaiting();
        },
      ],
      [
        'stalled',
        () => {
          if (el !== act()) {
            return;
          }
          onStalled();
        },
      ],
      [
        'playing',
        () => {
          if (el !== act()) {
            return;
          }
          onPlaying();
        },
      ],
      [
        'loadedmetadata',
        () => {
          if (el !== act()) {
            return;
          }
          onLoadedMetadata();
        },
      ],
      [
        'ended',
        () => {
          if (el !== act()) {
            return;
          }
          onElementEnded();
        },
      ],
      [
        'error',
        () => {
          if (el !== act()) {
            return;
          }
          onError();
        },
      ],
    ];
    for (const [type, listener] of listeners) {
      el.addEventListener(type, listener);
    }
    return listeners;
  };
  const bound = new Map<DemoAudioElement, ReadonlyArray<[DemoAudioEvent, () => void]>>();
  bound.set(element, bind(element));
  if (spare !== undefined) {
    bound.set(spare, bind(spare));
  }

  return {
    load(url: string, durationMs: number) {
      trackDurationMs = durationMs;
      virtualMs = 0;
      endedFired = false;
      // The armed successor belonged to the track just replaced.
      crossfadeFired = false;
      armed = undefined;
      if (fading) {
        /* A load mid-fade abandons the overlap: silence the spare. */
        other()?.pause();
      }
      fading = false;
      fadeProgress = 0;
      fadeNext = undefined;
      act().loop = true;
      act().src = url;
      act().currentTime = 0;
      lastElementSeconds = 0;
      pendingSeekMs = undefined;
      applyVolumes();
    },
    setPlaying(playing: boolean) {
      if (playing) {
        void act().play();
        return;
      }
      act().pause();
      if (fading) {
        /* Pausing mid-fade must silence the incoming side too. */
        other()?.pause();
      }
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
      applyVolumes();
    },
    setGainDb(db: number | undefined) {
      gainDb = db;
      applyVolumes();
    },
    setCrossfadeMs(ms: number) {
      crossfadeMs = ms;
    },
    armNext(url: string, durationMs: number) {
      armed = { url, durationMs };
    },
    setSinkId(id: string) {
      for (const el of [element, spare]) {
        if (el === undefined) {
          continue;
        }
        const sink = el.setSinkId;
        if (sink === undefined) {
          continue;
        }
        // A device the element refuses must not become an unhandled rejection.
        // Playback keeps the output it already has.
        void sink.call(el, id).catch(() => undefined);
      }
    },
    playingUrl() {
      const src = act().src;
      return src === '' ? undefined : src;
    },
    detach() {
      for (const [el, listeners] of bound) {
        for (const [type, listener] of listeners) {
          el.removeEventListener(type, listener);
        }
        el.pause();
      }
    },
  };
}
