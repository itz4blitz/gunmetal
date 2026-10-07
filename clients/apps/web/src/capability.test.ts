import { expect, test } from 'vitest';
import { supported } from './capability.ts';

// Verifies: SEC-API-052
test('the app runs only with WebAssembly and a secure context', () => {
  expect(supported({ wasm: true, secure: true })).toStrictEqual(true);
  expect(supported({ wasm: true, secure: false })).toStrictEqual(false);
  expect(supported({ wasm: false, secure: true })).toStrictEqual(false);
  expect(supported({ wasm: false, secure: false })).toStrictEqual(false);
});
