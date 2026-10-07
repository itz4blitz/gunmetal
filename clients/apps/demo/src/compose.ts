import { demoLibrary, demoLocalFilter, type DemoLibrary } from '../../../packages/fake-server/src/index.ts';

export type DemoCompose = {
  showDemoLabel: boolean;
  library: DemoLibrary;
  searchLibrary: typeof demoLocalFilter;
};

export function composeDemo(): DemoCompose {
  return {
    showDemoLabel: true,
    library: demoLibrary(),
    searchLibrary: demoLocalFilter,
  };
}
