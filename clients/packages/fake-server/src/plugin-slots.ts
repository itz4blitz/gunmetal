export type PluginPlane = 'server' | 'client';

export type PluginSlotId =
  'metadata-provider' | 'lyrics-provider' | 'search-provider' | 'scrobbler' | 'theme-pack' | 'home-row';

export type PluginSlot = {
  id: PluginSlotId;
  plane: PluginPlane;
  featureId: 'INT-075' | 'INT-077' | 'INT-078' | 'INT-081' | 'INT-083';
  loaded: boolean;
};

export function pluginSlots(): readonly PluginSlot[] {
  return [
    { id: 'metadata-provider', plane: 'server', featureId: 'INT-077', loaded: false },
    { id: 'lyrics-provider', plane: 'server', featureId: 'INT-078', loaded: false },
    { id: 'search-provider', plane: 'server', featureId: 'INT-083', loaded: false },
    { id: 'scrobbler', plane: 'server', featureId: 'INT-075', loaded: false },
    { id: 'theme-pack', plane: 'client', featureId: 'INT-081', loaded: false },
    { id: 'home-row', plane: 'client', featureId: 'INT-081', loaded: false },
  ];
}

export function loadedPluginSlots(slots: readonly PluginSlot[]): readonly PluginSlot[] {
  return slots.filter((slot) => slot.loaded);
}
