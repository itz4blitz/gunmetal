import { expect, test } from 'vitest';
import { demoLibrary } from '../../../fake-server/src/catalogue.ts';
import type { ShellAlbum, ShellArtist, ShellLibrary } from '../shell/library-types.ts';
import {
  albumAddress,
  artistAddress,
  itemAddress,
  resolveMedia,
  trackAddress,
} from './catalogue-address.ts';
import { parseMediaPath } from './media-path.ts';

function album(id: string, title: string, artistName = 'Mira Sol', artistKey = 'mira-sol'): ShellAlbum {
  return {
    id,
    title,
    artistName,
    artistKey,
    year: 2021,
    coverTone: '01',
    coverUrl: '/cover',
    discs: [],
    tracks: [],
    hostile: false,
  };
}

function artist(key: string, name: string): ShellArtist {
  return { key, name, albumIds: [] };
}

function library(albums: ShellAlbum[], artists: ShellArtist[]): ShellLibrary {
  return { albums, artists };
}

function media(pathname: string) {
  const parsed = parseMediaPath(pathname);
  if (parsed === undefined) {
    throw new Error(`${pathname} is not a media address`);
  }
  return parsed;
}

test('slug addresses are the canonical style, and an id opens the same page', () => {
  const catalogue = demoLibrary();
  expect(albumAddress(catalogue, 'demo-album-01')).toStrictEqual('/music/albums/harbour-lights');
  expect(albumAddress(catalogue, 'demo-album-01', 'slug')).toStrictEqual('/music/albums/harbour-lights');
  expect(albumAddress(catalogue, 'demo-album-01', 'id')).toStrictEqual('/music/albums/demo-album-01');
  expect(artistAddress(catalogue, 'mira-sol')).toStrictEqual('/music/artists/mira-sol');
  expect(artistAddress(catalogue, 'mira-sol', 'id')).toStrictEqual('/music/artists/mira-sol');
  expect(trackAddress(catalogue, 'demo-track-01-01')).toStrictEqual('/music/tracks/pier-at-dusk');
  expect(trackAddress(catalogue, 'demo-track-01-01', 'id')).toStrictEqual('/music/tracks/demo-track-01-01');
  expect(albumAddress(catalogue, 'missing')).toStrictEqual(undefined);
  expect(artistAddress(catalogue, 'missing')).toStrictEqual(undefined);
  expect(trackAddress(catalogue, 'missing')).toStrictEqual(undefined);

  const bySlug = resolveMedia(catalogue, media('/music/albums/harbour-lights'));
  const byId = resolveMedia(catalogue, media('/music/albums/demo-album-01'));
  expect(bySlug).toStrictEqual({
    kind: 'album',
    itemId: 'demo-album-01',
    label: 'Harbour Lights',
    path: '/music/albums/harbour-lights',
  });
  expect(byId).toStrictEqual(bySlug);
  expect(resolveMedia(catalogue, media('/music/artists/mira-sol'))?.itemId).toStrictEqual('mira-sol');
  expect(resolveMedia(catalogue, media('/music/tracks/pier-at-dusk'))).toStrictEqual({
    kind: 'track',
    itemId: 'demo-album-01',
    label: 'Pier at Dusk',
    path: '/music/tracks/pier-at-dusk',
  });
  expect(resolveMedia(catalogue, media('/music/tracks/demo-track-01-01'))?.path).toStrictEqual(
    '/music/tracks/pier-at-dusk',
  );
  expect(resolveMedia(catalogue, media('/music/albums/not-here'))).toStrictEqual(undefined);
  expect(resolveMedia(catalogue, media('/watch/movies/inception'))).toStrictEqual(undefined);
  expect(resolveMedia(catalogue, media('/watch/shows/the-wire'))).toStrictEqual(undefined);
});

test('a title collision and an id that is not a key still get one address each', () => {
  const catalogue = library(
    [
      album('harbour-lights', 'Other'),
      album('demo-album-01', 'Harbour Lights'),
      album('demo-album-02', 'Intro'),
      album('album-b', 'Intro'),
      album('bad--id', 'Broken'),
    ],
    [artist('Not-Key', 'North Wind'), artist('mira-sol', 'Mira Sol')],
  );
  expect(albumAddress(catalogue, 'harbour-lights')).toStrictEqual('/music/albums/other');
  expect(albumAddress(catalogue, 'demo-album-01')).toStrictEqual('/music/albums/harbour-lights-demoalbu');
  expect(albumAddress(catalogue, 'demo-album-02')).toStrictEqual('/music/albums/intro');
  expect(albumAddress(catalogue, 'album-b')).toStrictEqual('/music/albums/intro-albumb');
  expect(albumAddress(catalogue, 'bad--id')).toStrictEqual('/music/albums/broken');
  expect(artistAddress(catalogue, 'Not-Key', 'id')).toStrictEqual('/music/artists/north-wind');
  expect(artistAddress(catalogue, 'Not-Key', 'slug')).toStrictEqual('/music/artists/north-wind');
  expect(resolveMedia(catalogue, media('/music/albums/harbour-lights'))?.itemId).toStrictEqual('harbour-lights');
  expect(resolveMedia(catalogue, media('/music/artists/north-wind'))?.itemId).toStrictEqual('Not-Key');
  expect(itemAddress(undefined, 'mira-sol')).toStrictEqual(undefined);
  expect(itemAddress(catalogue, 'mira-sol')).toStrictEqual('/music/artists/mira-sol');
  expect(itemAddress(catalogue, 'demo-album-02')).toStrictEqual('/music/albums/intro');
  expect(itemAddress(catalogue, 'missing')).toStrictEqual(undefined);
  expect(resolveMedia(catalogue, media('/music/artists/north-wind'), 'id')?.path).toStrictEqual(
    '/music/artists/north-wind',
  );
});
