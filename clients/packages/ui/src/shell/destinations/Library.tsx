import { useState } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { artistInitial, formatDuration } from '../format.ts';
import type { ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { TrackRow } from './TrackRow.tsx';

export type LibraryTab = 'albums' | 'artists' | 'tracks';

export type LibraryProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  onOpenAlbum: (albumId: string) => void;
  onOpenArtist: (artistKey: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
  onPlayNextAlbum?: (albumId: string) => void;
  onAddAlbumToQueue?: (albumId: string) => void;
  onPlayNextTrack?: (albumId: string, trackId: string) => void;
  onAddTrackToQueue?: (albumId: string, trackId: string) => void;
};

function artistRowName(
  name: string,
  albumIds: readonly string[],
  library: ShellLibrary,
  messages: DestinationMessages,
): string {
  const hostile = albumIds.some((id) => library.albums.some((album) => album.id === id && album.hostile));
  if (hostile) {
    return messages.hostileArtistLabel;
  }
  return name;
}

export function Library({
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
            {library.albums.map((album, index) => (
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
      ) : null}
      {tab === 'artists' ? (
        <View id="library-artist-list">
          {library.artists.map((artist) => {
            const rowName = artistRowName(artist.name, artist.albumIds, library, messages);
            return (
              <View
                key={artist.key}
                id={`artist-row-${artist.key}`}
                dataSet={{ artistRow: artist.key }}
                accessibilityRole="button"
                accessibilityLabel={rowName}
                tabIndex={0}
                onClick={() => {
                  onOpenArtist(artist.key);
                }}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    onOpenArtist(artist.key);
                  }
                }}
              >
                <View dataSet={{ artistAvatar: '1' }} aria-hidden="true">
                  <Text dataSet={{ artistInitial: '1' }}>{artistInitial(rowName)}</Text>
                </View>
                <Text dataSet={{ artistName: '1' }}>{rowName}</Text>
                <Text dataSet={{ artistCount: '1' }}>
                  {`${artist.albumIds.length} ${messages.artistAlbumCount}`}
                </Text>
              </View>
            );
          })}
        </View>
      ) : null}
      {tab === 'tracks' ? (
        <View id="library-track-list">
          {library.albums.flatMap((album) =>
            album.tracks.map((track) => (
              <View key={track.id} dataSet={{ libraryTrack: track.id }}>
                <TrackRow
                  track={track}
                  messages={messages}
                  artistKey={album.artistKey}
                  onPlay={onPlayTrack}
                  onPlayNext={onPlayNextTrack}
                  onAddToQueue={onAddTrackToQueue}
                  onGoToAlbum={onOpenAlbum}
                  onOpenArtist={onOpenArtist}
                />
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
