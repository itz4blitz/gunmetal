// Registry: re-exports only, one line per module, sorted.
export type {
  ClientGrant,
  ClientPluginManifest,
  ClientPluginSlot,
  PluginConsent,
  PluginRecord,
} from './plugins/types.ts';
export { grantsSatisfy, isClientGrant, isClientPluginSlot, requiredGrantForSlot } from './plugins/types.ts';
export type {
  CatalogueAlbum,
  CatalogueArtist,
  CatalogueDisc,
  CatalogueLicense,
  CatalogueLibrary,
  CatalogueLyricsKind,
  CatalogueTrack,
  CatalogueTrackFlag,
} from './provisional/catalogue.ts';
export type { PlayerQueueLine, PlayerSnapshot } from './provisional/player.ts';
