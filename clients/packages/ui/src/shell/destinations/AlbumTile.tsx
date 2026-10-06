import type { KeyboardEvent, MouseEvent } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum } from '../library-types.ts';
import { CoverTile } from './CoverTile.tsx';

export type AlbumTileProps = {
  album: ShellAlbum;
  messages: DestinationMessages;
  onOpen: (albumId: string) => void;
  onPlay?: (albumId: string) => void;
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

function activatePlay(
  albumId: string,
  onOpen: (albumId: string) => void,
  onPlay: ((albumId: string) => void) | undefined,
): void {
  if (onPlay !== undefined) {
    onPlay(albumId);
    return;
  }
  onOpen(albumId);
}

export function AlbumTile({ album, messages, onOpen, onPlay }: AlbumTileProps) {
  const title = displayTitle(album, messages);
  const artist = displayArtist(album, messages);
  return (
    <View
      id={`album-tile-${album.id}`}
      dataSet={{ albumTile: album.id, hostile: album.hostile ? '1' : '0' }}
    >
      <View
        dataSet={{ albumOpen: '1' }}
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
        <View dataSet={{ albumArt: '1' }}>
          <CoverTile
            tone={album.coverTone}
            label={title}
            size="grid"
            coverId={`cover-grid-${album.id}`}
          />
          <View
            dataSet={{ albumPlay: '1' }}
            accessibilityRole="button"
            accessibilityLabel={messages.playAlbum}
            tabIndex={0}
            onClick={(event: MouseEvent<HTMLElement>) => {
              event.stopPropagation();
              activatePlay(album.id, onOpen, onPlay);
            }}
            onKeyDown={(event: KeyboardEvent<HTMLElement>) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                event.stopPropagation();
                activatePlay(album.id, onOpen, onPlay);
              }
            }}
          />
        </View>
        <Text dataSet={{ albumTitle: '1' }}>{title}</Text>
        <Text dataSet={{ albumArtist: '1' }}>{artist}</Text>
      </View>
    </View>
  );
}
