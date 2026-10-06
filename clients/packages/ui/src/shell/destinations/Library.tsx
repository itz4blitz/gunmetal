import { useState } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { formatDuration } from '../format.ts';
import type { ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { TrackRow } from './TrackRow.tsx';

export type LibraryTab = 'albums' | 'artists' | 'tracks';

export type LibraryProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
};

function artistInitial(name: string): string {
  const trimmed = name.trim();
  if (trimmed.length === 0) {
    return '?';
  }
  return trimmed.charAt(0).toUpperCase();
}

export function Library({
  messages,
  library,
  onOpenAlbum,
  onPlayAlbum,
  onPlayTrack,
}: LibraryProps) {
  const [tab, setTab] = useState<LibraryTab>('albums');

  return (
    <View id="destination-library">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.libraryHeadline}
      </Text>
      <View id="library-tabs" accessibilityRole="tablist" accessibilityLabel={messages.libraryHeadline}>
        <TabButton
          id="library-tab-albums"
          label={messages.tabAlbums}
          selected={tab === 'albums'}
          onSelect={() => {
            setTab('albums');
          }}
        />
        <TabButton
          id="library-tab-artists"
          label={messages.tabArtists}
          selected={tab === 'artists'}
          onSelect={() => {
            setTab('artists');
          }}
        />
        <TabButton
          id="library-tab-tracks"
          label={messages.tabTracks}
          selected={tab === 'tracks'}
          onSelect={() => {
            setTab('tracks');
          }}
        />
      </View>
      {tab === 'albums' ? (
        <>
          <Text id="library-section-count">
            {`${library.albums.length} ${messages.artistAlbumCount}`}
          </Text>
          <View id="library-album-grid" dataSet={{ albumGrid: '1' }}>
            {library.albums.map((album) => (
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
      ) : null}
      {tab === 'artists' ? (
        <View id="library-artist-list">
          {library.artists.map((artist) => (
            <View
              key={artist.key}
              id={`artist-row-${artist.key}`}
              dataSet={{ artistRow: artist.key }}
              accessibilityRole="button"
              accessibilityLabel={artist.name}
              tabIndex={0}
              onClick={() => {
                const first = artist.albumIds[0];
                if (first !== undefined) {
                  onOpenAlbum(first);
                }
              }}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  const first = artist.albumIds[0];
                  if (first !== undefined) {
                    onOpenAlbum(first);
                  }
                }
              }}
            >
              <View dataSet={{ artistAvatar: '1' }} aria-hidden="true">
                <Text dataSet={{ artistInitial: '1' }}>{artistInitial(artist.name)}</Text>
              </View>
              <Text dataSet={{ artistName: '1' }}>{artist.name}</Text>
              <Text dataSet={{ artistCount: '1' }}>
                {`${artist.albumIds.length} ${messages.artistAlbumCount}`}
              </Text>
            </View>
          ))}
        </View>
      ) : null}
      {tab === 'tracks' ? (
        <View id="library-track-list">
          {library.albums.flatMap((album) =>
            album.tracks.map((track) => (
              <View key={track.id} dataSet={{ libraryTrack: track.id }}>
                <TrackRow track={track} messages={messages} onPlay={onPlayTrack} />
                <Text dataSet={{ trackAlbumHint: '1' }}>
                  {`${album.title} · ${formatDuration(track.durationMs)}`}
                </Text>
              </View>
            )),
          )}
        </View>
      ) : null}
    </View>
  );
}

type TabButtonProps = {
  id: string;
  label: string;
  selected: boolean;
  onSelect: () => void;
};

function TabButton({ id, label, selected, onSelect }: TabButtonProps) {
  return (
    <View
      id={id}
      accessibilityRole="tab"
      accessibilityLabel={label}
      accessibilityState={{ selected }}
      dataSet={{ selected: selected ? '1' : '0' }}
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect();
        }
      }}
    >
      <Text>{label}</Text>
    </View>
  );
}
