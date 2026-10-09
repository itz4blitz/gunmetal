import { expect, test } from 'vitest';
import { linearFromDb, outputVolume } from './gain.ts';

// Expected amplitudes are 10^(dB/20), computed independently and written as
// literals. They are not taken from linearFromDb.

test('linearFromDb converts 0 dB to 1 and known steps to amplitude', () => {
  expect(linearFromDb(0)).toStrictEqual(1);
  // 10^(-6/20) is 0.5011872336272722, which rounds to 0.501.
  expect(linearFromDb(-6)).toStrictEqual(0.5011872336272722);
  expect(Number(linearFromDb(-6).toFixed(3))).toStrictEqual(0.501);
  // 10^(6/20) is 1.9952623149688795, which rounds to 2. The clamp is not here.
  expect(linearFromDb(6)).toStrictEqual(1.9952623149688795);
  expect(Number(linearFromDb(6).toFixed(0))).toStrictEqual(2);
  expect(linearFromDb(12)).toStrictEqual(3.9810717055349722);
  expect(linearFromDb(-12)).toStrictEqual(0.251188643150958);
  expect(linearFromDb(-18)).toStrictEqual(0.12589254117941673);
});

test('mode off ignores tags and returns the user volume clamped to 0..1', () => {
  expect(outputVolume(0.4, 'off', 12, -24)).toStrictEqual(0.4);
  expect(outputVolume(0, 'off', 6, -12)).toStrictEqual(0);
  expect(outputVolume(1, 'off', -12, 6)).toStrictEqual(1);
  expect(outputVolume(1.5, 'off', -6, 6)).toStrictEqual(1);
  expect(outputVolume(-0.2, 'off', 6, -12)).toStrictEqual(0);
  expect(outputVolume(Number.NaN, 'off', 6, -6)).toStrictEqual(0);
  expect(outputVolume(Number.POSITIVE_INFINITY, 'off', -12, 6)).toStrictEqual(1);
  expect(outputVolume(Number.NEGATIVE_INFINITY, 'off', 6, 6)).toStrictEqual(0);
  expect(outputVolume(0.4, 'off', Number.NaN, Number.POSITIVE_INFINITY)).toStrictEqual(0.4);
});

test('track mode uses the track tag, then the album tag, then 0 dB', () => {
  // 0.8 * 10^(-6/20). Album +6 dB must not win.
  expect(outputVolume(0.8, 'track', -6, 6)).toStrictEqual(0.4009497869018178);
  // 0.25 * 10^(-6/20). Distinct from the album-mode result for the same tags.
  expect(outputVolume(0.25, 'track', -6, 6)).toStrictEqual(0.12529680840681806);
  // Missing track falls through to the album tag: 0.4 * 10^(-6/20).
  expect(outputVolume(0.4, 'track', undefined, -6)).toStrictEqual(0.2004748934509089);
  // 0 dB is a real tag, not a missing one, so the album tag is not used.
  expect(outputVolume(0.4, 'track', 0, -6)).toStrictEqual(0.4);
  expect(outputVolume(1.2, 'track', 0, -6)).toStrictEqual(1);
  // Both missing is unity: the user volume, then clamped.
  expect(outputVolume(0.4, 'track', undefined, undefined)).toStrictEqual(0.4);
  expect(outputVolume(1.2, 'track', undefined, undefined)).toStrictEqual(1);
});

test('album mode uses the album tag, then the track tag, then 0 dB', () => {
  // 0.25 * 10^(6/20). Track -6 dB must not win.
  expect(outputVolume(0.25, 'album', -6, 6)).toStrictEqual(0.4988155787422199);
  expect(outputVolume(0.8, 'album', -6, 6)).toStrictEqual(1);
  // Missing album falls through to the track tag: 0.8 * 10^(-6/20).
  expect(outputVolume(0.8, 'album', -6, undefined)).toStrictEqual(0.4009497869018178);
  // 0 dB album is a real tag, so the track tag is not used.
  expect(outputVolume(0.4, 'album', -6, 0)).toStrictEqual(0.4);
  expect(outputVolume(0.4, 'album', undefined, undefined)).toStrictEqual(0.4);
  expect(outputVolume(1.2, 'album', undefined, undefined)).toStrictEqual(1);
});

test('a non-finite tag is ignored and the fallback is used', () => {
  expect(outputVolume(0.8, 'track', Number.NaN, -6)).toStrictEqual(0.4009497869018178);
  expect(outputVolume(0.8, 'track', Number.POSITIVE_INFINITY, -6)).toStrictEqual(0.4009497869018178);
  expect(outputVolume(0.8, 'track', Number.NEGATIVE_INFINITY, -6)).toStrictEqual(0.4009497869018178);
  expect(outputVolume(0.25, 'album', 6, Number.NaN)).toStrictEqual(0.4988155787422199);
  expect(outputVolume(0.25, 'album', 6, Number.POSITIVE_INFINITY)).toStrictEqual(0.4988155787422199);
  expect(outputVolume(0.25, 'album', 6, Number.NEGATIVE_INFINITY)).toStrictEqual(0.4988155787422199);
  expect(outputVolume(0.4, 'track', Number.NaN, Number.POSITIVE_INFINITY)).toStrictEqual(0.4);
  expect(outputVolume(0.4, 'album', Number.NEGATIVE_INFINITY, Number.NaN)).toStrictEqual(0.4);
});

test('the applied tag is clamped to -12..+6 dB before it is converted', () => {
  // 0.25 * 10^(6/20). +12 dB is not 0.25 * 10^(12/20) (0.9952679263837431).
  expect(outputVolume(0.25, 'track', 12, undefined)).toStrictEqual(0.4988155787422199);
  expect(outputVolume(0.25, 'track', 6, undefined)).toStrictEqual(0.4988155787422199);
  expect(outputVolume(0.2, 'track', 6.1, undefined)).toStrictEqual(0.39905246299377595);
  expect(outputVolume(0.2, 'track', 6, undefined)).toStrictEqual(0.39905246299377595);
  // Inside the cap, +5.9 dB is not snapped to +6.
  expect(outputVolume(0.3, 'track', 5.9, undefined)).toStrictEqual(0.5917268208344562);
  // 0.5 * 10^(-12/20). -24 dB is not 0.5 * 10^(-24/20) (0.031547867224009665).
  expect(outputVolume(0.5, 'track', -24, undefined)).toStrictEqual(0.125594321575479);
  expect(outputVolume(0.5, 'track', -12, undefined)).toStrictEqual(0.125594321575479);
  expect(outputVolume(0.5, 'album', undefined, -12.1)).toStrictEqual(0.125594321575479);
  // Inside the floor, -11.9 dB is not snapped to -12.
  expect(outputVolume(0.3, 'album', undefined, -11.9)).toStrictEqual(0.07622918116647916);
});

test('the product with the user volume is clamped to 0..1, and the user volume is not pre-clamped', () => {
  // 1 * 10^(6/20) is about 2, so the element amplitude stops at 1.
  expect(outputVolume(1, 'track', 6, undefined)).toStrictEqual(1);
  expect(outputVolume(0.7, 'track', 6, undefined)).toStrictEqual(1);
  expect(outputVolume(0, 'track', 6, undefined)).toStrictEqual(0);
  expect(outputVolume(-0.4, 'track', 0, undefined)).toStrictEqual(0);
  // 1.5 * 10^(-6/20) stays under 1, so a pre-clamp of the user volume to 1 would be wrong.
  expect(outputVolume(1.5, 'track', -6, undefined)).toStrictEqual(0.7517808504409084);
  // 2 * 10^(-6/20) is just over 1.
  expect(outputVolume(2, 'track', -6, undefined)).toStrictEqual(1);
  // 2 * 10^(-12/20) stays under 1. Pre-clamping the user volume would yield 10^(-12/20).
  expect(outputVolume(2, 'album', undefined, -12)).toStrictEqual(0.502377286301916);
  expect(outputVolume(1.2, 'album', undefined, 0)).toStrictEqual(1);
  expect(outputVolume(Number.NaN, 'track', 0, undefined)).toStrictEqual(0);
  expect(outputVolume(Number.POSITIVE_INFINITY, 'track', -12, undefined)).toStrictEqual(1);
  expect(outputVolume(Number.NEGATIVE_INFINITY, 'album', undefined, 6)).toStrictEqual(0);
});
