import { useMemo } from 'react';
import { Text, View } from 'react-native-web';
import type { MessageCatalogue } from '../../messages/catalogue.ts';
import type { MatchResult } from '../../router/match.ts';
import type { ShellLibrary } from '../library-types.ts';
import type { LibrarySearch, LyricsResolver } from '../content.ts';
import { settingsSectionFromPath, type PluginSlot, type SettingsSection } from './settings.ts';
import type { ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';
import { AlbumDetail } from './AlbumDetail.tsx';
import { ArtistDetail } from './ArtistDetail.tsx';
import { Home } from './Home.tsx';
import { albumsByArtistIndex, indexAlbums, indexArtists, otherAlbums } from './library-index.ts';
import { Library } from './Library.tsx';
import { Search } from './Search.tsx';
import { Settings } from './Settings.tsx';

export type DestinationProps = {
  searchLibrary: LibrarySearch;
  lyricsFor: LyricsResolver;
  pluginSlots: readonly PluginSlot[];
  /** Settings section from the route, when one arrives. The same settings page, not a second app. */
  section?: SettingsSection | undefined;
  match: MatchResult;
  messages: MessageCatalogue;
  library: ShellLibrary | undefined;
  itemId: string | undefined;
  theme: ThemeId;
  onThemeChange: (theme: ThemeId) => void;
  onOpenAlbum: (albumId: string) => void;
  onOpenArtist: (artistKey: string) => void;
  onBackFromAlbum: () => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
  onSeeAll: () => void;
  currentTrackId?: string | undefined;
  playingAlbumId?: string | undefined;
  /**
   * The browsing grids (Library albums, ArtistDetail albums) defer cover
   * URLs until tiles are near the viewport; the browser constructor by
   * default, undefined in jsdom — where everything paints immediately.
   */
  nearViewObserver?: typeof IntersectionObserver | undefined;
  /** A typed failure of the library read; the Library page says it honestly. */
  libraryError?: string | undefined;
  /** The library read's one fixing action, wired straight through. */
  onLibraryRetry?: (() => void) | undefined;
  width: WidthClass;
  onPlayNextAlbum?: (albumId: string) => void;
  onAddAlbumToQueue?: (albumId: string) => void;
  onPlayNextTrack?: (albumId: string, trackId: string) => void;
  onAddTrackToQueue?: (albumId: string, trackId: string) => void;
};

function pageKey(match: MatchResult, itemId: string | undefined): string {
  if (match.kind === 'not-found') {
    return 'not-found';
  }
  return `${match.route.path}:${itemId ?? ''}`;
}

export function Destination({
  searchLibrary,
  lyricsFor,
  pluginSlots,
  section,
  match,
  messages,
  library,
  itemId,
  theme,
  onThemeChange,
  onOpenAlbum,
  onOpenArtist,
  onBackFromAlbum,
  onPlayAlbum,
  onPlayTrack,
  onSeeAll,
  currentTrackId,
  playingAlbumId,
  nearViewObserver,
  libraryError,
  onLibraryRetry,
  width,
  onPlayNextAlbum,
  onAddAlbumToQueue,
  onPlayNextTrack,
  onAddTrackToQueue,
}: DestinationProps) {
  const enterKey = pageKey(match, itemId);
  // One O(n) index build per library change, shared by every page render —
  // playback ticks re-render the shell several times a second.
  const albums = useMemo(() => indexAlbums(library ?? { albums: [], artists: [] }), [library]);
  const artists = useMemo(() => indexArtists(library ?? { albums: [], artists: [] }), [library]);
  const byArtist = useMemo(() => albumsByArtistIndex(library ?? { albums: [], artists: [] }), [library]);

  if (match.kind === 'not-found') {
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <Text id="destination-headline" accessibilityRole="header">
          {messages.destinations.notFoundHeadline}
        </Text>
      </View>
    );
  }

  if (library !== undefined && itemId !== undefined) {
    const artist = artists.get(itemId);
    if (artist !== undefined) {
      return (
        <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
          <ArtistDetail
            artist={artist}
            library={library}
            messages={messages.destinations}
            onBack={onBackFromAlbum}
            onOpenAlbum={onOpenAlbum}
            onPlayAlbum={onPlayAlbum}
            onOpenArtist={onOpenArtist}
            onPlayNextAlbum={onPlayNextAlbum}
            onAddAlbumToQueue={onAddAlbumToQueue}
            onPlayTrack={onPlayTrack}
            currentTrackId={currentTrackId}
            nearViewObserver={nearViewObserver}
          />
        </View>
      );
    }
    const album = albums.get(itemId);
    const others = otherAlbums(byArtist, album);
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <AlbumDetail
          lyricsFor={lyricsFor}
          album={album}
          moreBy={others.length === 0 ? undefined : { albums: others, onOpenAlbum }}
          onPlayNextAlbum={onPlayNextAlbum}
          onAddAlbumToQueue={onAddAlbumToQueue}
          messages={messages.destinations}
          currentTrackId={currentTrackId}
          onBack={onBackFromAlbum}
          onPlayAlbum={onPlayAlbum}
          onPlayTrack={onPlayTrack}
          onOpenArtist={onOpenArtist}
          onPlayNextTrack={onPlayNextTrack}
          onAddTrackToQueue={onAddTrackToQueue}
        />
      </View>
    );
  }

  if (library === undefined) {
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <Text id="destination-headline" accessibilityRole="header">
          {headlineForEmpty(match, messages)}
        </Text>
      </View>
    );
  }

  if (match.route.path === '/') {
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <Home
          messages={messages.destinations}
          library={library}
          onOpenAlbum={onOpenAlbum}
          onOpenArtist={onOpenArtist}
          onPlayAlbum={onPlayAlbum}
          onPlayNextAlbum={onPlayNextAlbum}
          onAddAlbumToQueue={onAddAlbumToQueue}
          onSeeAll={onSeeAll}
          playingAlbumId={playingAlbumId}
        />
      </View>
    );
  }
  if (match.route.path === '/search') {
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <Search
          searchLibrary={searchLibrary}
          messages={messages.destinations}
          library={library}
          currentTrackId={currentTrackId}
          onOpenAlbum={onOpenAlbum}
          onOpenArtist={onOpenArtist}
          onPlayAlbum={onPlayAlbum}
          onPlayTrack={onPlayTrack}
          onPlayNextAlbum={onPlayNextAlbum}
          onAddAlbumToQueue={onAddAlbumToQueue}
          onPlayNextTrack={onPlayNextTrack}
          onAddTrackToQueue={onAddTrackToQueue}
        />
      </View>
    );
  }
  if (match.route.path === '/library') {
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <Library
          messages={messages.destinations}
          library={library}
          currentTrackId={currentTrackId}
          nearViewObserver={nearViewObserver}
          error={libraryError}
          onRetry={onLibraryRetry}
          onOpenAlbum={onOpenAlbum}
          onOpenArtist={onOpenArtist}
          onPlayAlbum={onPlayAlbum}
          onPlayTrack={onPlayTrack}
          onPlayNextAlbum={onPlayNextAlbum}
          onAddAlbumToQueue={onAddAlbumToQueue}
          onPlayNextTrack={onPlayNextTrack}
          onAddTrackToQueue={onAddTrackToQueue}
        />
      </View>
    );
  }
  return (
    <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
      <Settings
        section={section ?? settingsSectionFromPath(match.route.path)}
        pluginSlots={pluginSlots}
        messages={messages.destinations}
        shellMessages={messages.shell}
        theme={theme}
        onThemeChange={onThemeChange}
        width={width}
      />
    </View>
  );
}

function headlineForEmpty(match: Extract<MatchResult, { kind: 'ok' }>, messages: MessageCatalogue): string {
  if (match.route.path === '/') {
    return messages.destinations.homeHeadline;
  }
  if (match.route.path === '/search') {
    return messages.destinations.searchHeadline;
  }
  if (match.route.path === '/library') {
    return messages.destinations.libraryHeadline;
  }
  return messages.destinations.settingsHeadline;
}
