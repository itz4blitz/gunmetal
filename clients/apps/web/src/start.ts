import { supported } from './capability.ts';
import { installGunmetalLoader, scriptUrl, type TrustedTypes } from './policy.ts';

export type Host = {
  wasm: boolean;
  secure: boolean;
  origin: string;
  root: Element | null;
  trustedTypes: TrustedTypes | undefined;
  navigateUnsupported: () => void;
  render: (root: Element) => void;
};

export type BootResult = 'app' | 'unsupported' | 'missing-root';

export function start(host: Host): BootResult {
  installGunmetalLoader(host.trustedTypes, (url) => scriptUrl(url, host.origin));
  if (!supported({ wasm: host.wasm, secure: host.secure })) {
    host.navigateUnsupported();
    return 'unsupported';
  }
  if (host.root === null) {
    return 'missing-root';
  }
  host.render(host.root);
  return 'app';
}
