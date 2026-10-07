import type {
  CatalogueAlbum,
  CatalogueLibrary,
  CatalogueTrack,
} from '../../../packages/ports/src/provisional/catalogue.ts';
import { allTracks } from './catalogue.ts';

export type DemoLocalFilterHit = {
  albums: readonly CatalogueAlbum[];
  tracks: readonly CatalogueTrack[];
};

/**
 * Demo-local substring filter only — not CorePort search ranking.
 * Labelled as such wherever the UI surfaces results.
 */
export function demoLocalFilter(library: CatalogueLibrary, query: string): DemoLocalFilterHit {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) {
    return { albums: [], tracks: [] };
  }
  const albums = library.albums.filter((album) => {
    const license = album.license;
    return (
      album.title.toLowerCase().includes(needle) ||
      album.artistName.toLowerCase().includes(needle) ||
      (license !== undefined &&
        (license.spdx.toLowerCase().includes(needle) ||
          license.attribution.toLowerCase().includes(needle) ||
          license.source.toLowerCase().includes(needle)))
    );
  });
  const tracks = allTracks(library).filter((entry) => {
    return entry.title.toLowerCase().includes(needle) || entry.artistName.toLowerCase().includes(needle);
  });
  return { albums, tracks };
}
