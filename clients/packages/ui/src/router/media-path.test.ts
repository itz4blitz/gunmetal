import { expect, test } from 'vitest';
import { labelFromKey, mediaPath, parseMediaPath, slugify, uniqueSlug } from './media-path.ts';

test('music and watch addresses are one parameter, and everything else is not a route', () => {
  expect(parseMediaPath('/music/albums/harbour-lights')).toStrictEqual({
    kind: 'album',
    key: 'harbour-lights',
    path: '/music/albums/harbour-lights',
  });
  expect(parseMediaPath('/music/artists/mira-sol')).toStrictEqual({
    kind: 'artist',
    key: 'mira-sol',
    path: '/music/artists/mira-sol',
  });
  expect(parseMediaPath('/music/tracks/pier-at-dusk')).toStrictEqual({
    kind: 'track',
    key: 'pier-at-dusk',
    path: '/music/tracks/pier-at-dusk',
  });
  expect(parseMediaPath('/watch/movies/inception')).toStrictEqual({
    kind: 'movie',
    key: 'inception',
    path: '/watch/movies/inception',
  });
  expect(parseMediaPath('/watch/shows/the-wire')).toStrictEqual({
    kind: 'show',
    key: 'the-wire',
    path: '/watch/shows/the-wire',
  });
  expect(parseMediaPath('/music/albums/demo-album-01')).toStrictEqual({
    kind: 'album',
    key: 'demo-album-01',
    path: '/music/albums/demo-album-01',
  });
  expect(mediaPath('album', 'harbour-lights')).toStrictEqual('/music/albums/harbour-lights');
  expect(parseMediaPath('/music/albums')).toStrictEqual(undefined);
  expect(parseMediaPath('/music/albums/')).toStrictEqual(undefined);
  expect(parseMediaPath('/music/albums/harbour-lights/tracks')).toStrictEqual(undefined);
  expect(parseMediaPath('/music/albums/Harbour-Lights')).toStrictEqual(undefined);
  expect(parseMediaPath('/music/albums/harbour--lights')).toStrictEqual(undefined);
  expect(parseMediaPath('/music/albums/../secret')).toStrictEqual(undefined);
  expect(parseMediaPath('/library')).toStrictEqual(undefined);
  expect(parseMediaPath('/watch/movies/a b')).toStrictEqual(undefined);
});

test('a name becomes a slug, and a collision keeps both addresses', () => {
  expect(slugify('Harbour Lights')).toStrictEqual('harbour-lights');
  expect(slugify('Mira Sol')).toStrictEqual('mira-sol');
  expect(slugify('Café Lights')).toStrictEqual('cafe-lights');
  expect(slugify('!!!')).toStrictEqual('item');
  expect(slugify('a'.repeat(80))).toStrictEqual('a'.repeat(48));
  expect(mediaPath('album', 'harbour-lights')).toStrictEqual('/music/albums/harbour-lights');
  expect(mediaPath('album', 'Bad')).toStrictEqual(undefined);
  expect(labelFromKey('harbour-lights')).toStrictEqual('Harbour Lights');
  expect(labelFromKey('a')).toStrictEqual('A');
  const used = new Map<string, string>([['intro', 'track-a']]);
  expect(uniqueSlug('Intro', 'track-b', used)).toStrictEqual('intro-trackb');
  expect(uniqueSlug('Intro', 'track-a', used)).toStrictEqual('intro');
  expect(uniqueSlug('Harbour', 'album-1', new Map())).toStrictEqual('harbour');
  expect(uniqueSlug('Intro', '---', used)).toStrictEqual('intro');
});
