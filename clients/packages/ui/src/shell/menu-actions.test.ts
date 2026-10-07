import { expect, test } from 'vitest';
import { demoLibrary } from '../../../fake-server/src/catalogue.ts';
import { destinationMessages } from '../messages/en/destinations.ts';
import { catalogueMenuActions, resolveArtistNavigation } from './menu-actions.ts';

test('catalogue menus list Play, Play next, Add to queue, Go to album, Go to artist', () => {
  expect(catalogueMenuActions(destinationMessages())).toStrictEqual([
    { id: 'play', label: 'Play' },
    { id: 'play-next', label: 'Play next' },
    { id: 'add-to-queue', label: 'Add to queue' },
    { id: 'go-to-album', label: 'Go to album' },
    { id: 'go-to-artist', label: 'Go to artist' },
  ]);
});

test('go to artist opens the library artist tab or the first matching album', () => {
  const library = demoLibrary();
  expect(resolveArtistNavigation(library, 'Mira Sol')).toStrictEqual({ kind: 'artists-tab' });
  expect(resolveArtistNavigation(library, 'Ivy North')).toStrictEqual({
    kind: 'album',
    albumId: 'demo-album-06',
  });
  expect(resolveArtistNavigation(library, 'Nobody Here')).toStrictEqual({ kind: 'artists-tab' });
});
