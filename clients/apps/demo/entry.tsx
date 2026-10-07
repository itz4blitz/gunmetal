import { createRoot } from 'react-dom/client';
import { App } from './src/App.tsx';
import { loadServedLibrary } from './src/served-library.ts';

const root = document.getElementById('root');
if (root !== null) {
  const served = await loadServedLibrary();
  createRoot(root).render(<App library={served} />);
}
