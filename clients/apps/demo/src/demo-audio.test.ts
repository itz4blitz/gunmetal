import { expect, test } from 'vitest';
import {
  createDemoAudio,
  loopsFixtureTone,
  type DemoAudio,
  type DemoAudioElement,
  type DemoAudioHooks,
} from './demo-audio.ts';

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

class SinkElement extends ScriptedElement {
  readonly sinks: string[] = [];

  setSinkId(id: string): Promise<void> {
    this.sinks.push(id);
    return Promise.resolve();
  }
}

class RefusingSink extends ScriptedElement {
  readonly sinks: string[] = [];

  setSinkId(id: string): Promise<void> {
    this.sinks.push(id);
    return Promise.reject(new Error('device gone'));
  }
}

function quietHooks(onCrossfade?: (url: string) => void): DemoAudioHooks {
  const hooks: DemoAudioHooks = {
    onTime: () => {},
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  };
  if (onCrossfade !== undefined) {
    hooks.onCrossfade = onCrossfade;
  }
  return hooks;
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

test('a reload that resets the element is not a loop seam', () => {
  const element = new ScriptedElement();
  element.duration = 214;
  const seen: number[] = [];
  const audio = createDemoAudio(element, {
    onTime: (positionMs) => seen.push(positionMs),
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  audio.load('/media/audio/tone.wav', 214_000);
  audio.seek(42_000);
  element.currentTime = 0;
  element.tick(0);
  expect(seen.at(-1)).toStrictEqual(42_000);
  audio.seek(42_000);
  element.loop = false;
  element.currentTime = 0;
  element.tick(0);
  expect(seen.at(-1)).toStrictEqual(42_000);

  const seam = new ScriptedElement();
  seam.duration = 8;
  const walked: number[] = [];
  const walking = createDemoAudio(seam, {
    onTime: (positionMs) => walked.push(positionMs),
    onEnded: () => {},
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
  });
  walking.load('/media/audio/tone.wav', 30_000);
  seam.currentTime = 7.5;
  seam.tick(0);
  seam.currentTime = 2;
  seam.tick(0);
  expect(walked.at(-1)).toStrictEqual(7500);
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
  audio.load('/media/library/resume.flac', 178_000);
  element.duration = 0;
  audio.seek(42_000);
  expect(element.currentTime).toStrictEqual(0);
  element.duration = 178;
  element.fire('loadedmetadata');
  expect(element.currentTime).toStrictEqual(42);
  element.fire('ended');
  element.fire('ended');
  expect(ended).toStrictEqual([1]);
  audio.load('/media/library/empty.flac', 0);
  element.duration = 0;
  element.fire('ended');
  expect(ended).toStrictEqual([1, 1]);
});

test('setGainDb multiplies the clamped decibel factor into the user volume', () => {
  const element = new ScriptedElement();
  const audio: DemoAudio = createDemoAudio(element, quietHooks());
  // Independent reference, not read from the engine: clamp dB to [-12, +6],
  // linear = 10^(dB/20), product with the user volume clamped to [0, 1].
  // Undefined and non-finite gain leave the user volume alone. The user
  // volume itself is clamped before the multiply.
  audio.setGainDb(-12);
  expect(element.volume).toStrictEqual(0.251188643150958);
  audio.setGainDb(-40);
  expect(element.volume).toStrictEqual(0.251188643150958);
  audio.setVolume(0.5);
  audio.setGainDb(6);
  expect(element.volume).toStrictEqual(0.9976311574844398);
  audio.setGainDb(18);
  expect(element.volume).toStrictEqual(0.9976311574844398);
  audio.setVolume(1);
  expect(element.volume).toStrictEqual(1);
  audio.setVolume(1.7);
  audio.setGainDb(-6);
  expect(element.volume).toStrictEqual(0.5011872336272722);
  audio.setVolume(0.4);
  audio.setGainDb(undefined);
  expect(element.volume).toStrictEqual(0.4);
  audio.setGainDb(Number.NaN);
  expect(element.volume).toStrictEqual(0.4);
  audio.setGainDb(Number.POSITIVE_INFINITY);
  expect(element.volume).toStrictEqual(0.4);
  audio.setGainDb(Number.NEGATIVE_INFINITY);
  expect(element.volume).toStrictEqual(0.4);
  audio.setGainDb(0);
  expect(element.volume).toStrictEqual(0.4);
  audio.setVolume(0);
  audio.setGainDb(6);
  expect(element.volume).toStrictEqual(0);
  // A new track keeps the mix; load does not clear the user volume or the gain.
  audio.load('/media/audio/tone.wav', 30_000);
  expect(element.volume).toStrictEqual(0);
});

test('crossfade fires once inside the end window and does not loop the outgoing element', () => {
  const element = new ScriptedElement();
  const marks: string[] = [];
  const audio = createDemoAudio(element, {
    onTime: (positionMs) => {
      marks.push(`time:${positionMs}`);
    },
    onEnded: () => {
      marks.push('ended');
    },
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
    onCrossfade: (url) => {
      marks.push(`fade:${url}`);
    },
  });
  audio.load('/media/audio/tone.wav', 10_000);
  audio.setCrossfadeMs(2_000);
  audio.armNext('/media/audio/stale.wav', 1_000);
  audio.armNext('/media/audio/next.wav', 4_000);
  element.tick(7);
  expect(marks).toStrictEqual(['time:7000']);
  expect(element.loop).toStrictEqual(true);
  element.currentTime = 8;
  element.tick(0);
  expect(marks).toStrictEqual(['time:7000', 'fade:/media/audio/next.wav', 'time:8000']);
  expect(element.loop).toStrictEqual(false);
  element.tick(1);
  expect(marks).toStrictEqual(['time:7000', 'fade:/media/audio/next.wav', 'time:8000', 'time:9000']);
  // Seeking back out of the window does not arm another handoff.
  audio.seek(1_000);
  expect(marks).toStrictEqual(['time:7000', 'fade:/media/audio/next.wav', 'time:8000', 'time:9000', 'time:1000']);
  expect(element.loop).toStrictEqual(false);
  element.tick(8);
  expect(marks.at(-1)).toStrictEqual('time:9000');
  expect(marks.filter((mark) => mark.startsWith('fade:'))).toStrictEqual(['fade:/media/audio/next.wav']);
  // A metadata event after the handoff must not turn the fixture loop back on.
  element.duration = 8;
  element.fire('loadedmetadata');
  expect(element.loop).toStrictEqual(false);
  // The next load is a new track: the setting stays, the arm and the once-flag do not.
  audio.load('/media/audio/other.wav', 10_000);
  expect(element.loop).toStrictEqual(true);
  element.tick(9);
  expect(marks.filter((mark) => mark.startsWith('fade:'))).toStrictEqual(['fade:/media/audio/next.wav']);
  audio.armNext('/media/audio/third.wav', 5_000);
  element.tick(0);
  expect(marks.filter((mark) => mark.startsWith('fade:'))).toStrictEqual([
    'fade:/media/audio/next.wav',
    'fade:/media/audio/third.wav',
  ]);
  expect(element.loop).toStrictEqual(false);
});

test('a tick that crosses the end still crossfades once before the track ends', () => {
  const element = new ScriptedElement();
  const marks: string[] = [];
  const audio = createDemoAudio(element, {
    onTime: (positionMs) => {
      marks.push(`time:${positionMs}`);
    },
    onEnded: () => {
      marks.push('ended');
    },
    onBuffering: () => {},
    onError: () => {},
    onDuration: () => {},
    onCrossfade: (url) => {
      marks.push(`fade:${url}`);
    },
  });
  audio.load('/media/audio/tone.wav', 10_000);
  audio.setCrossfadeMs(2_000);
  audio.armNext('/media/audio/next.wav', 4_000);
  element.tick(12);
  expect(marks).toStrictEqual(['fade:/media/audio/next.wav', 'ended']);
  expect(element.loop).toStrictEqual(false);
  element.tick(1);
  expect(marks).toStrictEqual(['fade:/media/audio/next.wav', 'ended', 'time:10000']);
});

test('crossfade stays quiet until a positive window, an armed url and the end overlap', () => {
  const quiet = (durationMs: number) => {
    const element = new ScriptedElement();
    const faded: string[] = [];
    const audio = createDemoAudio(
      element,
      quietHooks((url) => faded.push(url)),
    );
    audio.load('/media/audio/tone.wav', durationMs);
    return { element, audio, faded };
  };

  const off = quiet(10_000);
  off.audio.setCrossfadeMs(0);
  off.audio.armNext('/media/audio/next.wav', 4_000);
  off.element.tick(10);
  expect(off.faded).toStrictEqual([]);
  expect(off.element.loop).toStrictEqual(true);

  const negative = quiet(10_000);
  negative.audio.setCrossfadeMs(-500);
  negative.audio.armNext('/media/audio/next.wav', 4_000);
  negative.element.tick(9);
  expect(negative.faded).toStrictEqual([]);
  expect(negative.element.loop).toStrictEqual(true);

  const unarmed = quiet(10_000);
  unarmed.audio.setCrossfadeMs(2_000);
  unarmed.element.tick(9);
  expect(unarmed.faded).toStrictEqual([]);
  expect(unarmed.element.loop).toStrictEqual(true);

  const unknown = quiet(0);
  unknown.audio.setCrossfadeMs(2_000);
  unknown.audio.armNext('/media/audio/next.wav', 4_000);
  unknown.element.tick(9);
  expect(unknown.faded).toStrictEqual([]);
  expect(unknown.element.loop).toStrictEqual(true);

  const early = quiet(10_000);
  early.audio.setCrossfadeMs(2_000);
  early.audio.armNext('/media/audio/next.wav', 4_000);
  early.element.tick(7);
  expect(early.faded).toStrictEqual([]);
  expect(early.element.loop).toStrictEqual(true);

  // Arming or widening the gap does not notify until the clock is sampled again.
  const waiting = quiet(10_000);
  waiting.element.tick(9);
  waiting.audio.setCrossfadeMs(2_000);
  waiting.audio.armNext('/media/audio/next.wav', 4_000);
  expect(waiting.faded).toStrictEqual([]);
  waiting.element.tick(0);
  expect(waiting.faded).toStrictEqual(['/media/audio/next.wav']);
  expect(waiting.element.loop).toStrictEqual(false);

  // A gap longer than the track covers position 0.
  const whole = quiet(10_000);
  whole.audio.setCrossfadeMs(20_000);
  whole.audio.armNext('/media/audio/next.wav', 4_000);
  whole.element.tick(0);
  expect(whole.faded).toStrictEqual(['/media/audio/next.wav']);
  expect(whole.element.loop).toStrictEqual(false);

  // Seeking into the window notifies; seeking there again does not.
  const sought = quiet(10_000);
  sought.audio.setCrossfadeMs(2_000);
  sought.audio.armNext('/media/audio/next.wav', 4_000);
  sought.audio.seek(1_000);
  expect(sought.faded).toStrictEqual([]);
  sought.audio.seek(8_000);
  expect(sought.faded).toStrictEqual(['/media/audio/next.wav']);
  expect(sought.element.loop).toStrictEqual(false);
  sought.audio.seek(9_000);
  expect(sought.faded).toStrictEqual(['/media/audio/next.wav']);

  // Re-arming after the handoff does not fire a second time on this track.
  sought.audio.armNext('/media/audio/later.wav', 3_000);
  sought.element.tick(0);
  expect(sought.faded).toStrictEqual(['/media/audio/next.wav']);

  // No hook is not a crash: the outgoing element still stops looping.
  const silent = new ScriptedElement();
  const silentAudio = createDemoAudio(silent, quietHooks());
  silentAudio.load('/media/audio/tone.wav', 10_000);
  silentAudio.setCrossfadeMs(2_000);
  silentAudio.armNext('/media/audio/next.wav', 4_000);
  silent.tick(8);
  expect(silent.loop).toStrictEqual(false);
});

test('setSinkId forwards the id when the element can take one and leaves the element alone when it cannot', () => {
  const element = new ScriptedElement();
  const audio = createDemoAudio(element, quietHooks());
  const before = {
    src: element.src,
    loop: element.loop,
    volume: element.volume,
    currentTime: element.currentTime,
  };
  audio.setSinkId('speakers');
  audio.setSinkId('');
  expect({
    src: element.src,
    loop: element.loop,
    volume: element.volume,
    currentTime: element.currentTime,
  }).toStrictEqual(before);

  const routed = new SinkElement();
  const routedAudio = createDemoAudio(routed, quietHooks());
  routedAudio.setSinkId('device-1');
  routedAudio.setSinkId('');
  expect(routed.sinks).toStrictEqual(['device-1', '']);
});

test('a sink the element refuses is still requested and does not escape as a rejection', async () => {
  const element = new RefusingSink();
  const audio = createDemoAudio(element, quietHooks());
  audio.setSinkId('gone');
  expect(element.sinks).toStrictEqual(['gone']);
  await Promise.resolve();
});
