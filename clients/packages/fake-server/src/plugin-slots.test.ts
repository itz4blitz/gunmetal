import { expect, test } from 'vitest';
import { hostServesCovers, pluginSlots } from './plugin-slots.ts';

test('the library host serves covers only at its own cover route', () => {
  expect(hostServesCovers([])).toStrictEqual(false);
  expect(hostServesCovers([{ coverUrl: '' }])).toStrictEqual(false);
  expect(hostServesCovers([{ coverUrl: '/media/covers/demo-album-01.svg' }])).toStrictEqual(false);
  expect(hostServesCovers([{ coverUrl: '/media/library/covers' }])).toStrictEqual(false);
  expect(hostServesCovers([{ coverUrl: 'https://coverartarchive.org/release/x/front' }])).toStrictEqual(false);
  expect(hostServesCovers([{ coverUrl: '/media/library/covers/aaaaaaaaaaaaaaaa.jpg' }])).toStrictEqual(true);
  expect(
    hostServesCovers([{ coverUrl: '' }, { coverUrl: '/media/library/covers/aaaaaaaaaaaaaaaa.png' }]),
  ).toStrictEqual(true);
});

test('jobs are first-party facts: Cover Art Archive is on only while covers are served', () => {
  expect(pluginSlots(true)).toStrictEqual([
    { id: 'metadata-provider', status: 'on' },
    { id: 'lyrics-provider', status: 'not-in-build' },
    { id: 'search-provider', status: 'not-in-build' },
    { id: 'scrobbler', status: 'not-in-build' },
    { id: 'theme-pack', status: 'not-a-plugin' },
    { id: 'home-row', status: 'not-a-plugin' },
  ]);
  expect(pluginSlots(false)).toStrictEqual([
    { id: 'metadata-provider', status: 'not-serving' },
    { id: 'lyrics-provider', status: 'not-in-build' },
    { id: 'search-provider', status: 'not-in-build' },
    { id: 'scrobbler', status: 'not-in-build' },
    { id: 'theme-pack', status: 'not-a-plugin' },
    { id: 'home-row', status: 'not-a-plugin' },
  ]);
});
