import { expect, test } from 'vitest';
import { historyPayload, pushPath, replacePath } from './navigate.ts';

test('navigation writes only scroll offset into history state', () => {
  expect(historyPayload(0)).toStrictEqual({ scrollY: 0, itemId: undefined });
  expect(historyPayload(40)).toStrictEqual({ scrollY: 40, itemId: undefined });
});

test('push and replace call the history writer with the closed path', () => {
  const calls: { kind: string; state: unknown; url: string }[] = [];
  const history = {
    pushState: (state: unknown, _unused: string, url: string) => {
      calls.push({ kind: 'push', state, url });
    },
    replaceState: (state: unknown, _unused: string, url: string) => {
      calls.push({ kind: 'replace', state, url });
    },
  };
  expect(pushPath(history, '/library', 8)).toStrictEqual({ scrollY: 8, itemId: undefined });
  expect(replacePath(history, '/search')).toStrictEqual({ scrollY: 0, itemId: undefined });
  expect(calls).toStrictEqual([
    { kind: 'push', state: { scrollY: 8, itemId: undefined }, url: '/library' },
    { kind: 'replace', state: { scrollY: 0, itemId: undefined }, url: '/search' },
  ]);
});
