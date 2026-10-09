import { expect, test } from 'vitest';
import { createSessionPlaybackStore } from './session-playback-store.ts';

test('the session store reads and writes only its own sessionStorage key', () => {
  window.sessionStorage.clear();
  window.localStorage.clear();
  const store = createSessionPlaybackStore(window.sessionStorage);
  expect(store.read()).toStrictEqual(null);
  store.write('{"queue":[]}');
  expect(store.read()).toStrictEqual('{"queue":[]}');
  expect(window.sessionStorage.getItem('gunmetal.playback.session')).toStrictEqual('{"queue":[]}');
  expect(window.localStorage.getItem('gunmetal.playback.session')).toStrictEqual(null);
});

test('a blocked session store reads as nothing and a failed write does not throw', () => {
  const blocked = {
    getItem(): string | null {
      throw new Error('blocked');
    },
    setItem(): void {
      throw new Error('full');
    },
  } as unknown as Storage;
  const store = createSessionPlaybackStore(blocked);
  expect(store.read()).toStrictEqual(null);
  expect(() => {
    store.write('{"queue":[]}');
  }).not.toThrow();
});
