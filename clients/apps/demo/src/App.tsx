import { Shell } from '../../../packages/ui/src/shell/Shell.tsx';
import { composeDemo } from './compose.ts';

export function App() {
  const demo = composeDemo();
  return <Shell showDemoLabel={demo.showDemoLabel} library={demo.library} />;
}
