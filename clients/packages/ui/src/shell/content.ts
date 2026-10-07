import type {
  CatalogueAlbum,
  CatalogueLibrary,
  CatalogueLyricsKind,
  CatalogueTrack,
} from '../../../ports/src/provisional/catalogue.ts';

/**
 * A read over the library the composition root supplies: apps/demo passes a
 * demo-local substring filter (labelled as such in the UI); apps/web passes
 * the ServerPort search when the core's search lands. The Search surface
 * never imports fixture data itself.
 */
export type LibrarySearchHits = {
  albums: readonly CatalogueAlbum[];
  tracks: readonly CatalogueTrack[];
};

export type LibrarySearch = (library: CatalogueLibrary, query: string) => LibrarySearchHits;

/**
 * Lyrics content for a track, resolved by the composition root (fixture
 * verses today, the server's lyrics later). The surfaces render the lines
 * they are given and never read a fixture table themselves.
 */
export type LyricsResolver = (trackId: string, kind: CatalogueLyricsKind) => readonly string[];

/** Where lyrics are unavailable, the resolver returns this line. */
export const noLyrics: readonly string[] = ['This file has no lyrics.'];
