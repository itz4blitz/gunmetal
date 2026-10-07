import { Shell } from '../../../packages/ui/src/shell/Shell.tsx';
import { useDemoPlayback } from './controller.ts';
import { composeDemo } from './compose.ts';

export function App() {
  const demo = composeDemo();
  const playback = useDemoPlayback(demo.library);
  return (
    <Shell
      showDemoLabel={demo.showDemoLabel}
      library={demo.library}
      searchLibrary={demo.searchLibrary}
      playback={playback}
    />
  );
}
