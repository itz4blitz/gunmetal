/**
 * The extension repository this build advertises.
 *
 * A package targets `gunmetal.extensions` version `1` by naming one of these
 * ids. The record is data: a title, a version, a plane, a slot, and the
 * grants it would ask for. This module does not fetch, import, or run
 * anything a record names (INT-074, SEC-EXT-018).
 */

import { EXTENSION_CHOICES_KEY, parseChoices } from './extension-choices.ts';

export const EXTENSION_REPOSITORY_ID = 'gunmetal.extensions';
export const EXTENSION_REPOSITORY_VERSION = '1';

/** The two official address styles. A plugin record names which one is canonical. */
export type AddressStyle = 'slug' | 'id';

/**
 * This build's canonical address style, from the official `url-style`
 * extension. Slugs are what the address bar shows. Ids still open.
 */
export function addressStyle(): AddressStyle {
  if (typeof localStorage === 'undefined') {
    return 'slug';
  }
  const saved = parseChoices(localStorage.getItem(EXTENSION_CHOICES_KEY), [{ id: 'url-style', status: 'on' }]);
  const choice = saved['url-style'];
  if (choice?.installed === true && choice.values.style === 'id') {
    return 'id';
  }
  return 'slug';
}

/**
 * The public GitHub repository where extension records are reviewed.
 * Membership is that repository's access list. This module does not clone,
 * fetch, or run it (SEC-EXT-018). A merged pull request there does not
 * install itself.
 */
export function officialExtensionsRepository(): {
  forge: 'https://github.com';
  owner: 'itz4blitz';
  name: 'gunmetal-extensions';
  path: 'itz4blitz/gunmetal-extensions';
  url: 'https://github.com/itz4blitz/gunmetal-extensions';
} {
  return {
    forge: 'https://github.com',
    owner: 'itz4blitz',
    name: 'gunmetal-extensions',
    path: 'itz4blitz/gunmetal-extensions',
    url: 'https://github.com/itz4blitz/gunmetal-extensions',
  };
}

export type ExtensionId =
  | 'cover-art'
  | 'lyrics'
  | 'catalogue-search'
  | 'scrobble'
  | 'themes'
  | 'home-rows'
  | 'url-style';

export type ExtensionPlane = 'server' | 'client';

export type ExtensionStatus = 'on' | 'not-in-build';

export type ExtensionRecord = {
  id: ExtensionId;
  title: string;
  version: string;
  plane: ExtensionPlane;
  slot: string;
  status: ExtensionStatus;
  summary: string;
  detail: readonly string[];
  grants: readonly string[];
};

function records(): readonly ExtensionRecord[] {
  return [
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
  ];
}

export function extensionRepository(): {
  id: typeof EXTENSION_REPOSITORY_ID;
  version: typeof EXTENSION_REPOSITORY_VERSION;
  extensions: readonly ExtensionRecord[];
} {
  return {
    id: EXTENSION_REPOSITORY_ID,
    version: EXTENSION_REPOSITORY_VERSION,
    extensions: records(),
  };
}

export function extensionById(id: string): ExtensionRecord | undefined {
  return records().find((entry) => entry.id === id);
}

/** The title of a known extension. An id that is not in the list is returned as itself. */
export function extensionTitle(id: string): string {
  return extensionById(id)?.title ?? id;
}

export function extensionPath(id: ExtensionId): `/settings/extensions/${ExtensionId}` {
  return `/settings/extensions/${id}`;
}

/** A store record stays in the store. Settings is a different door. */
export function storeDetailPath(id: ExtensionId): `/store/${ExtensionId}` {
  return `/store/${id}`;
}

export function storeIdFromPath(path: string): ExtensionId | undefined {
  const prefix = '/store/';
  if (!path.startsWith(prefix)) {
    return undefined;
  }
  const id = path.slice(prefix.length);
  return extensionById(id)?.id;
}

export function extensionIdFromPath(path: string): ExtensionId | undefined {
  const prefix = '/settings/extensions/';
  if (!path.startsWith(prefix)) {
    return undefined;
  }
  const id = path.slice(prefix.length);
  return extensionById(id)?.id;
}
