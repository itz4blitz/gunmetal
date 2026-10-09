import { App as Player } from '../../demo/src/App.tsx';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';

/** The product composition root: the player the demo proved, on this origin. */
export function App({ library }: { library?: DemoLibrary | undefined }) {
  return <Player library={library} />;
}
