import type { DestinationMessages } from '../messages/en/destinations.ts';
import type { ShellLibrary } from './library-types.ts';

export type CatalogueMenuActionId =
  | 'play'
  | 'play-next'
  | 'add-to-queue'
  | 'go-to-album'
  | 'go-to-artist';

export type CatalogueMenuAction = {
  id: CatalogueMenuActionId;
  label: string;
};

export type ArtistNavigation = { kind: 'artists-tab' } | { kind: 'album'; albumId: string };

export function catalogueMenuActions(messages: DestinationMessages): readonly CatalogueMenuAction[] {
  return [
    { id: 'play', label: messages.play },
    { id: 'play-next', label: messages.playNext },
    { id: 'add-to-queue', label: messages.addToQueue },
    { id: 'go-to-album', label: messages.goToAlbum },
    { id: 'go-to-artist', label: messages.goToArtist },
  ];
}

export function resolveArtistNavigation(library: ShellLibrary, artistName: string): ArtistNavigation {
  if (library.artists.some((artist) => artist.name === artistName)) {
    return { kind: 'artists-tab' };
  }
  const album = library.albums.find(
    (entry) =>
      entry.artistName === artistName ||
      entry.tracks.some((track) => track.artistName === artistName),
  );
  if (album !== undefined) {
    return { kind: 'album', albumId: album.id };
  }
  return { kind: 'artists-tab' };
}
