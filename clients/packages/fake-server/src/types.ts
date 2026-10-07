import type {
  CatalogueAlbum,
  CatalogueArtist,
  CatalogueLibrary,
  CatalogueLyricsKind,
  CatalogueTrack,
  CatalogueTrackFlag,
} from '../../../packages/ports/src/provisional/catalogue.ts';

/** A hand-written track before the generated media URLs are attached. */
export type FixtureTrack = Omit<CatalogueTrack, 'mediaUrl'>;

/** A hand-written album before the generated cover URL is attached. */
export type FixtureAlbum = Omit<CatalogueAlbum, 'coverUrl' | 'tracks'> & {
  tracks: readonly FixtureTrack[];
};

export type DemoAlbum = CatalogueAlbum;
export type DemoArtist = CatalogueArtist;
export type DemoLibrary = CatalogueLibrary & { kind: 'demo-fixtures' | 'folder' };

export type DemoTrack = CatalogueTrack;
export type DemoTrackFlag = CatalogueTrackFlag;
export type DemoLyricsKind = CatalogueLyricsKind;
export type DemoDisc = CatalogueAlbum['discs'][number];
export type DemoLicense = NonNullable<CatalogueAlbum['license']>;
