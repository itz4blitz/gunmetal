import { expect, test } from 'vitest';
import { addressOnLoad, browserWins, canonicalPath, matchAddress, parseHistoryState } from './match.ts';

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
    route: { path: '/settings', surface: 'SUR-076', needsSession: true, needsAdminSession: false },
    history: { scrollY: 12, itemId: undefined },
  });
  expect(matchAddress({ pathname: '/settings/appearance', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/settings/appearance', surface: 'SUR-076', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
  expect(matchAddress({ pathname: '/settings/playback', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/settings/playback', surface: 'SUR-074', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
  expect(matchAddress({ pathname: '/settings/extensions', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/settings/extensions', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
  expect(matchAddress({ pathname: '/settings/about', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/settings/about', surface: 'SUR-077', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
  expect(matchAddress({ pathname: '/settings/privacy', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/settings/privacy', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
});

// Verifies: SEC-CLI-025
test('an unknown path, a fragment and a query string each refuse the address', () => {
  expect(matchAddress({ pathname: '/album', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/settings/nope', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'not-found',
  });
  expect(matchAddress({ pathname: '/settings/appearance/', search: '', hash: '', state: null })).toStrictEqual({
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

test('the browser wins when no parent path is passed, including at /', () => {
  expect(browserWins('/', undefined)).toStrictEqual(true);
  expect(browserWins('/library', undefined)).toStrictEqual(true);
  expect(browserWins('/library', '/')).toStrictEqual(true);
  expect(browserWins('/', '/library')).toStrictEqual(false);
  expect(browserWins('/search', '/search')).toStrictEqual(false);
  expect(browserWins('/nope', '/library')).toStrictEqual(false);
  expect(browserWins('/music/albums/harbour-lights', '/')).toStrictEqual(true);
  expect(browserWins('/watch/shows/the-wire', '/library')).toStrictEqual(true);
});

test('a media address is a typed route, and a query or a bad key is not', () => {
  expect(matchAddress({ pathname: '/music/artists/mira-sol', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/music/artists/mira-sol', surface: 'SUR-024', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
    media: { kind: 'artist', key: 'mira-sol', path: '/music/artists/mira-sol' },
  });
  expect(matchAddress({ pathname: '/music/albums/harbour-lights', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/music/albums/harbour-lights', surface: 'SUR-025', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
    media: { kind: 'album', key: 'harbour-lights', path: '/music/albums/harbour-lights' },
  });
  expect(matchAddress({ pathname: '/music/tracks/pier-at-dusk', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/music/tracks/pier-at-dusk', surface: 'SUR-016', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
    media: { kind: 'track', key: 'pier-at-dusk', path: '/music/tracks/pier-at-dusk' },
  });
  expect(matchAddress({ pathname: '/watch/movies/inception', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/watch/movies/inception', surface: 'SUR-040', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
    media: { kind: 'movie', key: 'inception', path: '/watch/movies/inception' },
  });
  expect(matchAddress({ pathname: '/watch/shows/the-wire', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/watch/shows/the-wire', surface: 'SUR-041', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
    media: { kind: 'show', key: 'the-wire', path: '/watch/shows/the-wire' },
  });
  expect(matchAddress({ pathname: '/music/albums/harbour-lights', search: '?x=1', hash: '', state: null })).toStrictEqual({
    kind: 'not-found',
  });
  expect(
    matchAddress({ pathname: '/music/albums/harbour-lights', search: '', hash: '', state: { itemId: 'a/b' } }),
  ).toStrictEqual({ kind: 'not-found' });
});

test('settings resolves to appearance and every other path stays itself', () => {
  expect(canonicalPath('/settings')).toStrictEqual('/settings/appearance');
  expect(canonicalPath('/')).toStrictEqual('/');
  expect(canonicalPath('/library')).toStrictEqual('/library');
  expect(canonicalPath('/settings/playback')).toStrictEqual('/settings/playback');
  expect(canonicalPath('/nope')).toStrictEqual('/nope');
});

test('the browser address wins when a parent path would hide a closed route', () => {
  expect(
    addressOnLoad(
      { pathname: '/library', search: '', hash: '', state: null },
      { path: '/', search: '', hash: '', state: null },
    ),
  ).toStrictEqual({ pathname: '/library', search: '', hash: '', state: null });
  expect(
    addressOnLoad(
      { pathname: '/search', search: '?q=1', hash: '', state: { scrollY: 3 } },
      { path: '/library', search: '', hash: '', state: null },
    ),
  ).toStrictEqual({ pathname: '/search', search: '?q=1', hash: '', state: { scrollY: 3 } });
  expect(
    addressOnLoad(
      { pathname: '/settings', search: '', hash: '', state: null },
      { path: undefined, search: '', hash: '', state: null },
    ),
  ).toStrictEqual({ pathname: '/settings/appearance', search: '', hash: '', state: null });
});

test('a parent path applies at the default address, on agreement, and on an unknown address', () => {
  expect(
    addressOnLoad(
      { pathname: '/', search: '', hash: '', state: null },
      { path: '/library', search: '', hash: '', state: { scrollY: 4 } },
    ),
  ).toStrictEqual({ pathname: '/library', search: '', hash: '', state: { scrollY: 4 } });
  expect(
    addressOnLoad(
      { pathname: '/search', search: '', hash: '', state: null },
      { path: '/search', search: '?q=1', hash: '#x', state: { itemId: 'demo-album-01' } },
    ),
  ).toStrictEqual({
    pathname: '/search',
    search: '?q=1',
    hash: '#x',
    state: { itemId: 'demo-album-01' },
  });
  expect(
    addressOnLoad(
      { pathname: '/nope', search: '', hash: '', state: null },
      { path: '/library', search: '', hash: '', state: null },
    ),
  ).toStrictEqual({ pathname: '/library', search: '', hash: '', state: null });
  expect(
    addressOnLoad(
      { pathname: '/', search: '', hash: '', state: { scrollY: 2 } },
      { path: '/settings', search: '', hash: '', state: null },
    ),
  ).toStrictEqual({ pathname: '/settings/appearance', search: '', hash: '', state: null });
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
