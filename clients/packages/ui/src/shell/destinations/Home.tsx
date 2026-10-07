import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { artistInitial, staggerSlot } from '../format.ts';
import type { ShellAlbum, ShellArtist, ShellLibrary } from '../library-types.ts';
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
  /**
   * Album id of the release that is playing right now, for the brass
   * where-you-are state on shelf tiles. Playback state does not reach Home
   * from the shell yet (C2 wires playback into surfaces); the prop stays
   * optional so the composition root can switch it on without layout work.
   */
  playingAlbumId?: string | undefined;
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

/** Home browses the releases it can actually open; hostile fixtures stay out. */
function browsableAlbums(library: ShellLibrary): readonly ShellAlbum[] {
  return library.albums.filter((album) => !album.hostile);
}

/**
 * Artists seen through the browsable releases. An artist whose only album is
 * hostile does not appear on Home (Library owns that corpus with its label
 * swap); a mixed artist still appears under the catalogue-safe label.
 */
function shelfArtists(library: ShellLibrary, albums: readonly ShellAlbum[]): readonly ShellArtist[] {
  const browsable = new Set(albums.map((album) => album.id));
  return library.artists.filter((artist) => artist.albumIds.some((id) => browsable.has(id)));
}

/** Same rule as the Library rows: one hostile release swaps the whole name. */
function artistDisplayName(artist: ShellArtist, library: ShellLibrary, messages: DestinationMessages): string {
  const hostile = artist.albumIds.some((id) => library.albums.some((album) => album.id === id && album.hostile));
  return hostile ? messages.hostileArtistLabel : artist.name;
}

function artistTone(artist: ShellArtist, albums: readonly ShellAlbum[]): string | undefined {
  for (const id of artist.albumIds) {
    const album = albums.find((candidate) => candidate.id === id);
    if (album !== undefined) {
      return album.coverTone;
    }
  }
  return undefined;
}

type ArtistTileProps = {
  artist: ShellArtist;
  name: string;
  tone: string | undefined;
  onOpen: (artistKey: string) => void;
  staggerIndex: number;
};

/** People are nuts (design-language §7): hex avatar art, name under it. */
function ArtistTile({ artist, name, tone, onOpen, staggerIndex }: ArtistTileProps) {
  const hasArt = artist.imageUrl !== undefined && artist.imageUrl !== '';
  const art = hasArt
    ? {
        backgroundImage: `url("${artist.imageUrl}")`,
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }
    : undefined;
  return (
    <View id={`artist-tile-${artist.key}`} dataSet={{ artistTile: artist.key, tileStagger: staggerSlot(staggerIndex) }}>
      <View
        dataSet={{ artistHex: '1' }}
        accessibilityRole="button"
        accessibilityLabel={name}
        tabIndex={0}
        onClick={() => {
          onOpen(artist.key);
        }}
        onKeyDown={(event) => {
          activateKey(event, () => {
            onOpen(artist.key);
          });
        }}
      >
        <View dataSet={{ artistHexRing: '1' }} aria-hidden="true">
          <View dataSet={{ artistAvatar: '1', ...(tone === undefined ? {} : { coverTone: tone }) }} style={art}>
            {hasArt ? null : <Text dataSet={{ artistInitial: '1' }}>{artistInitial(name)}</Text>}
          </View>
        </View>
      </View>
      <View
        dataSet={{ artistOpen: '1' }}
        accessibilityRole="button"
        accessibilityLabel={name}
        tabIndex={0}
        onClick={() => {
          onOpen(artist.key);
        }}
        onKeyDown={(event) => {
          activateKey(event, () => {
            onOpen(artist.key);
          });
        }}
      >
        <Text dataSet={{ artistTileName: '1' }}>{name}</Text>
      </View>
    </View>
  );
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
  playingAlbumId,
}: HomeProps) {
  const albums = browsableAlbums(library);
  const spotlight = albums[0];
  const homeData = spotlight === undefined ? undefined : { artTone: spotlight.coverTone };
  const artists = shelfArtists(library, albums);

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
                  playing={album.id === playingAlbumId}
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
      {artists.length === 0 ? null : (
        <View id="home-row-artists" dataSet={{ homeRow: 'artists' }}>
          <View dataSet={{ homeRowHead: '1' }}>
            <Text accessibilityRole="header" dataSet={{ homeTitle: '1', type: 'title2' }}>
              {messages.tabArtists}
            </Text>
          </View>
          <View dataSet={{ artistShelf: '1' }}>
            {artists.map((artist, index) => (
              <ArtistTile
                key={artist.key}
                artist={artist}
                name={artistDisplayName(artist, library, messages)}
                tone={artistTone(artist, albums)}
                onOpen={onOpenArtist ?? (() => undefined)}
                staggerIndex={index}
              />
            ))}
          </View>
        </View>
      )}
    </View>
  );
}
