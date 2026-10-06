import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';

export type HomeProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
};

export function Home({ messages, library, onOpenAlbum }: HomeProps) {
  return (
    <View id="destination-home">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.homeHeadline}
      </Text>
      <View id="home-row-recent" dataSet={{ homeRow: 'recent' }}>
        <Text accessibilityRole="header" dataSet={{ homeTitle: '1' }}>
          {messages.recentlyAdded}
        </Text>
        <View dataSet={{ albumRow: '1' }}>
          {library.albums
            .filter((album) => !album.hostile)
            .map((album) => (
              <AlbumTile
                key={album.id}
                album={album}
                messages={messages}
                onOpen={onOpenAlbum}
              />
            ))}
        </View>
      </View>
      <View id="home-row-continue" dataSet={{ homeRow: 'continue' }}>
        <Text accessibilityRole="header" dataSet={{ homeTitle: '1' }}>
          {messages.continueListening}
        </Text>
        <View dataSet={{ emptyRow: '1' }}>
          <Text dataSet={{ emptyState: 'continue' }}>{messages.emptyContinue}</Text>
        </View>
      </View>
      <View id="home-row-loved" dataSet={{ homeRow: 'loved' }}>
        <Text accessibilityRole="header" dataSet={{ homeTitle: '1' }}>
          {messages.loved}
        </Text>
        <View dataSet={{ emptyRow: '1' }}>
          <Text dataSet={{ emptyState: 'loved' }}>{messages.emptyLoved}</Text>
        </View>
      </View>
    </View>
  );
}
