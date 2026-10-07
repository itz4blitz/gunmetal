import { demoLibrary, type DemoLibrary } from '../../../packages/fake-server/src/index.ts';

export type DemoCompose = {
  showDemoLabel: boolean;
  library: DemoLibrary;
};

export function composeDemo(): DemoCompose {
  return {
    showDemoLabel: true,
    library: demoLibrary(),
  };
}
