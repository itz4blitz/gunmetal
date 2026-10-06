import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';

export type HomeProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
  onPlayAlbum: (albumId: string) => void;
};

function EmptyCard({
  state,
  title,
  body,
}: {
  state: string;
  title: string;
  body: string;
}) {
  return (
    <View dataSet={{ emptyRow: '1', emptyCard: '1' }}>
      <View dataSet={{ emptyMark: '1' }} />
      <Text accessibilityRole="header" dataSet={{ emptyTitle: '1' }}>
        {title}
      </Text>
      <Text dataSet={{ emptyState: state }}>{body}</Text>
    </View>
  );
}

export function Home({ messages, library, onOpenAlbum, onPlayAlbum }: HomeProps) {
  const albums = library.albums.filter((album) => !album.hostile);

  return (
    <View id="destination-home">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.homeHeadline}
      </Text>
      <View id="home-row-continue" dataSet={{ homeRow: 'continue' }}>
        <EmptyCard
          state="continue"
          title={messages.continueListening}
          body={messages.emptyContinue}
        />
      </View>
      <View id="home-row-played" dataSet={{ homeRow: 'played' }}>
        <EmptyCard
          state="played"
          title={messages.recentlyPlayed}
          body={messages.emptyRecentlyPlayed}
        />
      </View>
      <View id="home-row-recent" dataSet={{ homeRow: 'recent' }}>
        {albums.length === 0 ? (
          <EmptyCard
            state="recent"
            title={messages.recentlyAdded}
            body={messages.emptyRecentlyAdded}
          />
        ) : (
          <>
            <Text accessibilityRole="header" dataSet={{ homeTitle: '1', type: 'title2' }}>
              {messages.recentlyAdded}
            </Text>
            <View dataSet={{ albumRow: '1' }}>
              {albums.map((album) => (
                <AlbumTile
                  key={album.id}
                  album={album}
                  messages={messages}
                  onOpen={onOpenAlbum}
                  onPlay={onPlayAlbum}
                />
              ))}
            </View>
          </>
        )}
      </View>
      <View id="home-row-loved" dataSet={{ homeRow: 'loved' }}>
        <EmptyCard state="loved" title={messages.loved} body={messages.emptyLoved} />
      </View>
    </View>
  );
}
