import { useState } from 'react';
import { Text, View } from 'react-native-web';
import { demoLocalFilter } from '../../../../fake-server/src/filter.ts';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary, ShellTrack } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { TrackRow } from './TrackRow.tsx';

export type SearchProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
  onOpenArtist?: (artistKey: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
  onPlayNextAlbum?: (albumId: string) => void;
  onAddAlbumToQueue?: (albumId: string) => void;
  onPlayNextTrack?: (albumId: string, trackId: string) => void;
  onAddTrackToQueue?: (albumId: string, trackId: string) => void;
};

function demoLocalHits(
  library: ShellLibrary,
  query: string,
): { albums: readonly ShellLibrary['albums'][number][]; tracks: readonly ShellTrack[] } {
  return demoLocalFilter(library, query);
}

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

export function Search({
  messages,
  library,
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
  const hits = demoLocalHits(library, query);
  const hasQuery = query.trim().length > 0;
  const visibleAlbums = showAlbums ? hits.albums : [];
  const visibleTracks = showTracks ? hits.tracks : [];
  const noVisibleHits = visibleAlbums.length === 0 && visibleTracks.length === 0;

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
      <View
        id="search-type-chips"
        accessibilityRole="group"
        accessibilityLabel={messages.searchTypeFilter}
      >
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
          <Text
            id="search-recent-heading"
            accessibilityRole="header"
            dataSet={{ emptyTitle: '1' }}
          >
            {messages.searchRecentHeading}
          </Text>
          <Text id="search-recent-empty" dataSet={{ emptyState: 'search-recent' }}>
            {messages.searchRecentEmpty}
          </Text>
        </View>
      ) : (
        <View id="search-results">
          <Text id="search-demo-notice">{messages.searchDemoLocalNotice}</Text>
          <Text id="search-plugin-notice">{messages.searchPluginNotice}</Text>
          {noVisibleHits ? (
            <View id="search-no-hits" dataSet={{ emptyCard: '1', emptyRow: '1' }}>
              <View dataSet={{ emptyMark: '1' }} />
              <Text accessibilityRole="header" dataSet={{ emptyTitle: '1' }}>
                {messages.searchNoHits}
              </Text>
              <Text dataSet={{ emptyState: 'search-no-hits' }}>{messages.searchDemoLocalNotice}</Text>
            </View>
          ) : (
            <>
              {visibleAlbums.length > 0 ? (
                <View dataSet={{ searchAlbums: '1' }}>
                  <Text
                    id="search-group-albums"
                    accessibilityRole="header"
                    dataSet={{ searchGroup: 'albums', type: 'title2' }}
                  >
                    {messages.tabAlbums}
                  </Text>
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
                  <Text
                    id="search-group-tracks"
                    accessibilityRole="header"
                    dataSet={{ searchGroup: 'tracks', type: 'title2' }}
                  >
                    {messages.tabTracks}
                  </Text>
                  {visibleTracks.map((track) => (
                    <TrackRow
                      key={track.id}
                      track={track}
                      messages={messages}
                      artistKey={
                        library.albums.find((album) => album.id === track.albumId)?.artistKey
                      }
                      onPlay={onPlayTrack}
                      onPlayNext={onPlayNextTrack}
                      onAddToQueue={onAddTrackToQueue}
                      onGoToAlbum={onOpenAlbum}
                      onOpenArtist={onOpenArtist}
                    />
                  ))}
                </View>
              ) : null}
            </>
          )}
        </View>
      )}
    </View>
  );
}
