import { expect, test } from 'vitest';
import { createDemoAudio, loopsFixtureTone, type DemoAudioElement } from './demo-audio.ts';

type Listener = () => void;

class ScriptedElement implements DemoAudioElement {
  src = '';
  loop = false;
  volume = 1;
  currentTime = 0;
  duration = 0;
  error: { message: string } | null = null;
  playCalls = 0;
  pauseCalls = 0;
  listeners: Record<string, Listener[]> = {};

  play(): void {
    this.playCalls += 1;
  }

  pause(): void {
    this.pauseCalls += 1;
  }

  addEventListener(type: string, listener: Listener): void {
    (this.listeners[type] ??= []).push(listener);
  }

  removeEventListener(type: string, listener: Listener): void {
    this.listeners[type] = (this.listeners[type] ?? []).filter((entry) => entry !== listener);
  }

  /** Simulates decoder time advancing while playing. */
  tick(seconds: number): void {
    this.currentTime += seconds;
    for (const listener of this.listeners.timeupdate ?? []) {
      listener();
    }
  }

  /** Fires one of the engine's lifecycle events. */
  fire(type: string): void {
    for (const listener of this.listeners[type] ?? []) {
      listener();
    }
  }

  has(type: string, listener: Listener): boolean {
    return (this.listeners[type] ?? []).includes(listener);
  }
}

test('load resets the clock, points the element at the fixture and sets loop', () => {
  const element = new ScriptedElement();
  const seen: Array<[number, number]> = [];
  const audio = createDemoAudio(element, {
    onTime: (p, d) => seen.push([p, d]),
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/demo-album-01.wav', 214_000);
  expect(element.src).toStrictEqual('/media/audio/demo-album-01.wav');
  expect(element.loop).toStrictEqual(true);
  expect(element.currentTime).toStrictEqual(0);
  // Loading reports the reset position once through the normal path.
  element.tick(0);
  expect(seen).toStrictEqual([[0, 214_000]]);
});

test('the virtual clock runs at catalogue scale across element loops', () => {
  const element = new ScriptedElement();
  element.duration = 8; // 8-second tone loop
  const seen: Array<[number, number]> = [];
  let ended = 0;
  const audio = createDemoAudio(element, {
    onTime: (p, d) => seen.push([p, d]),
    onEnded: () => {
      ended += 1;
    },
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 30_000);
  element.tick(3); // 3s -> 3000ms
  expect(seen.at(-1)?.[0]).toStrictEqual(3000);
  // The element reaches its loop's end and wraps: the clock carries the
  // seam (3s heard, then 4.5s of this pass reported, then the wrap adds
  // 0.7s more of the next pass).
  element.currentTime = 7.5;
  element.tick(0);
  expect(seen.at(-1)?.[0]).toStrictEqual(7500);
  element.currentTime = 0.2;
  element.tick(0);
  expect(seen.at(-1)?.[0]).toStrictEqual(8200);
  expect(seen.at(-1)?.[1]).toStrictEqual(30_000);
  expect(ended).toStrictEqual(0);
  element.tick(6); // 6.2s of this pass: 14.2s total
  expect(seen.at(-1)?.[0]).toStrictEqual(14_200);
  element.tick(22); // 28.2s of this pass: 36.2s total, past the 30s track
  expect(ended).toStrictEqual(1);
});

test('ended fires once per track until a seek rewinds', () => {
  const element = new ScriptedElement();
  const seen: number[] = [];
  const audio = createDemoAudio(element, {
    onTime: (p) => seen.push(p),
    onEnded: () => seen.push(-1),
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 10_000);
  element.tick(11); // past the end: ended fires (position reported clamped on later ticks)
  element.tick(1); // still past the end: no second ended, position stays clamped
  audio.seek(1000);
  expect(seen).toStrictEqual([-1, 10_000, 1000]);
  element.tick(10);
  expect(seen.at(-1)).toStrictEqual(-1);
});

test('seek clamps into the track and repositions the element inside its loop', () => {
  const element = new ScriptedElement();
  element.duration = 8;
  const seen: number[] = [];
  const audio = createDemoAudio(element, {
    onTime: (p) => seen.push(p),
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 214_000);
  audio.seek(214_500); // past the end -> clamps to the duration
  expect(seen.at(-1)).toStrictEqual(214_000);
  audio.seek(9_000); // 9s into a 3:34 track -> element sits at 1s of its 8s loop
  expect(element.currentTime).toBeCloseTo(1, 5);
});

test('a rewound element with no known length carries nothing across the seam', () => {
  const element = new ScriptedElement();
  const seen: number[] = [];
  const audio = createDemoAudio(element, {
    onTime: (p) => seen.push(p),
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 60_000);
  element.duration = 0; // no length to carry across with
  element.tick(2);
  expect(seen).toStrictEqual([2000]);
  // The element went backwards without a seek: with no length there is no
  // seam to carry across, so the clock stands still and reports as it is.
  element.currentTime = 1;
  element.tick(0);
  expect(seen).toStrictEqual([2000, 2000]);
});

test('setPlaying mirrors transport and volume is clamped to 0..1', () => {
  const element = new ScriptedElement();
  const audio = createDemoAudio(element, {
    onTime: () => {},
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.setPlaying(true);
  audio.setPlaying(false);
  expect(element.playCalls).toStrictEqual(1);
  expect(element.pauseCalls).toStrictEqual(1);
  audio.setVolume(1.7);
  expect(element.volume).toStrictEqual(1);
  audio.setVolume(-3);
  expect(element.volume).toStrictEqual(0);
  audio.setVolume(0.4);
  expect(element.volume).toBeCloseTo(0.4, 5);
});

test('detach pauses, unbinds and re-binding works after detach', () => {
  const element = new ScriptedElement();
  const seen: number[] = [];
  const audio = createDemoAudio(element, {
    onTime: (p) => seen.push(p),
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 60_000);
  audio.detach();
  expect(element.pauseCalls).toStrictEqual(1);
  const before = seen.length;
  element.tick(3); // no listener bound: nothing reports
  expect(seen.length).toStrictEqual(before);
  // A fresh engine over the same element reports again (effect re-run).
  const second = createDemoAudio(element, {
    onTime: (p) => seen.push(p),
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  second.load('/media/audio/tone.wav', 60_000);
  element.tick(2);
  expect(seen.at(-1)).toStrictEqual(2000);
});

test('waiting and stalled report buffering until the engine reports ready again', () => {
  const element = new ScriptedElement();
  const buffering: boolean[] = [];
  const audio = createDemoAudio(element, {
    onTime: () => {},
    onEnded: () => {},
    onBuffering: (value) => buffering.push(value),
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 30_000);
  element.fire('waiting');
  element.fire('stalled');
  expect(buffering).toStrictEqual([true, true]);
  element.fire('playing');
  expect(buffering.at(-1)).toStrictEqual(false);
  element.fire('loadedmetadata');
  expect(buffering).toStrictEqual([true, true, false, false]);
  // A detached engine stops reporting either way.
  audio.detach();
  element.fire('waiting');
  expect(buffering).toStrictEqual([true, true, false, false]);
});

test('an element error reports its message, with a quiet fallback when it has none', () => {
  const element = new ScriptedElement();
  const errors: string[] = [];
  const audio = createDemoAudio(element, {
    onTime: () => {},
    onEnded: () => {},
    onBuffering: () => {},
    onError: (message) => errors.push(message),
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 30_000);
  element.error = { message: 'DEMUXER_ERROR_COULD_NOT_OPEN' };
  element.fire('error');
  expect(errors).toStrictEqual(['DEMUXER_ERROR_COULD_NOT_OPEN']);
  element.error = null;
  element.fire('error');
  expect(errors.at(-1)).toStrictEqual('');
});

test('loadedmetadata adopts the element duration only when the catalogue gave none', () => {
  const element = new ScriptedElement();
  const durations: number[] = [];
  const times: number[] = [];
  const audio = createDemoAudio(element, {
    onTime: (ms) => times.push(ms),
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: (ms) => durations.push(ms),
  });
  // Virtual scale: the engine keeps the catalogue's clock and says nothing.
  audio.load('/media/audio/tone.wav', 214_000);
  element.duration = 8;
  element.fire('loadedmetadata');
  expect(durations).toStrictEqual([]);
  // Real scale: the catalogue gave no duration, so the element's is the truth.
  audio.load('/media/audio/live.wav', 0);
  element.duration = 12.5;
  element.fire('loadedmetadata');
  expect(durations).toStrictEqual([12_500]);
  // A zero or non-finite element duration is never adopted.
  audio.load('/media/audio/broken.wav', 0);
  element.duration = Number.NaN;
  element.fire('loadedmetadata');
  expect(durations).toStrictEqual([12_500]);
  element.duration = 0;
  element.fire('loadedmetadata');
  expect(durations).toStrictEqual([12_500]);
  // No catalogue length and no finite file length: keep the fixture loop.
  expect(element.loop).toStrictEqual(true);
  // The adopted duration is the clock's scale: a seek clamps into it.
  audio.load('/media/audio/live.wav', 0);
  element.duration = 12.5;
  element.fire('loadedmetadata');
  // The catalogue gave no length, so the file plays once.
  expect(element.loop).toStrictEqual(false);
  audio.seek(10_000);
  expect(times.at(-1)).toStrictEqual(10_000);
  element.tick(2);
  expect(times.at(-1)).toStrictEqual(12_000);
});

test('a short fixture keeps looping and a full-length file plays once', () => {
  expect(loopsFixtureTone(214_000, Number.NaN)).toStrictEqual(true);
  expect(loopsFixtureTone(214_000, Number.POSITIVE_INFINITY)).toStrictEqual(true);
  expect(loopsFixtureTone(214_000, 0)).toStrictEqual(true);
  expect(loopsFixtureTone(214_000, -1)).toStrictEqual(true);
  expect(loopsFixtureTone(0, 12.5)).toStrictEqual(false);
  expect(loopsFixtureTone(-5, 12.5)).toStrictEqual(false);
  expect(loopsFixtureTone(214_000, 8)).toStrictEqual(true);
  expect(loopsFixtureTone(10_000, 8)).toStrictEqual(true);
  expect(loopsFixtureTone(180_000, 180)).toStrictEqual(false);
  expect(loopsFixtureTone(10_000, 8.6)).toStrictEqual(false);

  const element = new ScriptedElement();
  const ended: number[] = [];
  const audio = createDemoAudio(element, {
    onTime: () => {},
    onEnded: () => ended.push(1),
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 214_000);
  element.duration = 8;
  element.fire('loadedmetadata');
  expect(element.loop).toStrictEqual(true);
  audio.load('/media/library/cccccccccccccccc.flac', 178_000);
  expect(element.loop).toStrictEqual(true);
  element.duration = 178;
  element.fire('loadedmetadata');
  expect(element.loop).toStrictEqual(false);
  audio.seek(90_000);
  expect(element.currentTime).toStrictEqual(90);
  element.fire('ended');
  element.fire('ended');
  expect(ended).toStrictEqual([1]);
  audio.load('/media/library/empty.flac', 0);
  element.duration = 0;
  element.fire('ended');
  expect(ended).toStrictEqual([1, 1]);
});
