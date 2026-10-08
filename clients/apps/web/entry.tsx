import { createRoot } from 'react-dom/client';
import { readOrigin, readRoot, readSecureContext, readTrustedTypes, readWebAssembly } from './browser/environment.ts';
import { goUnsupported } from './browser/navigate.ts';
import { App } from './src/App.tsx';
import { openPlayer } from './src/open-player.ts';
import { start } from './src/start.ts';

start({
  wasm: readWebAssembly() !== undefined,
  secure: readSecureContext(),
  origin: readOrigin(),
  root: readRoot(),
  trustedTypes: readTrustedTypes(),
  navigateUnsupported: goUnsupported,
  render: (root) => {
    void openPlayer(root, (element, library) => {
      createRoot(element).render(<App library={library} />);
    });
  },
});
