import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';
import {
  EXTENSION_REPOSITORY_ID,
  EXTENSION_REPOSITORY_VERSION,
  extensionById,
  extensionTitle,
  addressStyle,
  extensionRepository,
  officialExtensionsRepository,
  storeDetailPath,
  storeIdFromPath,
} from './repository.ts';

test('the repository is the closed list extensions target', () => {
  expect(extensionRepository()).toStrictEqual({
    id: 'gunmetal.extensions',
    version: '1',
    extensions: [
      {
        id: 'cover-art',
        title: 'Metadata and artwork',
        version: '1.0.0',
        plane: 'server',
        slot: 'metadata-provider',
        status: 'on',
        summary: 'Fills missing album art and artist photos from MusicBrainz and Cover Art Archive.',
        detail: [
          'Runs inside this library host. It is not a downloaded package.',
          'Album art comes from embedded pictures first, then Cover Art Archive.',
          'Artist photos come from the Wikidata portrait on the MusicBrainz artist.',
        ],
        grants: ['library:write-artwork'],
      },
      {
        id: 'lyrics',
        title: 'Lyrics lookup',
        version: '1.0.0',
        plane: 'server',
        slot: 'lyrics-provider',
        status: 'not-in-build',
        summary: 'Would fetch lyrics for tracks whose files have none.',
        detail: ['This build does not run it. A package targeting this id would ask for the lyrics grant and nothing else.'],
        grants: ['lyrics:read'],
      },
      {
        id: 'catalogue-search',
        title: 'Catalog search',
        version: '1.0.0',
        plane: 'server',
        slot: 'search-provider',
        status: 'not-in-build',
        summary: 'Would search a remote catalog and return matches as data.',
        detail: ['This build searches the library it already holds. A remote search package is not loaded.'],
        grants: ['search:query'],
      },
      {
        id: 'scrobble',
        title: 'Scrobblers',
        version: '1.0.0',
        plane: 'server',
        slot: 'scrobbler',
        status: 'not-in-build',
        summary: 'Would send plays to a service the owner names.',
        detail: ['Nothing is sent. A scrobbler targeting this id would need its own consent before a play left the server.'],
        grants: ['scrobble:write'],
      },
      {
        id: 'themes',
        title: 'Themes',
        version: '1.0.0',
        plane: 'client',
        slot: 'theme-pack',
        status: 'not-in-build',
        summary: 'Would add theme packs as data on top of the built-in themes.',
        detail: ['Appearance already has the built-in themes. A theme pack would be colours and names, not code.'],
        grants: ['theme:apply'],
      },
      {
        id: 'home-rows',
        title: 'Home rows',
        version: '1.0.0',
        plane: 'client',
        slot: 'home-row',
        status: 'not-in-build',
        summary: 'Would add a home row described as data.',
        detail: ['Home already shows this library. A row package would name which albums to show, not ship a script.'],
        grants: ['home-row:read'],
      },
      {
        id: 'url-style',
        title: 'Address style',
        version: '1.0.0',
        plane: 'client',
        slot: 'route-style',
        status: 'on',
        summary: 'Addresses for artists, albums, tracks, movies and shows. This build uses unique name slugs.',
        detail: [
          'An official extension, reviewed with the others in itz4blitz/gunmetal-extensions.',
          'Slug style: /music/albums/harbour-lights and /watch/shows/the-wire.',
          'Id style: /music/albums/demo-album-01. Both open the same page. The address bar shows the slug.',
          'This build does not download a URL style. The choice is data.',
        ],
        grants: ['route:read'],
      },
    ],
  });
  expect(addressStyle()).toStrictEqual('slug');
  localStorage.setItem(
    'gunmetal.extension.choices',
    JSON.stringify({ 'url-style': { installed: true, values: { style: 'id' } } }),
  );
  expect(addressStyle()).toStrictEqual('id');
  localStorage.setItem(
    'gunmetal.extension.choices',
    JSON.stringify({ 'url-style': { installed: false, values: { style: 'id' } } }),
  );
  expect(addressStyle()).toStrictEqual('slug');
  localStorage.removeItem('gunmetal.extension.choices');
  expect(EXTENSION_REPOSITORY_ID).toStrictEqual('gunmetal.extensions');
  expect(EXTENSION_REPOSITORY_VERSION).toStrictEqual('1');
});

test('the catalogue points at the official extensions repository and does not fetch it', async () => {
  expect(officialExtensionsRepository()).toStrictEqual({
    forge: 'https://github.com',
    owner: 'itz4blitz',
    name: 'gunmetal-extensions',
    path: 'itz4blitz/gunmetal-extensions',
    url: 'https://github.com/itz4blitz/gunmetal-extensions',
  });
  const source = await readFile(join(dirname(fileURLToPath(import.meta.url)), 'repository.ts'), 'utf8');
  expect(source.includes('fetch(')).toStrictEqual(false);
  expect(source.includes('http.get')).toStrictEqual(false);
  expect(source.includes('import(')).toStrictEqual(false);
});

test('an extension id is a store page and an unknown id is not', () => {
  expect(storeDetailPath('url-style')).toStrictEqual('/store/url-style');
  expect(storeDetailPath('cover-art')).toStrictEqual('/store/cover-art');
  expect(extensionById('lyrics')?.title).toStrictEqual('Lyrics lookup');
  expect(extensionById('nope')).toStrictEqual(undefined);
  expect(extensionTitle('url-style')).toStrictEqual('Address style');
  expect(extensionTitle('nope')).toStrictEqual('nope');
  expect(storeIdFromPath('/store/scrobble')).toStrictEqual('scrobble');
  expect(storeIdFromPath('/store')).toStrictEqual(undefined);
  expect(storeIdFromPath('/store/nope')).toStrictEqual(undefined);
});
