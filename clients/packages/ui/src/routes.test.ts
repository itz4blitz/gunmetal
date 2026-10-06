import { expect, test } from 'vitest';
import { routes } from './routes.ts';

test('the closed route list is the four C0 destinations with their surfaces', () => {
  expect(routes()).toStrictEqual([
    { path: '/', surface: 'SUR-020', needsSession: true, needsAdminSession: false },
    { path: '/search', surface: 'SUR-032', needsSession: true, needsAdminSession: false },
    { path: '/library', surface: 'SUR-022', needsSession: true, needsAdminSession: false },
    { path: '/settings', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
  ]);
});
