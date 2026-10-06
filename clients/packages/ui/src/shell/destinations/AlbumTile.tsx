import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum } from '../library-types.ts';
import { CoverTile } from './CoverTile.tsx';

export type AlbumTileProps = {
  album: ShellAlbum;
  messages: DestinationMessages;
  onOpen: (albumId: string) => void;
};

function displayTitle(album: ShellAlbum, messages: DestinationMessages): string {
  if (album.hostile) {
    return messages.hostileAlbumLabel;
  }
  return album.title;
}

function displayArtist(album: ShellAlbum, messages: DestinationMessages): string {
  if (album.hostile) {
    return messages.hostileArtistLabel;
  }
  return album.artistName;
}

export function AlbumTile({ album, messages, onOpen }: AlbumTileProps) {
  const title = displayTitle(album, messages);
  const artist = displayArtist(album, messages);
  return (
    <View
      id={`album-tile-${album.id}`}
      dataSet={{ albumTile: album.id, hostile: album.hostile ? '1' : '0' }}
      accessibilityRole="button"
      accessibilityLabel={title}
      tabIndex={0}
      onClick={() => {
        onOpen(album.id);
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onOpen(album.id);
        }
      }}
    >
      <CoverTile
        tone={album.coverTone}
        label={title}
        size="grid"
        coverId={`cover-grid-${album.id}`}
      />
      <Text dataSet={{ albumTitle: '1' }}>{title}</Text>
      <Text dataSet={{ albumArtist: '1' }}>{artist}</Text>
    </View>
  );
}
