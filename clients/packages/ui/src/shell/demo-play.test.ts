import { expect, test } from 'vitest';
import { demoLibrary } from '../../../fake-server/src/catalogue.ts';
import { applyPlayback, resolveAlbumPlayback, resolveTrackPlayback } from './demo-play.ts';

test('resolve playback helpers refuse missing library albums and mismatched tracks', () => {
  const library = demoLibrary();
  expect(resolveAlbumPlayback(undefined, 'demo-album-01')).toStrictEqual(undefined);
  expect(resolveAlbumPlayback(library, 'missing')).toStrictEqual(undefined);
  const album = resolveAlbumPlayback(library, 'demo-album-01');
  expect(album?.trackId).toStrictEqual('demo-track-01-01');
  expect(resolveTrackPlayback(undefined, 'demo-album-01', 'demo-track-01-01')).toStrictEqual(
    undefined,
  );
  expect(resolveTrackPlayback(library, 'demo-album-01', 'missing')).toStrictEqual(undefined);
  expect(resolveTrackPlayback(library, 'wrong-album', 'demo-track-01-01')).toStrictEqual(undefined);
  expect(resolveTrackPlayback(library, 'demo-album-01', 'demo-track-01-02')?.trackId).toStrictEqual(
    'demo-track-01-02',
  );
  expect(applyPlayback(library, 'demo-album-02', undefined).title).toStrictEqual('Clock In');
  expect(applyPlayback(library, 'demo-album-02', 'demo-track-02-02').title).toStrictEqual(
    'Freight Elevator',
  );
  expect(applyPlayback(undefined, 'demo-album-01', undefined).trackId).toStrictEqual(undefined);
  expect(applyPlayback(library, 'missing', 'missing').trackId).toStrictEqual(undefined);
});
