import { loadServedLibrary } from '../../demo/src/served-library.ts';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';

/**
 * Load the folder library from this origin, then mount the player.
 * A missing host or a refused document is `undefined`: the player keeps
 * its fixture catalogue. The URL is relative. Nothing here names a host.
 */
export async function openPlayer(
  root: Element,
  render: (root: Element, library: DemoLibrary | undefined) => void,
  fetchImpl: typeof fetch = globalThis.fetch,
): Promise<'opened'> {
  render(root, await loadServedLibrary(fetchImpl));
  return 'opened';
}
