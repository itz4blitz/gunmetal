import { useState } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum, ShellLibrary, ShellTrack } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { TrackRow } from './TrackRow.tsx';

export type SearchProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
};

function demoLocalHits(
  library: ShellLibrary,
  query: string,
): { albums: readonly ShellAlbum[]; tracks: readonly ShellTrack[] } {
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) {
    return { albums: [], tracks: [] };
  }
  const albums = library.albums.filter(
    (album) =>
      album.title.toLowerCase().includes(needle) || album.artistName.toLowerCase().includes(needle),
  );
  const tracks: ShellTrack[] = [];
  for (const album of library.albums) {
    for (const track of album.tracks) {
      if (
        track.title.toLowerCase().includes(needle) ||
        track.artistName.toLowerCase().includes(needle)
      ) {
        tracks.push(track);
      }
    }
  }
  return { albums, tracks };
}

export function Search({
  messages,
  library,
  onOpenAlbum,
  onPlayAlbum,
  onPlayTrack,
}: SearchProps) {
  const [query, setQuery] = useState('');
  const hits = demoLocalHits(library, query);
  const hasQuery = query.trim().length > 0;

  return (
    <View id="destination-search">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.searchHeadline}
      </Text>
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
      {!hasQuery ? (
        <Text id="search-recent-empty" dataSet={{ emptyState: 'search-recent' }}>
          {messages.searchRecentEmpty}
        </Text>
      ) : (
        <View id="search-results">
          <Text id="search-demo-notice">{messages.searchDemoLocalNotice}</Text>
          {hits.albums.length === 0 && hits.tracks.length === 0 ? (
            <Text id="search-no-hits">{messages.searchNoHits}</Text>
          ) : (
            <>
              <View dataSet={{ searchAlbums: '1' }}>
                {hits.albums.map((album) => (
                  <AlbumTile
                    key={album.id}
                    album={album}
                    messages={messages}
                    onOpen={onOpenAlbum}
                    onPlay={onPlayAlbum}
                  />
                ))}
              </View>
              <View dataSet={{ searchTracks: '1' }}>
                {hits.tracks.map((track) => (
                  <TrackRow
                    key={track.id}
                    track={track}
                    messages={messages}
                    onPlay={onPlayTrack}
                  />
                ))}
              </View>
            </>
          )}
        </View>
      )}
    </View>
  );
}
