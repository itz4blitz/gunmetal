import { useState } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { artistInitial } from '../format.ts';
import type { ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { TrackRow } from './TrackRow.tsx';

export type LibraryTab = 'albums' | 'artists' | 'tracks';

export type LibraryProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  /** Track the shell is playing now; its row paints the brass current state. */
  currentTrackId?: string | undefined;
  onOpenAlbum: (albumId: string) => void;
  onOpenArtist: (artistKey: string) => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
  onPlayNextAlbum?: ((albumId: string) => void) | undefined;
  onAddAlbumToQueue?: ((albumId: string) => void) | undefined;
  onPlayNextTrack?: ((albumId: string, trackId: string) => void) | undefined;
  onAddTrackToQueue?: ((albumId: string, trackId: string) => void) | undefined;
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

function trackTotal(library: ShellLibrary): number {
  return library.albums.reduce((total, album) => total + album.tracks.length, 0);
}

/** Whatever catalogue arrives, the tabs count it — never a hard-coded figure. */
function tabCount(tab: LibraryTab, library: ShellLibrary): number {
  if (tab === 'albums') {
    return library.albums.length;
  }
  if (tab === 'artists') {
    return library.artists.length;
  }
  return trackTotal(library);
}

export function Library({
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
}: LibraryProps) {
  const [tab, setTab] = useState<LibraryTab>('albums');
  const tabs: readonly LibraryTab[] = ['albums', 'artists', 'tracks'];
  const tabLabels: Record<LibraryTab, string> = {
    albums: messages.tabAlbums,
    artists: messages.tabArtists,
    tracks: messages.tabTracks,
  };

  return (
    <View id="destination-library">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.libraryHeadline}
      </Text>
      <View id="library-tabs" accessibilityRole="tablist" accessibilityLabel={messages.libraryHeadline}>
        {tabs.map((entry) => (
          <TabButton
            key={entry}
            id={`library-tab-${entry}`}
            label={tabLabels[entry]}
            count={tabCount(entry, library)}
            selected={tab === entry}
            onSelect={() => {
              setTab(entry);
            }}
          />
        ))}
      </View>
      {tab === 'albums' ? (
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
      ) : null}
      {tab === 'artists' ? (
        <View id="library-artist-list">
          {library.artists.map((artist) => {
            const rowName = artistRowName(artist.name, artist.albumIds, library, messages);
            const photo = artist.imageUrl === undefined || artist.imageUrl === '' ? null : artist.imageUrl;
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
                  {photo === null ? (
                    <Text dataSet={{ artistInitial: '1' }}>{artistInitial(rowName)}</Text>
                  ) : (
                    <View dataSet={{ artistPhoto: '1' }} style={{ backgroundImage: `url("${photo}")` }} />
                  )}
                </View>
                <View dataSet={{ artistMeta: '1' }}>
                  <Text dataSet={{ artistName: '1' }}>{rowName}</Text>
                </View>
                <Text dataSet={{ artistCount: '1' }}>{`${artist.albumIds.length} ${messages.artistAlbumCount}`}</Text>
              </View>
            );
          })}
        </View>
      ) : null}
      {tab === 'tracks' ? (
        <View id="library-track-list">
          <View dataSet={{ trackTableHead: '1' }} aria-hidden="true">
            <Text dataSet={{ trackHeadNumber: '1' }}>#</Text>
            <Text dataSet={{ trackHeadTitle: '1' }}>{messages.columnTitle}</Text>
            <Text dataSet={{ trackHeadAlbum: '1' }}>{messages.columnAlbum}</Text>
            <Text dataSet={{ trackHeadTime: '1' }}>{messages.columnTime}</Text>
          </View>
          {library.albums.flatMap((album) =>
            album.tracks.map((track) => (
              <TrackRow
                key={track.id}
                track={track}
                messages={messages}
                artistKey={album.artistKey}
                albumTitle={album.title}
                hostile={album.hostile}
                current={track.id === currentTrackId}
                onPlay={onPlayTrack}
                onPlayNext={onPlayNextTrack}
                onAddToQueue={onAddTrackToQueue}
                onGoToAlbum={onOpenAlbum}
                onOpenArtist={onOpenArtist}
              />
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
  count: number;
  selected: boolean;
  onSelect: () => void;
};

function TabButton({ id, label, count, selected, onSelect }: TabButtonProps) {
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
      <Text dataSet={{ tabLabel: '1' }}>{label}</Text>
      {/* The count is chrome; the accessible name stays the bare tab label. */}
      <Text dataSet={{ tabCount: '1' }} aria-hidden="true">
        {`${count}`}
      </Text>
    </View>
  );
}
