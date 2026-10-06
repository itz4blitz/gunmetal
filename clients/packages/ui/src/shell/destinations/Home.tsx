import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { CoverTile } from './CoverTile.tsx';

export type HomeProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onSeeAll: () => void;
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

function activateKey(
  event: { key: string; preventDefault: () => void },
  action: () => void,
): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

export function Home({
  messages,
  library,
  onOpenAlbum,
  onPlayAlbum,
  onSeeAll,
}: HomeProps) {
  const albums = library.albums.filter((album) => !album.hostile);
  const spotlight = albums[0];
  const homeData =
    spotlight === undefined ? undefined : { artTone: spotlight.coverTone };

  return (
    <View id="destination-home" dataSet={homeData}>
      {spotlight !== undefined ? (
        <View id="home-spotlight" dataSet={{ homeSpotlight: '1' }}>
          <CoverTile
            tone={spotlight.coverTone}
            label={spotlight.title}
            size="detail"
            coverId={`cover-spotlight-${spotlight.id}`}
          />
          <View dataSet={{ spotlightCopy: '1' }}>
            <Text
              accessibilityRole="header"
              dataSet={{ spotlightTitle: '1', type: 'display' }}
            >
              {spotlight.title}
            </Text>
            <Text dataSet={{ spotlightArtist: '1', type: 'title3' }}>
              {spotlight.artistName}
            </Text>
            <View dataSet={{ spotlightActions: '1' }}>
              <View
                id="home-spotlight-play"
                accessibilityRole="button"
                accessibilityLabel={messages.play}
                tabIndex={0}
                dataSet={{ brassHex: '1' }}
                onClick={() => {
                  onPlayAlbum(spotlight.id);
                }}
                onKeyDown={(event) => {
                  activateKey(event, () => {
                    onPlayAlbum(spotlight.id);
                  });
                }}
              >
                <Text>{messages.play}</Text>
              </View>
              <View
                id="home-spotlight-open"
                accessibilityRole="button"
                accessibilityLabel={messages.open}
                tabIndex={0}
                dataSet={{ spotlightOpen: '1' }}
                onClick={() => {
                  onOpenAlbum(spotlight.id);
                }}
                onKeyDown={(event) => {
                  activateKey(event, () => {
                    onOpenAlbum(spotlight.id);
                  });
                }}
              >
                <Text>{messages.open}</Text>
              </View>
            </View>
          </View>
        </View>
      ) : null}
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
            <View dataSet={{ homeRowHead: '1' }}>
              <Text accessibilityRole="header" dataSet={{ homeTitle: '1', type: 'title2' }}>
                {messages.recentlyAdded}
              </Text>
              <View
                id="home-see-all-recent"
                accessibilityRole="button"
                accessibilityLabel={messages.seeAll}
                tabIndex={0}
                dataSet={{ homeSeeAll: '1' }}
                onClick={onSeeAll}
                onKeyDown={(event) => {
                  activateKey(event, onSeeAll);
                }}
              >
                <Text>{messages.seeAll}</Text>
              </View>
            </View>
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
