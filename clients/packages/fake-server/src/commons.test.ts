import { expect, test } from 'vitest';
import { cylindersAlbum } from './commons.ts';

test('Cylinders is the real 2014 CC-BY-4.0 album with official track lengths', () => {
  const album = cylindersAlbum();
  expect(album.id).toStrictEqual('demo-album-09');
  expect(album.title).toStrictEqual('Cylinders');
  expect(album.artistName).toStrictEqual('Chris Zabriskie');
  expect(album.artistKey).toStrictEqual('chris-zabriskie');
  expect(album.year).toStrictEqual(2014);
  expect(album.hostile).toStrictEqual(false);
  expect(album.license).toStrictEqual({
    spdx: 'CC-BY-4.0',
    attribution: 'Cylinders by Chris Zabriskie',
    source: 'chriszabriskie.com',
  });
  expect(album.tracks.map((track) => [track.number, track.title, track.durationMs])).toStrictEqual([
    [1, 'Cylinder One', 176_000],
    [2, 'Cylinder Two', 228_000],
    [3, 'Cylinder Three', 166_000],
    [4, 'Cylinder Four', 177_000],
    [5, 'Cylinder Five', 174_000],
    [6, 'Cylinder Six', 106_000],
    [7, 'Cylinder Seven', 532_000],
    [8, 'Cylinder Eight', 339_000],
    [9, 'Cylinder Nine', 323_000],
  ]);
  expect(album.tracks.every((track) => track.artistName === 'Chris Zabriskie')).toStrictEqual(true);
  expect(album.tracks.every((track) => track.flag === 'ok')).toStrictEqual(true);
});
