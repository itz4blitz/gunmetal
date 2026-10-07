import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { artistInitial } from '../format.ts';
import type { ShellAlbum, ShellArtist, ShellLibrary } from '../library-types.ts';
import { albumsForArtist } from '../playback.ts';
import { AlbumTile } from './AlbumTile.tsx';

export type ArtistDetailProps = {
  artist: ShellArtist | undefined;
  library: ShellLibrary;
  messages: DestinationMessages;
  onBack: () => void;
  onOpenAlbum: (albumId: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onOpenArtist?: (artistKey: string) => void;
  onPlayNextAlbum?: (albumId: string) => void;
  onAddAlbumToQueue?: (albumId: string) => void;
};

function activateKey(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

function artistHeading(artist: ShellArtist, albums: readonly ShellAlbum[], messages: DestinationMessages): string {
  if (albums.some((album) => album.hostile)) {
    return messages.hostileArtistLabel;
  }
  return artist.name;
}

export function ArtistDetail({
  artist,
  library,
  messages,
  onBack,
  onOpenAlbum,
  onPlayAlbum,
  onOpenArtist,
  onPlayNextAlbum,
  onAddAlbumToQueue,
}: ArtistDetailProps) {
  if (artist === undefined) {
    return (
      <View id="destination-artist-missing">
        <Text id="destination-headline" accessibilityRole="header">
          {messages.artistMissing}
        </Text>
        <View
          id="artist-back"
          accessibilityRole="button"
          accessibilityLabel={messages.backToLibrary}
          tabIndex={0}
          onClick={onBack}
          onKeyDown={(event) => {
            activateKey(event, onBack);
          }}
        >
          <Text>{messages.backToLibrary}</Text>
        </View>
      </View>
    );
  }

  const albums = albumsForArtist(library, artist);
  const first = albums[0];
  const heading = artistHeading(artist, albums, messages);
  const artTone = first?.coverTone;
  const artistData =
    artTone === undefined
      ? { artistKey: artist.key }
      : { artistKey: artist.key, artTone };

  return (
    <View id="destination-artist" dataSet={artistData}>
      <View
        id="artist-back"
        accessibilityRole="button"
        accessibilityLabel={messages.backToLibrary}
        tabIndex={0}
        onClick={onBack}
        onKeyDown={(event) => {
          activateKey(event, onBack);
        }}
      >
        <Text>{messages.backToLibrary}</Text>
      </View>
      <View dataSet={{ artistHeader: '1' }}>
        <View dataSet={{ artistHero: '1', artistAvatar: 'hero', artistAvatarNut: '1' }} aria-hidden="true">
          <Text dataSet={{ artistInitial: '1' }}>{artistInitial(heading)}</Text>
        </View>
        <View dataSet={{ artistHeaderText: '1' }}>
          <Text id="destination-headline" accessibilityRole="header">
            {heading}
          </Text>
          <Text dataSet={{ artistAlbumCount: '1' }}>
            {`${albums.length} ${messages.artistAlbumCount}`}
          </Text>
          <View
            id="artist-play"
            accessibilityRole="button"
            accessibilityLabel={messages.play}
            tabIndex={0}
            dataSet={{ brassHex: '1' }}
            onClick={() => {
              if (first !== undefined) {
                onPlayAlbum(first.id);
              }
            }}
            onKeyDown={(event) => {
              activateKey(event, () => {
                if (first !== undefined) {
                  onPlayAlbum(first.id);
                }
              });
            }}
          >
            <Text>{messages.play}</Text>
          </View>
        </View>
      </View>
      <View id="artist-album-grid" dataSet={{ albumGrid: '1' }}>
        {albums.map((album, index) => (
          <AlbumTile
            key={album.id}
            album={album}
            messages={messages}
            staggerIndex={index}
            onOpen={onOpenAlbum}
            onPlay={onPlayAlbum}
            onPlayNext={onPlayNextAlbum}
            onAddToQueue={onAddAlbumToQueue}
            onOpenArtist={onOpenArtist}
          />
        ))}
      </View>
    </View>
  );
}
