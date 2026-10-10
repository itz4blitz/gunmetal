import { useCallback, useEffect, useMemo, useState } from 'react';
import { Shell } from '../../../packages/ui/src/shell/Shell.tsx';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';
import { ActivityBar } from './ActivityBar.tsx';
import { SourcesPane } from './SourcesPane.tsx';
import { WatchArea } from './WatchArea.tsx';
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
  const [mode, setMode] = useState<'music' | 'video'>('music');
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
    void listOutputDevices().then((devices) => {
      if (liveList) {
        setOutputs(devices);
      }
    });
    return () => {
      liveList = false;
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
      <nav id="demo-mode-switch" aria-label="Area">
        <button
          type="button"
          aria-pressed={mode === 'music'}
          onClick={() => setMode('music')}
        >
          Music
        </button>
        <button
          type="button"
          aria-pressed={mode === 'video'}
          onClick={() => setMode('video')}
        >
          Watch
        </button>
      </nav>
      {mode === 'music' ? (
        <Shell
        showDemoLabel={demo.showDemoLabel}
        library={demo.library}
        searchLibrary={demo.searchLibrary}
        lyricsFor={demo.lyricsFor}
        timedLyricsFor={demo.timedLyricsFor}
        pluginSlots={demo.pluginSlots}
        settingsStore={createSettingsStore(globalThis.localStorage)}
        settingsLibraries={<SourcesPane />}
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
      ) : (
        <WatchArea />
      )}
    </>
  );
}
