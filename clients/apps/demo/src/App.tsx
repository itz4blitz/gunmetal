import { useCallback, useEffect, useMemo, useState } from 'react';
import { Shell } from '../../../packages/ui/src/shell/Shell.tsx';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';
import { ActivityBar } from './ActivityBar.tsx';
import { listOutputDevices, type OutputDevice } from './browser/audio-element.ts';
import { createLayoutStore } from './browser/layout-store.ts';
import { createSettingsStore } from './browser/settings-store.ts';
import { createSessionPlaybackStore } from './browser/session-playback-store.ts';
import { createVolumeStore } from './browser/volume-store.ts';
import { useDemoPlayback } from './controller.ts';
import { composeDemo } from './compose.ts';
import {
  defaultPlaybackPrefs,
  readPlaybackPrefs,
  serializePlaybackPrefs,
  type CrossfadeSeconds,
  type Levelling,
  type PlaybackPrefs,
} from './playback-prefs.ts';
import { loadServedLibrary } from './served-library.ts';

const PLAYBACK_KEY = 'gunmetal.playback';

function readPlaybackStorage(): string | null {
  try {
    return globalThis.localStorage.getItem(PLAYBACK_KEY);
  } catch {
    return null;
  }
}

function writePlaybackStorage(value: string): void {
  try {
    globalThis.localStorage.setItem(PLAYBACK_KEY, value);
  } catch {
    // The choice still applies for this visit.
  }
}

function loadPlaybackPrefs(): PlaybackPrefs {
  const raw = readPlaybackStorage();
  if (raw === null) {
    return defaultPlaybackPrefs();
  }
  return readPlaybackPrefs(raw);
}

export function App({ library }: { library?: DemoLibrary | undefined }) {
  const [live, setLive] = useState(library);
  const refresh = useCallback((done: number) => {
    if (done < 1) {
      return;
    }
    void loadServedLibrary().then((next) => {
      if (next !== undefined) {
        setLive(next);
      }
    });
  }, []);
  const demo = composeDemo(live);
  const [prefs, setPrefs] = useState<PlaybackPrefs>(() => loadPlaybackPrefs());
  const [outputs, setOutputs] = useState<readonly OutputDevice[]>([]);
  useEffect(() => {
    let liveList = true;
    const load = () => {
      void listOutputDevices().then((devices) => {
        if (liveList) {
          setOutputs(devices);
        }
      });
    };
    load();
    /* Plugging in headphones re-lists the outputs; the browser does not reload
       the page for it. The listener leaves with the component. */
    const mediaDevices = navigator.mediaDevices as MediaDevices | undefined;
    mediaDevices?.addEventListener('devicechange', load);
    return () => {
      liveList = false;
      mediaDevices?.removeEventListener('devicechange', load);
    };
  }, []);
  const onLevelling = useCallback((levelling: Levelling) => {
    setPrefs((current) => {
      const next = { ...current, levelling };
      writePlaybackStorage(serializePlaybackPrefs(next));
      return next;
    });
  }, []);
  const onCrossfade = useCallback((crossfadeSeconds: CrossfadeSeconds) => {
    setPrefs((current) => {
      const next = { ...current, crossfadeSeconds };
      writePlaybackStorage(serializePlaybackPrefs(next));
      return next;
    });
  }, []);
  const onOutput = useCallback((sinkId: string) => {
    setPrefs((current) => {
      const next = { ...current, sinkId };
      writePlaybackStorage(serializePlaybackPrefs(next));
      return next;
    });
  }, []);
  const volumeStore = useMemo(() => createVolumeStore(globalThis.localStorage), []);
  const sessionStore = useMemo(() => createSessionPlaybackStore(globalThis.sessionStorage), []);
  const playback = useDemoPlayback(demo.library, {
    volumeStore,
    sessionStore,
    prefs,
  });
  return (
    <>
      <ActivityBar onAdvance={refresh} />
      <Shell
        showDemoLabel={demo.showDemoLabel}
        library={demo.library}
        searchLibrary={demo.searchLibrary}
        lyricsFor={demo.lyricsFor}
        timedLyricsFor={demo.timedLyricsFor}
        pluginSlots={demo.pluginSlots}
        settingsStore={createSettingsStore(globalThis.localStorage)}
        playback={playback}
        layoutStore={createLayoutStore(globalThis.localStorage)}
        levelling={prefs.levelling}
        onLevelling={onLevelling}
        crossfadeSeconds={prefs.crossfadeSeconds}
        onCrossfade={onCrossfade}
        outputs={outputs}
        sinkId={prefs.sinkId}
        onOutput={onOutput}
      />
    </>
  );
}
