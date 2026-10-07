import { afterEach, expect, test } from 'vitest';
import { createVolumeStore } from './volume-store.ts';

afterEach(() => {
  window.localStorage.clear();
});

test('the volume is kept under one versioned key and read back as written', () => {
  const store = createVolumeStore(window.localStorage);
  expect(store.read()).toStrictEqual(null);
  store.write('{"volume":0.4,"muted":true}');
  expect(window.localStorage.getItem('gunmetal.volume.v1')).toStrictEqual('{"volume":0.4,"muted":true}');
  expect(window.localStorage.length).toStrictEqual(1);
  expect(store.read()).toStrictEqual('{"volume":0.4,"muted":true}');
});

test('blocked storage reads as nothing stored and drops the write', () => {
  const calls: string[] = [];
  const blocked: Storage = {
    length: 0,
    clear: () => undefined,
    key: () => null,
    removeItem: () => undefined,
    getItem: (key) => {
      calls.push(`get ${key}`);
      throw new DOMException('denied', 'SecurityError');
    },
    setItem: (key, value) => {
      calls.push(`set ${key} ${value}`);
      throw new DOMException('full', 'QuotaExceededError');
    },
  };
  const store = createVolumeStore(blocked);
  expect(store.read()).toStrictEqual(null);
  expect(store.write('{"volume":0.5,"muted":false}')).toStrictEqual(undefined);
  expect(calls).toStrictEqual(['get gunmetal.volume.v1', 'set gunmetal.volume.v1 {"volume":0.5,"muted":false}']);
});
