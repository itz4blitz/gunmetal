import type { TrustedTypes } from '../src/policy.ts';

export function readWebAssembly(): object | undefined {
  return globalThis.WebAssembly;
}

export function readSecureContext(): boolean {
  return globalThis.isSecureContext;
}

export function readTrustedTypes(): TrustedTypes | undefined {
  return Reflect.get(globalThis, 'trustedTypes') as TrustedTypes | undefined;
}

export function readOrigin(): string {
  return globalThis.location.origin;
}

export function readRoot(): HTMLElement | null {
  return document.getElementById('root');
}
