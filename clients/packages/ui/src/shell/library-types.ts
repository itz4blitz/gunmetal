/**
 * Re-exports of the provisional catalogue DTOs (packages/ports). The shell
 * reads the library through these types; the real generated declarations
 * replace them when WP-235/WP-040 land (see ports/src/provisional/).
 */
export type {
  CatalogueAlbum as ShellAlbum,
  CatalogueArtist as ShellArtist,
  CatalogueDisc as ShellDisc,
  CatalogueLicense as ShellLicense,
  CatalogueLibrary as ShellLibrary,
  CatalogueLyricsKind as ShellLyricsKind,
  CatalogueTrack as ShellTrack,
  CatalogueTrackFlag as ShellTrackFlag,
} from '../../../ports/src/provisional/catalogue.ts';
