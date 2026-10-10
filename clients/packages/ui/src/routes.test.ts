import { expect, test } from 'vitest';
import { routes } from './routes.ts';

test('the closed route list is home, search, library, the store and the settings sections', () => {
  expect(routes()).toStrictEqual([
    { path: '/', surface: 'SUR-020', needsSession: true, needsAdminSession: false },
    { path: '/search', surface: 'SUR-032', needsSession: true, needsAdminSession: false },
    { path: '/library', surface: 'SUR-022', needsSession: true, needsAdminSession: false },
    { path: '/store', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/store/cover-art', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/store/lyrics', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/store/catalogue-search', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/store/scrobble', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/store/themes', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/store/home-rows', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/store/url-style', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/settings', surface: 'SUR-076', needsSession: true, needsAdminSession: false },
    { path: '/settings/appearance', surface: 'SUR-076', needsSession: true, needsAdminSession: false },
    { path: '/settings/playback', surface: 'SUR-074', needsSession: true, needsAdminSession: false },
    { path: '/settings/connected', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/settings/about', surface: 'SUR-077', needsSession: true, needsAdminSession: false },
    { path: '/settings/privacy', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
  ]);
});
