import { useState } from 'react';
import type { KeyboardEvent } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { artistInitial, countNoun, staggerSlot } from '../format.ts';
import { Icon, type IconName } from '../Icon.tsx';
import type { ShellArtist, ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { TrackRow } from './TrackRow.tsx';

export type LibraryTab = 'albums' | 'artists' | 'tracks';

/** Comfortable is the survey default; compact tightens the table to 44px rows. */
export type LibraryDensity = 'comfortable' | 'compact';

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

/**
 * The name an artist is shown under. An artist of a hostile fixture album is
 * never shown by its own text: the safe catalogue label stands in.
 */
export function artistDisplayName(artist: ShellArtist, library: ShellLibrary, messages: DestinationMessages): string {
  const hostile = artist.albumIds.some((id) => library.albums.some((album) => album.id === id && album.hostile));
  if (hostile) {
    return messages.hostileArtistLabel;
  }
  return artist.name;
}

/** People are nuts: the artist's photo in the nut, or the initial when there is none. */
export function ArtistAvatar({ name, imageUrl }: { name: string; imageUrl: string | undefined }) {
  const photo = imageUrl === undefined || imageUrl === '' ? null : imageUrl;
  return (
    <View dataSet={{ artistAvatar: '1' }}>
      {photo === null ? (
        <Text dataSet={{ artistInitial: '1' }}>{artistInitial(name)}</Text>
      ) : (
        <View dataSet={{ artistPhoto: '1' }} style={{ backgroundImage: `url("${photo}")` }} />
      )}
    </View>
  );
}

function trackTotal(library: ShellLibrary): number {
  return library.albums.reduce((total, album) => total + album.tracks.length, 0);
}

/** Whatever catalogue arrives, the header counts it — never a hard-coded figure. */
function totalsLine(library: ShellLibrary, messages: DestinationMessages): string {
  return [
    countNoun(library.albums.length, messages.albumCountOne, messages.artistAlbumCount),
    countNoun(library.artists.length, messages.artistCountOne, messages.artistCountLabel),
    countNoun(trackTotal(library), messages.trackCountOne, messages.trackCountLabel),
  ].join(' · ');
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

const TABS: readonly LibraryTab[] = ['albums', 'artists', 'tracks'];

/** Each tab's neighbours in the tab bar, wrapping at the ends. */
const TAB_NEIGHBOURS: Record<LibraryTab, { before: LibraryTab; after: LibraryTab }> = {
  albums: { before: 'tracks', after: 'artists' },
  artists: { before: 'albums', after: 'tracks' },
  tracks: { before: 'artists', after: 'albums' },
};

/** The tab a tablist key moves to; any other key answers undefined. */
function tabForKey(current: LibraryTab, key: string): LibraryTab | undefined {
  if (key === 'ArrowRight') {
    return TAB_NEIGHBOURS[current].after;
  }
  if (key === 'ArrowLeft') {
    return TAB_NEIGHBOURS[current].before;
  }
  if (key === 'Home') {
    return 'albums';
  }
  if (key === 'End') {
    return 'tracks';
  }
  return undefined;
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
  const [density, setDensity] = useState<LibraryDensity>('comfortable');
  const tabLabels: Record<LibraryTab, string> = {
    albums: messages.tabAlbums,
    artists: messages.tabArtists,
    tracks: messages.tabTracks,
  };
  const emptyLabels: Record<LibraryTab, string> = {
    albums: messages.libraryEmptyAlbums,
    artists: messages.libraryEmptyArtists,
    tracks: messages.libraryEmptyTracks,
  };
  // A tab with nothing in it says so; it never draws an empty grid or a
  // table head over no rows.
  const view: LibraryTab | 'empty' = tabCount(tab, library) === 0 ? 'empty' : tab;
  const selectAndFocus = (entry: LibraryTab) => {
    setTab(entry);
    document.getElementById(`library-tab-${entry}`)?.focus();
  };
  // Roving tabindex with automatic activation: the arrows move both focus and
  // the selected section, wrapping at the ends (design-language §8).
  const onTablistKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const target = tabForKey(tab, event.key);
    if (target === undefined) {
      return;
    }
    event.preventDefault();
    selectAndFocus(target);
  };

  return (
    <View id="destination-library" dataSet={{ density }}>
      <View id="library-header">
        <View dataSet={{ libraryHeading: '1' }}>
          <Text id="destination-headline" accessibilityRole="header">
            {messages.libraryHeadline}
          </Text>
          <Text id="library-totals" dataSet={{ libraryTotals: '1' }}>
            {totalsLine(library, messages)}
          </Text>
        </View>
        <View id="library-density" accessibilityRole="group" accessibilityLabel={messages.densityLabel}>
          <DensityButton
            id="library-density-comfortable"
            icon="rows"
            label={messages.densityComfortable}
            selected={density === 'comfortable'}
            onSelect={() => {
              setDensity('comfortable');
            }}
          />
          <DensityButton
            id="library-density-compact"
            icon="rowsDense"
            label={messages.densityCompact}
            selected={density === 'compact'}
            onSelect={() => {
              setDensity('compact');
            }}
          />
        </View>
      </View>
      <View
        id="library-tabs"
        accessibilityRole="tablist"
        accessibilityLabel={messages.libraryHeadline}
        onKeyDown={onTablistKeyDown}
      >
        {TABS.map((entry) => (
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
      {view === 'empty' ? (
        <View id="library-empty" dataSet={{ libraryEmpty: tab }}>
          <View dataSet={{ libraryEmptyMark: '1' }}>
            <Icon name="library" size={22} />
          </View>
          <Text dataSet={{ libraryEmptyText: '1' }}>{emptyLabels[tab]}</Text>
        </View>
      ) : null}
      {view === 'albums' ? (
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
      {view === 'artists' ? (
        <View id="library-artist-list">
          {library.artists.map((artist, index) => {
            const rowName = artistDisplayName(artist, library, messages);
            const firstAlbumId = artist.albumIds[0];
            return (
              <View
                key={artist.key}
                id={`artist-row-${artist.key}`}
                dataSet={{ artistRow: artist.key, rowStagger: staggerSlot(index) }}
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
                <ArtistAvatar name={rowName} imageUrl={artist.imageUrl} />
                <View dataSet={{ artistMeta: '1' }}>
                  <Text dataSet={{ artistName: '1' }}>{rowName}</Text>
                </View>
                <Text dataSet={{ artistCount: '1' }}>
                  {countNoun(artist.albumIds.length, messages.albumCountOne, messages.artistAlbumCount)}
                </Text>
                {firstAlbumId === undefined ? null : (
                  <View
                    dataSet={{ artistPlay: '1' }}
                    accessibilityRole="button"
                    accessibilityLabel={`${messages.play} ${rowName}`}
                    tabIndex={0}
                    onClick={(event) => {
                      event.stopPropagation();
                      onPlayAlbum(firstAlbumId);
                    }}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault();
                        event.stopPropagation();
                        onPlayAlbum(firstAlbumId);
                      }
                    }}
                  >
                    <Text>{messages.play}</Text>
                  </View>
                )}
              </View>
            );
          })}
        </View>
      ) : null}
      {view === 'tracks' ? (
        <View id="library-track-list">
          <View dataSet={{ trackTableHead: '1' }} aria-hidden="true">
            <Text dataSet={{ trackHeadNumber: '1' }}>#</Text>
            <Text dataSet={{ trackHeadTitle: '1' }}>{messages.columnTitle}</Text>
            <Text dataSet={{ trackHeadAlbum: '1' }}>{messages.columnAlbum}</Text>
            <Text dataSet={{ trackHeadTime: '1' }}>{messages.columnTime}</Text>
          </View>
          {library.albums
            .flatMap((album) => album.tracks.map((track) => ({ album, track })))
            .map(({ album, track }, index) => (
              <TrackRow
                key={track.id}
                track={track}
                messages={messages}
                artistKey={album.artistKey}
                albumTitle={album.title}
                hostile={album.hostile}
                current={track.id === currentTrackId}
                staggerIndex={index}
                onPlay={onPlayTrack}
                onPlayNext={onPlayNextTrack}
                onAddToQueue={onAddTrackToQueue}
                onGoToAlbum={onOpenAlbum}
                onOpenArtist={onOpenArtist}
              />
            ))}
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
      aria-selected={selected}
      dataSet={{ selected: selected ? '1' : '0' }}
      // Roving tabindex: the selected tab is the only tab stop.
      tabIndex={selected ? 0 : -1}
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

type DensityButtonProps = {
  id: string;
  icon: IconName;
  label: string;
  selected: boolean;
  onSelect: () => void;
};

function DensityButton({ id, icon, label, selected, onSelect }: DensityButtonProps) {
  return (
    <View
      id={id}
      accessibilityRole="button"
      accessibilityLabel={label}
      aria-pressed={selected}
      dataSet={{ densityOption: '1', selected: selected ? '1' : '0' }}
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect();
        }
      }}
    >
      <Icon name={icon} size={16} />
      <Text dataSet={{ densityLabel: '1' }}>{label}</Text>
    </View>
  );
}
