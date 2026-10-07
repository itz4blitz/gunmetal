import { expect, test } from 'vitest';
import {
  demoPluginConsents,
  demoPluginManifests,
  loadedPluginSlots,
  pluginSlots,
  type PluginSlot,
} from './plugin-slots.ts';

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
  expect(slots.map((slot) => slot.plane)).toStrictEqual(['server', 'server', 'server', 'server', 'client', 'client']);
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
  const metadata = slots[0] as PluginSlot;
  const themePack = slots[4] as PluginSlot;
  expect(metadata).toStrictEqual({ id: 'metadata-provider', plane: 'server', featureId: 'INT-077', loaded: false });
  expect(loadedPluginSlots([{ ...metadata, loaded: true }, ...slots.slice(1)]).map((slot) => slot.id)).toStrictEqual([
    'metadata-provider',
  ]);
  expect(loadedPluginSlots([{ ...themePack, loaded: true }]).map((slot) => slot.plane)).toStrictEqual(['client']);
});

test('the demo advertises one manifest per client-plane slot, as data only', () => {
  expect(demoPluginManifests()).toStrictEqual({
    'home-row': {
      id: 'from-the-vault',
      title: 'From the vault',
      version: '1.0.0',
      plane: 'client',
      slot: 'home-row',
      grants: ['home-row:read'],
    },
    'theme-pack': {
      id: 'demo-theme-pack',
      title: 'Demo theme pack',
      version: '1.0.0',
      plane: 'client',
      slot: 'theme-pack',
      grants: ['theme-pack:apply'],
    },
  });
});

test('the demo consent ledger ships empty: R1 grants nothing, so every client plugin stays dark', () => {
  expect(demoPluginConsents()).toStrictEqual([]);
});

test('the two client-plane slots carry the demo manifests; the server-plane slots carry none', () => {
  const slots = pluginSlots();
  const manifests = demoPluginManifests();
  expect(slots[4]).toStrictEqual({
    id: 'theme-pack',
    plane: 'client',
    featureId: 'INT-081',
    loaded: false,
    manifest: manifests['theme-pack'],
  });
  expect(slots[5]).toStrictEqual({
    id: 'home-row',
    plane: 'client',
    featureId: 'INT-081',
    loaded: false,
    manifest: manifests['home-row'],
  });
  expect(slots.slice(0, 4).map((slot) => 'manifest' in slot)).toStrictEqual([false, false, false, false]);
});
