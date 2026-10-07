import { expect, test } from 'vitest';
import { defaultVolumeMemory, parseVolume, serializeVolume, type VolumeMemory } from './volume-store.ts';

test('a missing or unread entry falls back to the default level, unmuted', () => {
  expect(defaultVolumeMemory).toStrictEqual({ volume: 0.8, muted: false });
  expect(parseVolume(null)).toStrictEqual({ volume: 0.8, muted: false });
  expect(parseVolume('')).toStrictEqual({ volume: 0.8, muted: false });
  expect(parseVolume('not json')).toStrictEqual({ volume: 0.8, muted: false });
  expect(parseVolume('[]')).toStrictEqual({ volume: 0.8, muted: false });
  expect(parseVolume('null')).toStrictEqual({ volume: 0.8, muted: false });
  expect(parseVolume('{"volume":"0.4"}')).toStrictEqual({ volume: 0.8, muted: false });
  // A broken mute does not discard a readable level.
  expect(parseVolume('{"volume":0.4,"muted":"yes"}')).toStrictEqual({ volume: 0.4, muted: false });
});

test('a stored entry reads back as the level and the mute it was written with', () => {
  expect(parseVolume('{"volume":0.4,"muted":false}')).toStrictEqual({ volume: 0.4, muted: false });
  expect(parseVolume('{"volume":0,"muted":true}')).toStrictEqual({ volume: 0, muted: true });
  expect(parseVolume('{"volume":0.65}')).toStrictEqual({ volume: 0.65, muted: false });
  // Out-of-range or broken levels come back clamped, not defaulted.
  expect(parseVolume('{"volume":2,"muted":true}')).toStrictEqual({ volume: 1, muted: true });
  expect(parseVolume('{"volume":-3,"muted":true}')).toStrictEqual({ volume: 0, muted: true });
  expect(parseVolume('{"volume":null,"muted":false}')).toStrictEqual({ volume: 0.8, muted: false });
});

test('the entry round-trips through one string', () => {
  const memory: VolumeMemory = { volume: 0.35, muted: true };
  expect(serializeVolume(memory)).toStrictEqual('{"volume":0.35,"muted":true}');
  expect(parseVolume(serializeVolume(memory))).toStrictEqual(memory);
  expect(parseVolume(serializeVolume(defaultVolumeMemory))).toStrictEqual(defaultVolumeMemory);
});
