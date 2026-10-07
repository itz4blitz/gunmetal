import type { VolumeStore } from '../../../../packages/ui/src/shell/volume-store.ts';

const KEY = 'gunmetal.volume.v1';

// The one place the demo touches localStorage for sound. It holds the output
// level and the mute choice and nothing else: a listener preference, not
// Activity, Identity or Secret data (SEC-PRV-019). Storage can be blocked or
// full; then the level is simply not remembered.
export function createVolumeStore(storage: Storage): VolumeStore {
  return {
    read: () => {
      try {
        return storage.getItem(KEY);
      } catch {
        return null;
      }
    },
    write: (value) => {
      try {
        storage.setItem(KEY, value);
      } catch {
        // Nothing to do: the level still applies for this visit.
      }
    },
  };
}
