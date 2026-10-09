import { useSyncExternalStore } from 'react';

/**
 * The player's frame port. The browser's requestAnimationFrame is the
 * scheduler when it exists; a timer is the fallback; tests drive a manual
 * one. Components never call the platform directly.
 */
export type FrameScheduler = {
  request(callback: (time: number) => void): number;
  cancel(handle: number): void;
};

/** The browser's frame scheduler, or undefined where there is none (tests, odd embeds). */
export function rafScheduler(): FrameScheduler | undefined {
  if (typeof globalThis.requestAnimationFrame !== 'function') {
    return undefined;
  }
  return {
    request: (callback) => globalThis.requestAnimationFrame(callback),
    cancel: (handle) => globalThis.cancelAnimationFrame(handle),
  };
}

/** A timer scheduler for engines without frames: smooth enough, never busy. */
export function intervalScheduler(intervalMs: number): FrameScheduler {
  return {
    /* The browser's timer id is a number; the Node-typed realm calls it a
       Timeout object — the same opaque handle either way. */
    request: (callback) =>
      globalThis.setTimeout(() => callback(globalThis.Date.now()), intervalMs) as unknown as number,
    cancel: (handle) => globalThis.clearTimeout(handle as unknown as ReturnType<typeof setTimeout>),
  };
}

/** The scheduler a composition root uses: frames where they exist, a timer otherwise. */
export function defaultScheduler(): FrameScheduler {
  return rafScheduler() ?? intervalScheduler(250);
}

export type PositionClock = {
  /**
   * The engine's latest truth: the track's duration, its position and
   * whether it is playing. The clock interpolates between these syncs.
   */
  sync(state: { durationMs: number; positionMs: number; playing: boolean }): void;
  /** The interpolated position, clamped into the track. */
  positionMs(): number;
  /** Hear every frame while the track plays; returns the unsubscribe. */
  subscribe(listener: () => void): () => void;
  /** Stop the walk and forget the listeners (effect cleanup). */
  detach(): void;
};

/**
 * A display clock for the playing position. The engine reports the truth a
 * few times a second; this clock walks it forward one frame at a time so
 * scrubbers, times and lyric lines move smoothly without the whole shell
 * re-rendering on every engine event. Deterministic under any scheduler.
 */
export function createPositionClock(scheduler: FrameScheduler): PositionClock {
  let durationMs = 0;
  let baseMs = 0;
  let playing = false;
  let lastFrameTime: number | undefined;
  let handle: number | undefined;
  const listeners = new Set<() => void>();

  const requestFrame = () => {
    if (handle !== undefined || !playing || listeners.size === 0) {
      return;
    }
    handle = scheduler.request(onFrame);
  };

  const onFrame = (time: number) => {
    handle = undefined;
    const elapsed = lastFrameTime === undefined ? 0 : Math.max(0, time - lastFrameTime);
    lastFrameTime = time;
    if (playing) {
      baseMs = Math.min(durationMs, baseMs + elapsed);
      for (const listener of listeners) {
        listener();
      }
    }
    requestFrame();
  };

  return {
    sync(state) {
      durationMs = Math.max(0, state.durationMs);
      baseMs = Math.max(0, Math.min(durationMs, state.positionMs));
      playing = state.playing;
      if (handle !== undefined) {
        scheduler.cancel(handle);
        handle = undefined;
      }
      /* The engine's truth re-centres the walk: the next frame only learns
         the new baseline instead of adding stale time to it. */
      lastFrameTime = undefined;
      requestFrame();
    },
    positionMs: () => baseMs,
    subscribe(listener) {
      listeners.add(listener);
      requestFrame();
      return () => {
        listeners.delete(listener);
        if (listeners.size === 0 && handle !== undefined) {
          scheduler.cancel(handle);
          handle = undefined;
        }
      };
    },
    detach() {
      if (handle !== undefined) {
        scheduler.cancel(handle);
        handle = undefined;
      }
      playing = false;
      listeners.clear();
    },
  };
}

const noClockSubscribe = () => () => undefined;

/**
 * The position a readout renders: the clock's walk when the composition
 * root wired one, the snapshot's position when it did not.
 */
export function usePositionMs(clock: PositionClock | undefined, fallbackMs: number): number {
  return useSyncExternalStore(clock?.subscribe ?? noClockSubscribe, clock?.positionMs ?? (() => fallbackMs));
}

/**
 * The scrubber's painted width. A whole percent of a long track is several
 * seconds, so the fill is not rounded: it grows with the clock.
 */
export function progressFillWidth(positionMs: number, durationMs: number): string {
  if (!(durationMs > 0) || !(positionMs > 0)) {
    return '0%';
  }
  return `${Math.min(100, (positionMs / durationMs) * 100)}%`;
}
