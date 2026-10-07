import { expect, test } from 'vitest';
import { matchAddress, parseHistoryState } from './match.ts';

// Verifies: SEC-CLI-025
test('an exact closed path matches its route and empty history', () => {
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/', surface: 'SUR-020', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
  expect(matchAddress({ pathname: '/search', search: '', hash: '', state: undefined })).toStrictEqual({
    kind: 'ok',
    route: { path: '/search', surface: 'SUR-032', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
  expect(matchAddress({ pathname: '/library', search: '', hash: '', state: {} })).toStrictEqual({
    kind: 'ok',
    route: { path: '/library', surface: 'SUR-022', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
  expect(matchAddress({ pathname: '/settings', search: '', hash: '', state: { scrollY: 12 } })).toStrictEqual({
    kind: 'ok',
    route: { path: '/settings', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    history: { scrollY: 12, itemId: undefined },
  });
});

// Verifies: SEC-CLI-025
test('an unknown path, a fragment and a query string each refuse the address', () => {
  expect(matchAddress({ pathname: '/album', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/library/', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '#x', state: null })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/search', search: '?q=1', hash: '', state: null })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/library', search: '?a=1', hash: '#b', state: null })).toStrictEqual({
    kind: 'not-found',
  });
});

// Verifies: SEC-CLI-025
test('opaque item ids stay in history state and malformed ids refuse the address', () => {
  expect(
    matchAddress({ pathname: '/library', search: '', hash: '', state: { itemId: 'demo-album-01' } }),
  ).toStrictEqual({
    kind: 'ok',
    route: { path: '/library', surface: 'SUR-022', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: 'demo-album-01' },
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { itemId: '' } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { itemId: 'a/b' } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { itemId: 'a?b' } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { itemId: 'a#b' } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { itemId: 'a\\b' } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { itemId: 'a b' } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { itemId: 'x'.repeat(129) } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { itemId: 1 } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: 'bad' })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { extra: true } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { scrollY: -1 } })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/', search: '', hash: '', state: { scrollY: Number.NaN } })).toStrictEqual({
    kind: 'not-found',
  });
});

test('parseHistoryState accepts a finite non-negative scroll offset', () => {
  expect(parseHistoryState({ scrollY: 0 })).toStrictEqual({
    ok: true,
    value: { scrollY: 0, itemId: undefined },
  });
  expect(parseHistoryState(null)).toStrictEqual({
    ok: true,
    value: { scrollY: 0, itemId: undefined },
  });
});
