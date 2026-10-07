import { afterEach, expect, test } from 'vitest';
import { createSettingsStore } from './browser/settings-store.ts';

afterEach(() => {
  window.localStorage.clear();
});

test('the theme choice is kept under one versioned key and read back as written', () => {
  const store = createSettingsStore(window.localStorage);
  expect(store.read()).toStrictEqual(null);
  store.write('{"theme":"light"}');
  expect(window.localStorage.getItem('gunmetal.settings.v1')).toStrictEqual('{"theme":"light"}');
  expect(window.localStorage.length).toStrictEqual(1);
  expect(store.read()).toStrictEqual('{"theme":"light"}');
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
  const store = createSettingsStore(blocked);
  expect(store.read()).toStrictEqual(null);
  expect(store.write('{"theme":"dark"}')).toStrictEqual(undefined);
  expect(calls).toStrictEqual(['get gunmetal.settings.v1', 'set gunmetal.settings.v1 {"theme":"dark"}']);
});
