import type { DemoAlbum, DemoLibrary, DemoTrack } from './types.ts';
import { allTracks } from './catalogue.ts';

export type DemoLocalFilterHit = {
  albums: readonly DemoAlbum[];
  tracks: readonly DemoTrack[];
};

/**
 * Demo-local substring filter only — not CorePort search ranking.
 * Labelled as such wherever the UI surfaces results.
 */
export function demoLocalFilter(library: DemoLibrary, query: string): DemoLocalFilterHit {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) {
    return { albums: [], tracks: [] };
  }
  const albums = library.albums.filter((album) => {
    return (
      album.title.toLowerCase().includes(needle) ||
      album.artistName.toLowerCase().includes(needle)
    );
  });
  const tracks = allTracks(library).filter((entry) => {
    return (
      entry.title.toLowerCase().includes(needle) ||
      entry.artistName.toLowerCase().includes(needle)
    );
  });
  return { albums, tracks };
}
