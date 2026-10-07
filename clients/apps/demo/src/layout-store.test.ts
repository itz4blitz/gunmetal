import { afterEach, expect, test } from 'vitest';
import { createLayoutStore } from './browser/layout-store.ts';

afterEach(() => {
  window.localStorage.clear();
});

test('the layout is kept under one versioned key and read back as written', () => {
  const store = createLayoutStore(window.localStorage);
  expect(store.read()).toStrictEqual(null);
  store.write('{"sidebar":312,"queue":400}');
  expect(window.localStorage.getItem('gunmetal.layout.v1')).toStrictEqual('{"sidebar":312,"queue":400}');
  expect(window.localStorage.length).toStrictEqual(1);
  expect(store.read()).toStrictEqual('{"sidebar":312,"queue":400}');
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
  const store = createLayoutStore(blocked);
  expect(store.read()).toStrictEqual(null);
  expect(store.write('{"sidebar":300,"queue":340}')).toStrictEqual(undefined);
  expect(calls).toStrictEqual(['get gunmetal.layout.v1', 'set gunmetal.layout.v1 {"sidebar":300,"queue":340}']);
});
