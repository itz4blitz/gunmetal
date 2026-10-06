export type Capability = { wasm: boolean; secure: boolean };

// The start-up check of SEC-API-052: WebAssembly and a secure context. Loopback HTTP is a secure
// context in the browser; a cleartext non-loopback origin is not.

export function supported(capability: Capability): boolean {
  return capability.wasm && capability.secure;
}
