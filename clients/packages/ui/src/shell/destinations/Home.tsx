import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum, ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';

export type HomeProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
  onPlayAlbum: (albumId: string) => void;
};

function fixtureAlbums(library: ShellLibrary): readonly ShellAlbum[] {
  return library.albums.filter((album) => !album.hostile);
}

function EmptyCard({ state, copy }: { state: string; copy: string }) {
  return (
    <View dataSet={{ emptyRow: '1', emptyCard: '1' }}>
      <Text dataSet={{ emptyState: state }}>{copy}</Text>
    </View>
  );
}

function AlbumRow({
  albums,
  messages,
  onOpenAlbum,
  onPlayAlbum,
}: {
  albums: readonly ShellAlbum[];
  messages: DestinationMessages;
  onOpenAlbum: (albumId: string) => void;
  onPlayAlbum: (albumId: string) => void;
}) {
  return (
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
  );
}

export function Home({ messages, library, onOpenAlbum, onPlayAlbum }: HomeProps) {
  const albums = fixtureAlbums(library);

  return (
    <View id="destination-home">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.homeHeadline}
      </Text>
      <View id="home-row-continue" dataSet={{ homeRow: 'continue' }}>
        <Text accessibilityRole="header" dataSet={{ homeTitle: '1', type: 'title2' }}>
          {messages.continueListening}
        </Text>
        <EmptyCard state="continue" copy={messages.emptyContinue} />
      </View>
      <View id="home-row-played" dataSet={{ homeRow: 'played' }}>
        <Text accessibilityRole="header" dataSet={{ homeTitle: '1', type: 'title2' }}>
          {messages.recentlyPlayed}
        </Text>
        {albums.length === 0 ? (
          <EmptyCard state="played" copy={messages.emptyRecentlyPlayed} />
        ) : (
          <AlbumRow
            albums={albums}
            messages={messages}
            onOpenAlbum={onOpenAlbum}
            onPlayAlbum={onPlayAlbum}
          />
        )}
      </View>
      <View id="home-row-recent" dataSet={{ homeRow: 'recent' }}>
        <Text accessibilityRole="header" dataSet={{ homeTitle: '1', type: 'title2' }}>
          {messages.recentlyAdded}
        </Text>
        {albums.length === 0 ? (
          <EmptyCard state="recent" copy={messages.emptyRecentlyPlayed} />
        ) : (
          <AlbumRow
            albums={albums}
            messages={messages}
            onOpenAlbum={onOpenAlbum}
            onPlayAlbum={onPlayAlbum}
          />
        )}
      </View>
      <View id="home-row-loved" dataSet={{ homeRow: 'loved' }}>
        <Text accessibilityRole="header" dataSet={{ homeTitle: '1', type: 'title2' }}>
          {messages.loved}
        </Text>
        <EmptyCard state="loved" copy={messages.emptyLoved} />
      </View>
    </View>
  );
}
