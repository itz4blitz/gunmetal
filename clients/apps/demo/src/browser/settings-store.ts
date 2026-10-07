import type { SettingsStore } from '../../../../packages/ui/src/shell/theme.ts';

const KEY = 'gunmetal.settings.v1';

// The one place the demo touches localStorage for preferences. It holds the
// theme choice and nothing else: a preference is not Activity, Identity or
// Secret data (SEC-PRV-019), and no token or history ever goes here
// (SEC-IAM-017). Storage can be blocked or full (private windows, site-data
// settings); then the choice is simply not remembered and the shell follows
// the system again.
export function createSettingsStore(storage: Storage): SettingsStore {
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
        // Nothing to do: the choice still applies for this visit.
      }
    },
  };
}
