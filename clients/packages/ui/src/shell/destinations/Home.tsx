import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { CoverTile } from './CoverTile.tsx';

export type HomeProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
  onOpenArtist?: ((artistKey: string) => void) | undefined;
  onPlayAlbum: (albumId: string) => void;
  onPlayNextAlbum?: ((albumId: string) => void) | undefined;
  onAddAlbumToQueue?: ((albumId: string) => void) | undefined;
  onSeeAll: () => void;
};

function EmptyCard({ state, title, body }: { state: string; title: string; body: string }) {
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

function activateKey(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

export function Home({
  messages,
  library,
  onOpenAlbum,
  onOpenArtist,
  onPlayAlbum,
  onPlayNextAlbum,
  onAddAlbumToQueue,
  onSeeAll,
}: HomeProps) {
  const albums = library.albums.filter((album) => !album.hostile);
  const spotlight = albums[0];
  const homeData = spotlight === undefined ? undefined : { artTone: spotlight.coverTone };

  return (
    <View id="destination-home" dataSet={homeData}>
      {spotlight !== undefined ? (
        <View id="home-spotlight" dataSet={{ homeSpotlight: '1' }}>
          <CoverTile
            tone={spotlight.coverTone}
            label={spotlight.title}
            size="detail"
            coverId={`cover-spotlight-${spotlight.id}`}
            artUrl={spotlight.coverUrl}
          />
          <View dataSet={{ spotlightCopy: '1' }}>
            <Text dataSet={{ spotlightEyebrow: '1' }}>{messages.featured}</Text>
            <Text accessibilityRole="header" dataSet={{ spotlightTitle: '1', type: 'display' }}>
              {spotlight.title}
            </Text>
            <Text dataSet={{ spotlightArtist: '1', type: 'title3' }}>{spotlight.artistName}</Text>
            <View dataSet={{ spotlightActions: '1' }}>
              <View dataSet={{ hexWrap: '1' }}>
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
              </View>
              <View
                id="home-spotlight-open"
                accessibilityRole="button"
                accessibilityLabel={messages.goToAlbum}
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
                <Text>{messages.goToAlbum}</Text>
              </View>
            </View>
          </View>
        </View>
      ) : (
        <Text id="destination-headline" accessibilityRole="header">
          {messages.homeHeadline}
        </Text>
      )}
      {/* History rows (continue / played / loved) stay hidden until C2 wires plays
          and loves; an empty shell must not spend the first screen on placeholders. */}
      <View id="home-row-recent" dataSet={{ homeRow: 'recent' }}>
        {albums.length === 0 ? (
          <EmptyCard state="recent" title={messages.recentlyAdded} body={messages.emptyRecentlyAdded} />
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
          </>
        )}
      </View>
    </View>
  );
}
