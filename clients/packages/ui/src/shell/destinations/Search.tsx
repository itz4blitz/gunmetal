import { useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { LibrarySearch, LibrarySearchHits } from '../content.ts';
import { countNoun } from '../format.ts';
import { Icon } from '../Icon.tsx';
import type { ShellAlbum, ShellLibrary, ShellTrack } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { CoverTile } from './CoverTile.tsx';
import { ArtistAvatar, artistDisplayName } from './Library.tsx';
import { hostileArtistKeys, indexAlbums } from './library-index.ts';
import { TrackRow } from './TrackRow.tsx';

export type SearchProps = {
  searchLibrary?: LibrarySearch | undefined;
  /** A served library is not the demo-local filter, so that notice is not drawn. */
  served?: boolean | undefined;
  messages: DestinationMessages;
  library: ShellLibrary;
  /** Track the shell is playing now; its row paints the brass current state. */
  currentTrackId?: string | undefined;
  onOpenAlbum: (albumId: string) => void;
  onOpenArtist?: ((artistKey: string) => void) | undefined;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
  onPlayNextAlbum?: ((albumId: string) => void) | undefined;
  onAddAlbumToQueue?: ((albumId: string) => void) | undefined;
  onPlayNextTrack?: ((albumId: string, trackId: string) => void) | undefined;
  onAddTrackToQueue?: ((albumId: string, trackId: string) => void) | undefined;
};

/** Enter and Space press a control; every other key is left alone. */
function pressOn(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

function TypeChip({
  id,
  label,
  count,
  pressed,
  onToggle,
}: {
  id: string;
  label: string;
  count: number;
  pressed: boolean;
  onToggle: () => void;
}) {
  return (
    <View
      id={id}
      accessibilityRole="button"
      accessibilityLabel={label}
      aria-pressed={pressed}
      tabIndex={0}
      dataSet={{ searchChip: '1', pressed: pressed ? '1' : '0' }}
      onClick={onToggle}
      onKeyDown={(event) => {
        pressOn(event, onToggle);
      }}
    >
      <Text dataSet={{ searchChipLabel: '1' }}>{label}</Text>
      {/* The count is chrome; the accessible name stays the bare type. */}
      <Text dataSet={{ searchChipCount: '1' }}>{`${count}`}</Text>
    </View>
  );
}

/** The typed query is the one untrusted string this surface echoes: it goes
 * out as a text node inside its own bidirectional isolate, never as markup. */
function QueryEcho({ query }: { query: string }) {
  const echo: ReactNode = (
    <span data-search-query-echo="1" dir="auto">
      {`“${query}”`}
    </span>
  );
  return <Text dataSet={{ searchQueryEcho: '1' }}>{echo}</Text>;
}

/** The first thing the lookup answered, with what the card needs to show and play it. */
type TopHit = {
  kind: 'album' | 'track';
  kindLabel: string;
  albumId: string;
  title: string;
  byline: string;
  tone: string;
  coverUrl: string | undefined;
  play: () => void;
};

/** A hostile fixture album never shows its own text: the safe catalogue labels stand in. */
function albumHit(album: ShellAlbum, messages: DestinationMessages, onPlayAlbum: (albumId: string) => void): TopHit {
  return {
    kind: 'album',
    kindLabel: messages.albumEyebrow,
    albumId: album.id,
    title: album.hostile ? messages.hostileAlbumLabel : album.title,
    byline: album.hostile ? messages.hostileArtistLabel : album.artistName,
    tone: album.coverTone,
    coverUrl: album.coverUrl,
    play: () => {
      onPlayAlbum(album.id);
    },
  };
}

function trackHit(
  track: ShellTrack,
  album: ShellAlbum | undefined,
  messages: DestinationMessages,
  onPlayTrack: (albumId: string, trackId: string) => void,
): TopHit {
  const hostile = album?.hostile === true;
  return {
    kind: 'track',
    kindLabel: messages.searchKindTrack,
    albumId: track.albumId,
    title: hostile ? messages.hostileAlbumLabel : track.title,
    byline: hostile ? messages.hostileArtistLabel : track.artistName,
    tone: album === undefined ? '' : album.coverTone,
    coverUrl: album?.coverUrl,
    play: () => {
      onPlayTrack(track.albumId, track.id);
    },
  };
}

function TopResult({
  hit,
  messages,
  onOpenAlbum,
}: {
  hit: TopHit;
  messages: DestinationMessages;
  onOpenAlbum: (albumId: string) => void;
}) {
  const open = () => {
    onOpenAlbum(hit.albumId);
  };
  return (
    <View id="search-top" dataSet={{ searchTop: hit.kind }}>
      <Text accessibilityRole="header" dataSet={{ searchGroup: 'top' }}>
        {messages.searchTopResult}
      </Text>
      <View dataSet={{ searchTopCard: '1' }}>
        <View
          dataSet={{ searchTopOpen: '1' }}
          accessibilityRole="button"
          accessibilityLabel={`${messages.searchTopResult}: ${hit.title}`}
          tabIndex={0}
          onClick={open}
          onKeyDown={(event) => {
            pressOn(event, open);
          }}
        >
          <CoverTile tone={hit.tone} label={hit.title} size="grid" coverId="search-top-cover" artUrl={hit.coverUrl} />
          <View dataSet={{ searchTopText: '1' }}>
            <Text dataSet={{ searchTopTitle: '1' }}>{hit.title}</Text>
            <Text dataSet={{ searchTopByline: '1' }}>{`${hit.kindLabel} · ${hit.byline}`}</Text>
          </View>
        </View>
        <View dataSet={{ hexWrap: '1' }}>
          <View
            dataSet={{ brassHex: '1', searchTopPlay: '1' }}
            accessibilityRole="button"
            accessibilityLabel={`${messages.play} ${hit.title}`}
            tabIndex={0}
            onClick={hit.play}
            onKeyDown={(event) => {
              pressOn(event, hit.play);
            }}
          />
        </View>
      </View>
    </View>
  );
}

export function Search({
  searchLibrary = (): LibrarySearchHits => ({ albums: [], tracks: [] }),
  served = false,
  messages,
  library,
  currentTrackId,
  onOpenAlbum,
  onOpenArtist,
  onPlayAlbum,
  onPlayTrack,
  onPlayNextAlbum,
  onAddAlbumToQueue,
  onPlayNextTrack,
  onAddTrackToQueue,
}: SearchProps) {
  const [query, setQuery] = useState('');
  const [showAlbums, setShowAlbums] = useState(true);
  const [showTracks, setShowTracks] = useState(true);
  const fieldRef = useRef<HTMLInputElement | null>(null);
  // One album index per library change; every track hit resolves its album in O(1).
  const albumIndex = useMemo(() => indexAlbums(library), [library]);
  const hostileKeys = useMemo(() => hostileArtistKeys(library), [library]);
  const hits = searchLibrary(library, query);
  const hasQuery = query.trim().length > 0;
  const clearQuery = () => {
    setQuery('');
    fieldRef.current?.focus();
  };
  const visibleAlbums = showAlbums ? hits.albums : [];
  const visibleTracks = showTracks ? hits.tracks : [];
  const albumFor = (track: ShellTrack): ShellAlbum | undefined => albumIndex.get(track.albumId);
  // The top result is the first album the lookup answered, or else its first
  // track. No top result is the same fact as nothing to show.
  const topAlbum = visibleAlbums[0];
  const topTrack = visibleTracks[0];
  const top =
    topAlbum !== undefined
      ? albumHit(topAlbum, messages, onPlayAlbum)
      : topTrack !== undefined
        ? trackHit(topTrack, albumFor(topTrack), messages, onPlayTrack)
        : undefined;

  return (
    <View id="destination-search">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.searchHeadline}
      </Text>
      <View id="search-field-wrap">
        <View id="search-affordance">
          <Icon name="search" size={18} />
        </View>
        <input
          id="search-field"
          ref={fieldRef}
          type="search"
          value={query}
          placeholder={messages.searchPlaceholder}
          aria-label={messages.searchPlaceholder}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
        />
        {hasQuery ? (
          <View
            id="search-clear"
            dataSet={{ searchClear: '1' }}
            accessibilityRole="button"
            accessibilityLabel={messages.searchClear}
            tabIndex={0}
            onClick={clearQuery}
            onKeyDown={(event) => {
              pressOn(event, clearQuery);
            }}
          >
            <Icon name="close" size={16} />
          </View>
        ) : null}
      </View>
      {!hasQuery ? (
        <View id="search-idle">
          <Text id="search-hint">{messages.searchHint}</Text>
          {/* Browse by what the library holds: its artists. Without a way to
              open an artist, or without artists, there is nothing to offer. */}
          {onOpenArtist !== undefined && library.artists.length > 0 ? (
            <View id="search-browse">
              <View dataSet={{ searchGroupHead: '1' }}>
                <Text accessibilityRole="header" dataSet={{ searchGroup: 'artists' }}>
                  {messages.searchBrowseArtists}
                </Text>
                <Text dataSet={{ searchGroupCount: '1' }}>{`${library.artists.length}`}</Text>
              </View>
              <View dataSet={{ searchArtistGrid: '1' }}>
                {library.artists.map((artist) => {
                  const name = artistDisplayName(artist, hostileKeys, messages);
                  const open = () => {
                    onOpenArtist(artist.key);
                  };
                  return (
                    <View
                      key={artist.key}
                      dataSet={{ searchArtist: artist.key }}
                      accessibilityRole="button"
                      accessibilityLabel={name}
                      tabIndex={0}
                      onClick={open}
                      onKeyDown={(event) => {
                        pressOn(event, open);
                      }}
                    >
                      <ArtistAvatar name={name} imageUrl={artist.imageUrl} />
                      <Text dataSet={{ searchArtistName: '1' }}>{name}</Text>
                    </View>
                  );
                })}
              </View>
            </View>
          ) : null}
        </View>
      ) : (
        <View id="search-results">
          <View id="search-results-bar">
            <View id="search-type-chips" accessibilityRole="group" accessibilityLabel={messages.searchTypeFilter}>
              <TypeChip
                id="search-chip-albums"
                label={messages.tabAlbums}
                count={hits.albums.length}
                pressed={showAlbums}
                onToggle={() => {
                  if (showAlbums && !showTracks) {
                    return;
                  }
                  setShowAlbums(!showAlbums);
                }}
              />
              <TypeChip
                id="search-chip-tracks"
                label={messages.tabTracks}
                count={hits.tracks.length}
                pressed={showTracks}
                onToggle={() => {
                  if (showTracks && !showAlbums) {
                    return;
                  }
                  setShowTracks(!showTracks);
                }}
              />
            </View>
            <Text id="search-results-count">
              {countNoun(
                visibleAlbums.length + visibleTracks.length,
                messages.searchResultCountOne,
                messages.searchResultCount,
              )}
            </Text>
          </View>
          {top === undefined && library.albums.length === 0 && library.artists.length === 0 ? (
            <View id="search-empty-library">
              <View dataSet={{ searchEmptyMark: '1' }}>
                <Icon name="library" size={22} />
              </View>
              <Text accessibilityRole="header" dataSet={{ searchEmptyTitle: '1' }}>
                {messages.searchEmptyLibrary}
              </Text>
            </View>
          ) : top === undefined && hits.albums.length + hits.tracks.length > 0 ? (
            <View id="search-filter-empty">
              <View dataSet={{ searchEmptyMark: '1' }}>
                <Icon name="search" size={22} />
              </View>
              <Text accessibilityRole="header" dataSet={{ searchEmptyTitle: '1' }}>
                {messages.searchFilterHeadline}
              </Text>
              <Text dataSet={{ searchFilterRemaining: '1' }}>
                {showAlbums ? messages.tabAlbums : messages.tabTracks}
              </Text>
              <Text dataSet={{ searchFilterHint: '1' }}>{messages.searchFilterHint}</Text>
            </View>
          ) : top === undefined ? (
            <View id="search-no-hits">
              <View dataSet={{ searchEmptyMark: '1' }}>
                <Icon name="search" size={22} />
              </View>
              <Text accessibilityRole="header" dataSet={{ searchEmptyTitle: '1' }}>
                {messages.searchNoHits}
              </Text>
              <QueryEcho query={query.trim()} />
              <Text dataSet={{ searchEmptyHint: '1' }}>{messages.searchNoHitsHint}</Text>
            </View>
          ) : (
            <>
              <TopResult hit={top} messages={messages} onOpenAlbum={onOpenAlbum} />
              {visibleAlbums.length > 0 ? (
                <View dataSet={{ searchAlbums: '1' }}>
                  <View dataSet={{ searchGroupHead: '1' }}>
                    <Text id="search-group-albums" accessibilityRole="header" dataSet={{ searchGroup: 'albums' }}>
                      {messages.tabAlbums}
                    </Text>
                    <Text dataSet={{ searchGroupCount: '1' }}>{`${visibleAlbums.length}`}</Text>
                  </View>
                  <View dataSet={{ searchAlbumGrid: '1' }}>
                    {visibleAlbums.map((album, index) => (
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
              ) : null}
              {visibleTracks.length > 0 ? (
                <View dataSet={{ searchTracks: '1' }}>
                  <View dataSet={{ searchGroupHead: '1' }}>
                    <Text id="search-group-tracks" accessibilityRole="header" dataSet={{ searchGroup: 'tracks' }}>
                      {messages.tabTracks}
                    </Text>
                    <Text dataSet={{ searchGroupCount: '1' }}>{`${visibleTracks.length}`}</Text>
                  </View>
                  <View id="search-track-list">
                    {visibleTracks.map((track) => {
                      const album = albumFor(track);
                      return (
                        <TrackRow
                          key={track.id}
                          track={track}
                          messages={messages}
                          artistKey={album?.artistKey}
                          albumTitle={album?.title}
                          hostile={album?.hostile === true}
                          current={track.id === currentTrackId}
                          onPlay={onPlayTrack}
                          onPlayNext={onPlayNextTrack}
                          onAddToQueue={onAddTrackToQueue}
                          onGoToAlbum={onOpenAlbum}
                          onOpenArtist={onOpenArtist}
                        />
                      );
                    })}
                  </View>
                </View>
              ) : null}
            </>
          )}
          {/* The plugin line always; the demo-local line only while this is not served. */}
          <View id="search-notices">
            {served ? null : <Text id="search-demo-notice">{messages.searchDemoLocalNotice}</Text>}
            <Text id="search-plugin-notice">{messages.searchPluginNotice}</Text>
          </View>
        </View>
      )}
    </View>
  );
}
