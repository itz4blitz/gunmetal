import { useState } from 'react';
import type { ReactNode } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { LibrarySearch, LibrarySearchHits } from '../content.ts';
import type { ShellLibrary, ShellTrack } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { TrackRow } from './TrackRow.tsx';

export type SearchProps = {
  searchLibrary?: LibrarySearch | undefined;
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

function TypeChip({
  id,
  label,
  pressed,
  onToggle,
}: {
  id: string;
  label: string;
  pressed: boolean;
  onToggle: () => void;
}) {
  return (
    <View
      id={id}
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected: pressed }}
      tabIndex={0}
      dataSet={{ searchChip: '1', pressed: pressed ? '1' : '0' }}
      onClick={onToggle}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onToggle();
        }
      }}
    >
      <Text>{label}</Text>
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

export function Search({
  searchLibrary = (): LibrarySearchHits => ({ albums: [], tracks: [] }),
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
  const hits = searchLibrary(library, query);
  const hasQuery = query.trim().length > 0;
  const visibleAlbums = showAlbums ? hits.albums : [];
  const visibleTracks = showTracks ? hits.tracks : [];
  const noVisibleHits = visibleAlbums.length === 0 && visibleTracks.length === 0;
  const albumFor = (track: ShellTrack) => library.albums.find((album) => album.id === track.albumId);

  return (
    <View id="destination-search">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.searchHeadline}
      </Text>
      <View id="search-field-wrap">
        <div id="search-affordance" aria-hidden="true" />
        <input
          id="search-field"
          type="search"
          value={query}
          placeholder={messages.searchPlaceholder}
          aria-label={messages.searchPlaceholder}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
        />
      </View>
      <View id="search-type-chips" accessibilityRole="group" accessibilityLabel={messages.searchTypeFilter}>
        <TypeChip
          id="search-chip-albums"
          label={messages.tabAlbums}
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
          pressed={showTracks}
          onToggle={() => {
            if (showTracks && !showAlbums) {
              return;
            }
            setShowTracks(!showTracks);
          }}
        />
      </View>
      {!hasQuery ? (
        <View id="search-recent" dataSet={{ emptyCard: '1', emptyRow: '1' }}>
          <View dataSet={{ emptyMark: '1' }} />
          <Text id="search-recent-heading" accessibilityRole="header" dataSet={{ emptyTitle: '1' }}>
            {messages.searchRecentHeading}
          </Text>
          <Text id="search-recent-empty" dataSet={{ emptyState: 'search-recent' }}>
            {messages.searchRecentEmpty}
          </Text>
        </View>
      ) : (
        <View id="search-results">
          <View id="search-results-meta">
            <Text id="search-results-count">
              {`${visibleAlbums.length + visibleTracks.length} ${messages.searchResultCount}`}
            </Text>
            <Text id="search-demo-notice">{messages.searchDemoLocalNotice}</Text>
          </View>
          <Text id="search-plugin-notice">{messages.searchPluginNotice}</Text>
          {noVisibleHits ? (
            <View id="search-no-hits" dataSet={{ emptyCard: '1', emptyRow: '1' }}>
              <View dataSet={{ emptyMark: '1' }} />
              <Text accessibilityRole="header" dataSet={{ emptyTitle: '1' }}>
                {messages.searchNoHits}
              </Text>
              <QueryEcho query={query.trim()} />
            </View>
          ) : (
            <>
              {visibleAlbums.length > 0 ? (
                <View dataSet={{ searchAlbums: '1' }}>
                  <View dataSet={{ searchGroupHead: '1' }}>
                    <Text
                      id="search-group-albums"
                      accessibilityRole="header"
                      dataSet={{ searchGroup: 'albums', type: 'title2' }}
                    >
                      {messages.tabAlbums}
                    </Text>
                    <Text dataSet={{ searchGroupCount: '1' }} aria-hidden="true">
                      {`${visibleAlbums.length}`}
                    </Text>
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
                    <Text
                      id="search-group-tracks"
                      accessibilityRole="header"
                      dataSet={{ searchGroup: 'tracks', type: 'title2' }}
                    >
                      {messages.tabTracks}
                    </Text>
                    <Text dataSet={{ searchGroupCount: '1' }} aria-hidden="true">
                      {`${visibleTracks.length}`}
                    </Text>
                  </View>
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
              ) : null}
            </>
          )}
        </View>
      )}
    </View>
  );
}
