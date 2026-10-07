// Demo audio engine (fixture tones until CP-019 wires the real engine and
// the core's decision engine). All logic lives here so it runs in Vitest
// against a scripted element; the DOM element itself comes from the one
// dumb adapter in src/browser/audio-element.ts.
//
// The fixture tone is an 8-second loop; the catalogue says a track is
// 3:34. The element loops and this engine keeps a virtual clock that runs
// at the catalogue's scale, so progress advances like a real player and
// tracks hand over at their listed duration. CP-019/WP-030 replace the
// clock with the core's player state machine.

export type DemoAudioElement = {
  src: string;
  loop: boolean;
  volume: number;
  currentTime: number;
  duration: number;
  play(): void | Promise<void>;
  pause(): void;
  addEventListener(type: 'timeupdate' | 'ended', listener: () => void): void;
  removeEventListener(type: 'timeupdate' | 'ended', listener: () => void): void;
};

export type DemoAudioHooks = {
  /** Fired on the element's timeupdate with the virtual position in ms. */
  onTime: (positionMs: number, durationMs: number) => void;
  /** Fired once when the virtual clock reaches the track's duration. */
  onEnded: () => void;
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
  element.addEventListener('timeupdate', onTime);

  return {
    load(url: string, durationMs: number) {
      trackDurationMs = durationMs;
      virtualMs = 0;
      endedFired = false;
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
      element.removeEventListener('timeupdate', onTime);
      element.pause();
    },
  };
}
