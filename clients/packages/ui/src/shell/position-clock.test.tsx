import { act, cleanup, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { Text, View } from 'react-native-web';
import {
  createPositionClock,
  defaultScheduler,
  intervalScheduler,
  rafScheduler,
  usePositionMs,
  type FrameScheduler,
} from './position-clock.ts';

afterEach(cleanup);

/** A scheduler a test drives by hand: nothing moves until flush says so. */
function manualScheduler(): FrameScheduler & { flush(time: number): void; pending(): number } {
  let next = 1;
  let callback: ((time: number) => void) | undefined;
  return {
    request(requested) {
      callback = requested;
      return next++;
    },
    cancel() {
      callback = undefined;
    },
    pending: () => (callback === undefined ? 0 : 1),
    flush(time: number) {
      const run = callback;
      callback = undefined;
      run?.(time);
    },
  };
}

test('the clock walks the position forward one frame at a time while playing', () => {
  const scheduler = manualScheduler();
  const clock = createPositionClock(scheduler);
  const seen: number[] = [];
  clock.subscribe(() => {
    seen.push(clock.positionMs());
  });
  expect(scheduler.pending()).toStrictEqual(0);
  clock.sync({ durationMs: 10_000, positionMs: 1000, playing: true });
  // A playing track with a listener keeps a frame on the wire.
  expect(scheduler.pending()).toStrictEqual(1);
  // The first frame only learns the clock's baseline.
  scheduler.flush(16);
  expect(seen).toStrictEqual([1000]);
  scheduler.flush(32);
  expect(seen).toStrictEqual([1000, 1016]);
  scheduler.flush(64);
  expect(seen).toStrictEqual([1000, 1016, 1048]);
  expect(clock.positionMs()).toStrictEqual(1048);
  // Pausing stops the walk and the frames.
  clock.sync({ durationMs: 10_000, positionMs: 1048, playing: false });
  expect(scheduler.pending()).toStrictEqual(0);
  scheduler.flush(80);
  expect(seen).toStrictEqual([1000, 1016, 1048]);
  expect(clock.positionMs()).toStrictEqual(1048);
  clock.detach();
});

test('the clock clamps at the track duration and rebases on every sync', () => {
  const scheduler = manualScheduler();
  const clock = createPositionClock(scheduler);
  clock.subscribe(() => {});
  clock.sync({ durationMs: 2000, positionMs: 1990, playing: true });
  scheduler.flush(16);
  scheduler.flush(100_000);
  expect(clock.positionMs()).toStrictEqual(2000);
  // The engine's truth wins over the walk: a sync re-centres the base.
  clock.sync({ durationMs: 2000, positionMs: 500, playing: true });
  expect(clock.positionMs()).toStrictEqual(500);
  scheduler.flush(32);
  expect(clock.positionMs()).toStrictEqual(500);
  scheduler.flush(64);
  expect(clock.positionMs()).toStrictEqual(532);
  // A seek while paused re-centres too.
  clock.sync({ durationMs: 2000, positionMs: 1500, playing: false });
  expect(clock.positionMs()).toStrictEqual(1500);
  clock.detach();
});

test('frames are only requested while a listener is subscribed', () => {
  const scheduler = manualScheduler();
  const clock = createPositionClock(scheduler);
  clock.sync({ durationMs: 10_000, positionMs: 0, playing: true });
  expect(scheduler.pending()).toStrictEqual(0);
  const stop = clock.subscribe(() => {});
  expect(scheduler.pending()).toStrictEqual(1);
  scheduler.flush(16);
  expect(clock.positionMs()).toStrictEqual(0);
  stop();
  expect(scheduler.pending()).toStrictEqual(0);
  scheduler.flush(32);
  expect(clock.positionMs()).toStrictEqual(0);
  // A second listener shares the same walk; leaving does not stop the other.
  const first = clock.subscribe(() => {});
  clock.subscribe(() => {});
  expect(scheduler.pending()).toStrictEqual(1);
  first();
  expect(scheduler.pending()).toStrictEqual(1);
  clock.detach();
});

test('detach cancels the pending frame and drops the listeners', () => {
  const scheduler = manualScheduler();
  const clock = createPositionClock(scheduler);
  const seen: number[] = [];
  clock.subscribe(() => {
    seen.push(clock.positionMs());
  });
  clock.sync({ durationMs: 10_000, positionMs: 0, playing: true });
  expect(scheduler.pending()).toStrictEqual(1);
  clock.detach();
  expect(scheduler.pending()).toStrictEqual(0);
  scheduler.flush(16);
  expect(seen).toStrictEqual([]);
  // A detached clock still answers reads with its last known position.
  expect(clock.positionMs()).toStrictEqual(0);
  // A fresh sync starts the next track once someone listens again.
  clock.sync({ durationMs: 10_000, positionMs: 40, playing: true });
  expect(scheduler.pending()).toStrictEqual(0);
  const again = clock.subscribe(() => {});
  expect(scheduler.pending()).toStrictEqual(1);
  again();
  clock.detach();
});

test('usePositionMs follows the clock while it walks and falls back without one', () => {
  const scheduler = manualScheduler();
  const clock = createPositionClock(scheduler);
  clock.sync({ durationMs: 10_000, positionMs: 1000, playing: true });
  function Probe({ source, fallback }: { source?: typeof clock; fallback: number }) {
    return (
      <View>
        <Text>{usePositionMs(source, fallback)}</Text>
      </View>
    );
  }
  const view = render(<Probe source={clock} fallback={7} />);
  expect(view.getByText('1000').textContent).toStrictEqual('1000');
  act(() => {
    scheduler.flush(16);
  });
  expect(view.getByText('1000').textContent).toStrictEqual('1000');
  act(() => {
    scheduler.flush(32);
  });
  expect(view.getByText('1016').textContent).toStrictEqual('1016');
  // Without a clock the readout shows the snapshot's position.
  view.rerender(<Probe fallback={45} />);
  expect(view.getByText('45').textContent).toStrictEqual('45');
  clock.detach();
});

test('rafScheduler wires the frame port to the browser, or reports that there is none', () => {
  const requested: Array<(time: number) => void> = [];
  const cancelled: number[] = [];
  vi.stubGlobal('requestAnimationFrame', (callback: (time: number) => void) => {
    requested.push(callback);
    return 41;
  });
  vi.stubGlobal('cancelAnimationFrame', (handle: number) => {
    cancelled.push(handle);
  });
  try {
    const scheduler = rafScheduler();
    if (scheduler === undefined) {
      throw new Error('the stubbed browser reported no frame scheduler');
    }
    const handle = scheduler.request((time) => time);
    expect(handle).toStrictEqual(41);
    expect(requested).toHaveLength(1);
    scheduler.cancel(41);
    expect(cancelled).toStrictEqual([41]);
  } finally {
    vi.unstubAllGlobals();
  }
  // Without a frame port the shell falls back to timers.
  vi.stubGlobal('requestAnimationFrame', undefined);
  try {
    expect(rafScheduler()).toStrictEqual(undefined);
  } finally {
    vi.unstubAllGlobals();
  }
});

test('intervalScheduler falls back to timers and cancels them', () => {
  vi.useFakeTimers();
  try {
    const scheduler = intervalScheduler(250);
    const seen: number[] = [];
    scheduler.request((time) => seen.push(time));
    expect(seen).toStrictEqual([]);
    vi.advanceTimersByTime(250);
    expect(seen).toHaveLength(1);
    const handle = scheduler.request(() => seen.push(-1));
    scheduler.cancel(handle);
    vi.advanceTimersByTime(1000);
    expect(seen).toHaveLength(1);
  } finally {
    vi.useRealTimers();
  }
});

test('defaultScheduler takes frames when the browser has them and a timer otherwise', () => {
  vi.stubGlobal('requestAnimationFrame', () => 1);
  vi.stubGlobal('cancelAnimationFrame', () => undefined);
  try {
    expect(defaultScheduler().request(() => {})).toStrictEqual(1);
  } finally {
    vi.unstubAllGlobals();
  }
  // The timer fallback needs the frame port gone, not merely stubbed: fake
  // timers first (they install their own rAF), then the property is removed.
  vi.useFakeTimers();
  const realRaf = globalThis.requestAnimationFrame;
  const realCancel = globalThis.cancelAnimationFrame;
  Reflect.deleteProperty(globalThis, 'requestAnimationFrame');
  try {
    expect(rafScheduler()).toStrictEqual(undefined);
    const seen: number[] = [];
    defaultScheduler().request((time) => seen.push(time));
    vi.advanceTimersByTime(250);
    expect(seen).toHaveLength(1);
  } finally {
    vi.useRealTimers();
    globalThis.cancelAnimationFrame = realCancel;
    globalThis.requestAnimationFrame = realRaf;
  }
});

test('a frame already in flight must not move a clock the engine has paused', () => {
  // A scheduler whose cancel cannot recall a frame already queued: the
  // browser can deliver one of those after a sync. The clock must shrug.
  let queued: ((time: number) => void) | undefined;
  const looseScheduler: FrameScheduler = {
    request: (callback) => {
      queued = callback;
      return 1;
    },
    cancel: () => undefined,
  };
  const clock = createPositionClock(looseScheduler);
  const seen: number[] = [];
  clock.subscribe(() => {
    seen.push(clock.positionMs());
  });
  clock.sync({ durationMs: 10_000, positionMs: 1000, playing: true });
  queued?.(16);
  expect(seen).toStrictEqual([1000]);
  // The engine pauses; the walk stops; the stale frame the scheduler still
  // had queued arrives and must change nothing.
  clock.sync({ durationMs: 10_000, positionMs: 1016, playing: false });
  const frame = queued;
  expect(frame === undefined).toStrictEqual(false);
  frame?.(32);
  expect(clock.positionMs()).toStrictEqual(1016);
  expect(seen).toStrictEqual([1000]);
  clock.detach();
});

test('a second listener books no second frame, and a paused clock cancels nothing on leave', () => {
  const scheduler = manualScheduler();
  const clock = createPositionClock(scheduler);
  clock.sync({ durationMs: 10_000, positionMs: 0, playing: true });
  const first = clock.subscribe(() => {});
  expect(scheduler.pending()).toStrictEqual(1);
  // The frame is already on the wire: a second listener adds no second one.
  const second = clock.subscribe(() => {});
  expect(scheduler.pending()).toStrictEqual(1);
  // Leaving while paused — no frame pending — cancels nothing and survives.
  clock.sync({ durationMs: 10_000, positionMs: 0, playing: false });
  expect(scheduler.pending()).toStrictEqual(0);
  first();
  second();
  expect(scheduler.pending()).toStrictEqual(0);
  clock.detach();
});
