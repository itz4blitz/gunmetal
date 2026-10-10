import type { SessionPlaybackStore } from '../session-playback.ts';

const KEY = 'gunmetal.playback.session';

/**
 * The one place the player touches sessionStorage. The queue and the place
 * in the song are Activity, so they stay out of localStorage (SEC-PRV-019).
 * The tab drops this when it closes. A blocked store remembers nothing.
 */
export function createSessionPlaybackStore(storage: Storage): SessionPlaybackStore {
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
        // The song still plays for this visit. A refresh will not find it.
      }
    },
  };
}
