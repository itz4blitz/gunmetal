import { useMemo, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { artistInitial, countNoun, staggerSlot } from '../format.ts';
import { Icon, type IconName } from '../Icon.tsx';
import type { ShellArtist, ShellLibrary } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';
import { hostileArtistKeys } from './library-index.ts';
import {
  nextSortOption,
  sortAlbums,
  sortArtists,
  sortTracks,
  type AlbumSort,
  type ArtistSort,
  type TrackSort,
} from './library-sort.ts';
import { TrackRow } from './TrackRow.tsx';
import { intersectionObserverFactory } from './near-view.ts';
import { LIST_OVERSCAN_ROWS, contentScroller, useContentViewport, visibleRange, spacerHeights } from './windowing.ts';

export type LibraryTab = 'albums' | 'artists' | 'tracks';

/** Comfortable is the survey default; compact tightens the table to 44px rows. */
export type LibraryDensity = 'comfortable' | 'compact';

/** The track table's row heights, pinned by the density contract in area-library.css. */
const TRACK_ROW_HEIGHTS: Record<LibraryDensity, number> = { comfortable: 52, compact: 44 };

/** Each tab's sort choice, remembered across tab switches. */
export type LibrarySorts = { albums: AlbumSort; artists: ArtistSort; tracks: TrackSort };

const DEFAULT_SORTS: LibrarySorts = { albums: 'recent', artists: 'default', tracks: 'default' };

/** The sort choices a tab offers, keyed to their catalogue labels. */
function sortOptionsFor(
  tab: LibraryTab,
  messages: DestinationMessages,
): ReadonlyArray<{ key: AlbumSort | ArtistSort | TrackSort; label: string }> {
  if (tab === 'albums') {
    return [
      { key: 'recent', label: messages.libSortRecent },
      { key: 'title', label: messages.libSortTitle },
      { key: 'artist', label: messages.libSortArtist },
      { key: 'year', label: messages.libSortYear },
    ];
  }
  if (tab === 'artists') {
    return [
      { key: 'default', label: messages.libSortDefault },
      { key: 'name', label: messages.libSortName },
      { key: 'albums', label: messages.libSortAlbumCount },
    ];
  }
  return [
    { key: 'default', label: messages.libSortDefault },
    { key: 'title', label: messages.libSortTitle },
    { key: 'duration', label: messages.libSortDuration },
  ];
}

export type LibraryProps = {
  messages: DestinationMessages;
  library: ShellLibrary;
  /** Track the shell is playing now; its row paints the brass current state. */
  currentTrackId?: string | undefined;
  /**
   * The scroll container the track table windows against; the shell's
   * content pane by default. Tests inject a stand-in (jsdom has no layout).
   */
  getScroller?: (() => HTMLElement | null) | undefined;
  /**
   * The grid defers cover URLs until tiles are near the viewport. The
   * browser constructor by default; undefined (jsdom) paints immediately.
   */
  nearViewObserver?: typeof IntersectionObserver | undefined;
  /** A typed failure of the library read, said instead of a half-working UI. */
  error?: string | undefined;
  /** The one fixing action; without it the statement offers nothing. */
  onRetry?: (() => void) | undefined;
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
 * never shown by its own text: the safe catalogue label stands in. The
 * hostile keys come precomputed (one set per library, not per row).
 */
export function artistDisplayName(
  artist: ShellArtist,
  hostileKeys: ReadonlySet<string>,
  messages: DestinationMessages,
): string {
  if (hostileKeys.has(artist.key)) {
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
  getScroller,
  nearViewObserver,
  error,
  onRetry,
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
  const [sorts, setSorts] = useState<LibrarySorts>(DEFAULT_SORTS);
  // One hostile-key set per library change; every artist row reads it in O(1).
  const hostileKeys = useMemo(() => hostileArtistKeys(library), [library]);
  // The flattened track table, once per library change.
  const allTracks = useMemo(
    () => library.albums.flatMap((album) => album.tracks.map((track) => ({ album, track }))),
    [library],
  );
  // The rows the content pane can see; a zero read (jsdom) shows everything.
  const viewport = useContentViewport(getScroller ?? contentScroller);
  // Undefined in jsdom: no factory, every cover paints immediately.
  const nearArt = useMemo(
    () => intersectionObserverFactory(nearViewObserver ?? globalThis.IntersectionObserver),
    [nearViewObserver],
  );
  // The tab's order: sorted copies feed the grid, the list and the window.
  const sortOptions = sortOptionsFor(tab, messages);
  const currentSort = sorts[tab];
  const shownAlbums = useMemo(() => sortAlbums(library.albums, sorts.albums), [library.albums, sorts.albums]);
  const shownArtists = useMemo(() => sortArtists(library.artists, sorts.artists), [library.artists, sorts.artists]);
  const sortedTracks = useMemo(() => sortTracks(allTracks, sorts.tracks), [allTracks, sorts.tracks]);
  const trackWindow = visibleRange(
    viewport.scrollTop,
    viewport.viewportHeight,
    TRACK_ROW_HEIGHTS[density],
    sortedTracks.length,
    LIST_OVERSCAN_ROWS,
  );
  const shownTracks = sortedTracks.slice(trackWindow.start, trackWindow.end);
  /* The rows the window leaves out must still take up their space, or the
     list shrinks to the window and the tail can never be scrolled to. */
  const trackSpacers = spacerHeights(trackWindow, sortedTracks.length, TRACK_ROW_HEIGHTS[density]);
  const chooseSort = (key: AlbumSort | ArtistSort | TrackSort) => {
    setSorts((prev) => ({ ...prev, [tab]: key }) as LibrarySorts);
  };
  // Arrows move the checked option and the focus together, wrapping at the
  // ends — the Settings radiogroup's automatic model.
  const onSortKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const next = nextSortOption(sortOptions, sorts[tab as LibraryTab], event.key);
    if (next === undefined) {
      return;
    }
    event.preventDefault();
    chooseSort(next.key as AlbumSort | ArtistSort | TrackSort);
    document.getElementById(`library-sort-${tab}-${next.key}`)?.focus();
  };
  // A failed read says so and offers its one fixing action; nothing else
  // pretends to work over data the surface cannot see.
  if (error !== undefined) {
    return (
      <View id="destination-library" dataSet={{ density }}>
        <View id="library-error" accessibilityRole="alert">
          <Text id="destination-headline" accessibilityRole="header">
            {messages.libErrorHeadline}
          </Text>
          <Text dataSet={{ libraryErrorText: '1' }}>{error}</Text>
          {onRetry === undefined ? null : (
            <View
              id="library-retry"
              accessibilityRole="button"
              accessibilityLabel={messages.libErrorRetry}
              tabIndex={0}
              onClick={onRetry}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  onRetry();
                }
              }}
            >
              <Text>{messages.libErrorRetry}</Text>
            </View>
          )}
        </View>
      </View>
    );
  }
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
        <SortControl
          tab={tab}
          label={messages.libSortLabel}
          options={sortOptions}
          current={currentSort}
          onChoose={chooseSort}
          onKeyDown={onSortKeyDown}
        />
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
          {shownAlbums.map((album, index) => (
            <AlbumTile
              key={album.id}
              album={album}
              messages={messages}
              staggerIndex={index}
              nearArt={nearArt}
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
          {shownArtists.map((artist, index) => {
            const rowName = artistDisplayName(artist, hostileKeys, messages);
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
          <View aria-hidden="true" dataSet={{ listSpacer: 'top' }} style={{ height: trackSpacers.top }} />
          {shownTracks.map(({ album, track }, index) => (
            <TrackRow
              key={track.id}
              track={track}
              messages={messages}
              artistKey={album.artistKey}
              albumTitle={album.title}
              hostile={album.hostile}
              current={track.id === currentTrackId}
              staggerIndex={trackWindow.start + index}
              onPlay={onPlayTrack}
              onPlayNext={onPlayNextTrack}
              onAddToQueue={onAddTrackToQueue}
              onGoToAlbum={onOpenAlbum}
              onOpenArtist={onOpenArtist}
            />
          ))}
          <View aria-hidden="true" dataSet={{ listSpacer: 'bottom' }} style={{ height: trackSpacers.bottom }} />
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

type SortControlProps = {
  tab: LibraryTab;
  label: string;
  options: ReadonlyArray<{ key: AlbumSort | ArtistSort | TrackSort; label: string }>;
  current: AlbumSort | ArtistSort | TrackSort;
  onChoose: (key: AlbumSort | ArtistSort | TrackSort) => void;
  onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
};

/** One sort choice: a quiet radio in the toolbar, checked where you are. */
function SortOption({
  id,
  label,
  selected,
  onChoose,
}: {
  id: string;
  label: string;
  selected: boolean;
  onChoose: () => void;
}) {
  return (
    <View
      id={id}
      accessibilityRole="radio"
      accessibilityLabel={label}
      aria-checked={selected}
      dataSet={{ sortOption: '1', selected: selected ? '1' : '0' }}
      // Roving tabindex: the checked option is the group's one tab stop.
      tabIndex={selected ? 0 : -1}
      onClick={onChoose}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onChoose();
        }
      }}
    >
      <Text dataSet={{ sortLabel: '1' }}>{label}</Text>
    </View>
  );
}

function SortControl({ tab, label, options, current, onChoose, onKeyDown }: SortControlProps) {
  return (
    <View id="library-sort" accessibilityRole="radiogroup" accessibilityLabel={label} onKeyDown={onKeyDown}>
      {options.map((option) => (
        <SortOption
          key={option.key}
          id={`library-sort-${tab}-${option.key}`}
          label={option.label}
          selected={option.key === current}
          onChoose={() => {
            onChoose(option.key);
          }}
        />
      ))}
    </View>
  );
}
