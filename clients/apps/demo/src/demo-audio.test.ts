import { expect, test } from 'vitest';
import { createDemoAudio, type DemoAudioElement } from './demo-audio.ts';

type Listener = () => void;

class ScriptedElement implements DemoAudioElement {
  src = '';
  loop = false;
  volume = 1;
  currentTime = 0;
  duration = 0;
  playCalls = 0;
  pauseCalls = 0;
  listeners: Record<string, Listener[]> = {};

  play(): void {
    this.playCalls += 1;
  }

  pause(): void {
    this.pauseCalls += 1;
  }

  addEventListener(type: 'timeupdate' | 'ended', listener: Listener): void {
    (this.listeners[type] ??= []).push(listener);
  }

  removeEventListener(type: 'timeupdate' | 'ended', listener: Listener): void {
    this.listeners[type] = (this.listeners[type] ?? []).filter((entry) => entry !== listener);
  }

  /** Simulates decoder time advancing while playing. */
  tick(seconds: number): void {
    this.currentTime += seconds;
    for (const listener of this.listeners.timeupdate ?? []) {
      listener();
    }
  }

  has(type: 'timeupdate' | 'ended', listener: Listener): boolean {
    return (this.listeners[type] ?? []).includes(listener);
  }
}

test('load resets the clock, points the element at the fixture and sets loop', () => {
  const element = new ScriptedElement();
  const seen: Array<[number, number]> = [];
  const audio = createDemoAudio(element, { onTime: (p, d) => seen.push([p, d]), onEnded: () => {} });
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
  });
  audio.load('/media/audio/tone.wav', 30_000);
  element.tick(3); // 3s -> 3000ms
  element.tick(6); // crosses the 8s seam: 3 + 6 = 9s -> 9000ms
  expect(seen.at(-1)?.[0]).toStrictEqual(9000);
  expect(seen.at(-1)?.[1]).toStrictEqual(30_000);
  expect(ended).toStrictEqual(0);
  element.tick(22); // 31s total, past the 30s track
  expect(ended).toStrictEqual(1);
});

test('ended fires once per track until a seek rewinds', () => {
  const element = new ScriptedElement();
  const seen: number[] = [];
  const audio = createDemoAudio(element, { onTime: (p) => seen.push(p), onEnded: () => seen.push(-1) });
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
  const audio = createDemoAudio(element, { onTime: (p) => seen.push(p), onEnded: () => {} });
  audio.load('/media/audio/tone.wav', 214_000);
  audio.seek(214_500); // past the end -> clamps to the duration
  expect(seen.at(-1)).toStrictEqual(214_000);
  audio.seek(9_000); // 9s into a 3:34 track -> element sits at 1s of its 8s loop
  expect(element.currentTime).toBeCloseTo(1, 5);
});

test('setPlaying mirrors transport and volume is clamped to 0..1', () => {
  const element = new ScriptedElement();
  const audio = createDemoAudio(element, { onTime: () => {}, onEnded: () => {} });
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
  const audio = createDemoAudio(element, { onTime: (p) => seen.push(p), onEnded: () => {} });
  audio.load('/media/audio/tone.wav', 60_000);
  audio.detach();
  expect(element.pauseCalls).toStrictEqual(1);
  const before = seen.length;
  element.tick(3); // no listener bound: nothing reports
  expect(seen.length).toStrictEqual(before);
  // A fresh engine over the same element reports again (effect re-run).
  const second = createDemoAudio(element, { onTime: (p) => seen.push(p), onEnded: () => {} });
  second.load('/media/audio/tone.wav', 60_000);
  element.tick(2);
  expect(seen.at(-1)).toStrictEqual(2000);
});
