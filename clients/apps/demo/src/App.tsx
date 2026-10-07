import { Shell } from '../../../packages/ui/src/shell/Shell.tsx';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';
import { createLayoutStore } from './browser/layout-store.ts';
import { createSettingsStore } from './browser/settings-store.ts';
import { createVolumeStore } from './browser/volume-store.ts';
import { useDemoPlayback } from './controller.ts';
import { composeDemo } from './compose.ts';

export function App({ library }: { library?: DemoLibrary | undefined }) {
  const demo = composeDemo(library);
  const playback = useDemoPlayback(demo.library, { volumeStore: createVolumeStore(globalThis.localStorage) });
  return (
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
    />
  );
}
