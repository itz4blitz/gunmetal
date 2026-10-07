import type { ClientPluginManifest, PluginConsent } from '../../ports/src/plugins/types.ts';

export type PluginPlane = 'server' | 'client';

export type PluginSlotId =
  'metadata-provider' | 'lyrics-provider' | 'search-provider' | 'scrobbler' | 'theme-pack' | 'home-row';

export type PluginSlot = {
  id: PluginSlotId;
  plane: PluginPlane;
  featureId: 'INT-075' | 'INT-077' | 'INT-078' | 'INT-081' | 'INT-083';
  loaded: boolean;
  /** What a server would advertise for a client-plane slot; server-plane rows carry none. */
  manifest?: ClientPluginManifest;
};

/** The client-plane manifests the demo advertises, keyed by slot: data only, nothing loads (ADR 22). */
export function demoPluginManifests(): Readonly<{
  'home-row': ClientPluginManifest;
  'theme-pack': ClientPluginManifest;
}> {
  return {
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
  };
}

/** The demo consent ledger: empty, because R1 grants nothing and loads nothing (ADR 22). */
export function demoPluginConsents(): readonly PluginConsent[] {
  return [];
}

export function pluginSlots(): readonly PluginSlot[] {
  const manifests = demoPluginManifests();
  return [
    { id: 'metadata-provider', plane: 'server', featureId: 'INT-077', loaded: false },
    { id: 'lyrics-provider', plane: 'server', featureId: 'INT-078', loaded: false },
    { id: 'search-provider', plane: 'server', featureId: 'INT-083', loaded: false },
    { id: 'scrobbler', plane: 'server', featureId: 'INT-075', loaded: false },
    { id: 'theme-pack', plane: 'client', featureId: 'INT-081', loaded: false, manifest: manifests['theme-pack'] },
    { id: 'home-row', plane: 'client', featureId: 'INT-081', loaded: false, manifest: manifests['home-row'] },
  ];
}

export function loadedPluginSlots(slots: readonly PluginSlot[]): readonly PluginSlot[] {
  return slots.filter((slot) => slot.loaded);
}
