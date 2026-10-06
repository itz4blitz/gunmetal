import { expect, test } from 'vitest';
import { loadedPluginSlots, pluginSlots } from './plugin-slots.ts';

test('first-party extension points exist on both planes and none are loaded', () => {
  const slots = pluginSlots();
  expect(slots.map((slot) => slot.id)).toStrictEqual([
    'metadata-provider',
    'lyrics-provider',
    'search-provider',
    'scrobbler',
    'theme-pack',
    'home-row',
  ]);
  expect(slots.map((slot) => slot.plane)).toStrictEqual([
    'server',
    'server',
    'server',
    'server',
    'client',
    'client',
  ]);
  expect(slots.map((slot) => slot.featureId)).toStrictEqual([
    'INT-077',
    'INT-078',
    'INT-083',
    'INT-075',
    'INT-081',
    'INT-081',
  ]);
  expect(slots.every((slot) => slot.loaded === false)).toStrictEqual(true);
  expect(loadedPluginSlots(slots)).toStrictEqual([]);
});

test('a slot marked loaded is the only one the host index would return', () => {
  const slots = pluginSlots();
  const armed = { ...slots[0]!, loaded: true };
  expect(loadedPluginSlots([armed, ...slots.slice(1)]).map((slot) => slot.id)).toStrictEqual([
    'metadata-provider',
  ]);
  expect(loadedPluginSlots([{ ...slots[4]!, loaded: true }]).map((slot) => slot.plane)).toStrictEqual([
    'client',
  ]);
});
