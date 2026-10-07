import type { LayoutStore } from '../../../../packages/ui/src/shell/pane-widths.ts';

const KEY = 'gunmetal.layout.v1';

// The one place the demo touches localStorage. It holds the pane widths and
// nothing else: layout is not Activity, Identity or Secret data (SEC-PRV-019),
// and no token or history ever goes here (SEC-IAM-017). Storage can be blocked
// or full (private windows, site-data settings); then the layout is simply
// not remembered.
export function createLayoutStore(storage: Storage): LayoutStore {
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
        // Nothing to do: the width still applies for this visit.
      }
    },
  };
}
