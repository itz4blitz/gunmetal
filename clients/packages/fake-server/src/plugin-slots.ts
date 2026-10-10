export type PluginSlotId =
  'metadata-provider' | 'lyrics-provider' | 'search-provider' | 'scrobbler' | 'theme-pack' | 'home-row';

/**
 * A first-party job, not a plugin load bit. `on` is Cover Art Archive while
 * this host is serving covers. The other jobs are absent or not plugins.
 */
export type FirstPartyJobStatus = 'on' | 'not-serving' | 'not-in-build' | 'not-a-plugin';

export type PluginSlot = {
  readonly id: PluginSlotId;
  readonly status: FirstPartyJobStatus;
};

/** Covers this library host serves: `GET /media/library/covers/{id}.jpg` or `.png`. */
const LIBRARY_COVER = '/media/library/covers/';

export function hostServesCovers(albums: readonly { readonly coverUrl: string }[]): boolean {
  return albums.some((album) => album.coverUrl.startsWith(LIBRARY_COVER));
}

/** The six jobs the extensions pane lists. No manifest, no version, no host. */
export function pluginSlots(coversServed: boolean): readonly PluginSlot[] {
  return [
    { id: 'metadata-provider', status: coversServed ? 'on' : 'not-serving' },
    { id: 'lyrics-provider', status: 'not-in-build' },
    { id: 'search-provider', status: 'not-in-build' },
    { id: 'scrobbler', status: 'not-in-build' },
    { id: 'theme-pack', status: 'not-a-plugin' },
    { id: 'home-row', status: 'not-a-plugin' },
  ];
}
