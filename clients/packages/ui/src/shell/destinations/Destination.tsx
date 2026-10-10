import { useMemo } from 'react';
import type { ReactNode } from 'react';
import { Text, View } from 'react-native-web';
import type { MessageCatalogue } from '../../messages/catalogue.ts';
import type { MatchResult } from '../../router/match.ts';
import type { ShellLibrary } from '../library-types.ts';
import type { LibrarySearch, LyricsResolver } from '../content.ts';
import {
  settingsSectionFromPath,
  settingsSectionPath,
  type PluginSlot,
  type SettingsCrossfadeSeconds,
  type SettingsLevelling,
  type SettingsSection,
} from './settings.ts';
import type { ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';
import { extensionIdFromPath } from '../../plugins/repository.ts';
import { labelFromKey } from '../../router/media-path.ts';
import { AlbumDetail } from './AlbumDetail.tsx';
import { ArtistDetail } from './ArtistDetail.tsx';
import { Home } from './Home.tsx';
import { albumsByArtistIndex, indexAlbums, indexArtists, otherAlbums } from './library-index.ts';
import { Library } from './Library.tsx';
import { Search } from './Search.tsx';
import { Settings } from './Settings.tsx';
import { Store } from './Store.tsx';

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
  onOpenPath?: ((path: string) => void) | undefined;
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
  /** A folder library is served by this origin. Search drops the demo-local notice. */
  served?: boolean | undefined;
  /** About-page library size. Set only when the library is a folder. */
  libraryFact?: string | undefined;
  /** Libraries content for the settings page. Absent, the section stays hidden. */
  libraries?: ReactNode | undefined;
  width: WidthClass;
  onPlayNextAlbum?: (albumId: string) => void;
  onAddAlbumToQueue?: (albumId: string) => void;
  onPlayNextTrack?: (albumId: string, trackId: string) => void;
  onAddTrackToQueue?: (albumId: string, trackId: string) => void;
  levelling?: SettingsLevelling | undefined;
  onLevelling?: ((levelling: SettingsLevelling) => void) | undefined;
  crossfadeSeconds?: SettingsCrossfadeSeconds | undefined;
  onCrossfade?: ((seconds: SettingsCrossfadeSeconds) => void) | undefined;
  outputs?: readonly { id: string; label: string }[] | undefined;
  sinkId?: string | undefined;
  onOutput?: ((id: string) => void) | undefined;
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
  onOpenPath,
  currentTrackId,
  playingAlbumId,
  nearViewObserver,
  libraryError,
  onLibraryRetry,
  served,
  libraryFact,
  libraries,
  width,
  onPlayNextAlbum,
  onAddAlbumToQueue,
  onPlayNextTrack,
  onAddTrackToQueue,
  levelling,
  onLevelling,
  crossfadeSeconds,
  onCrossfade,
  outputs,
  sinkId,
  onOutput,
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

  if (match.route.path === '/store') {
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <Store messages={messages.destinations} onOpenPath={onOpenPath} />
      </View>
    );
  }

  if (match.media !== undefined && itemId === undefined) {
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1', mediaKind: match.media.kind }}>
        <Text id="destination-headline" accessibilityRole="header">
          {labelFromKey(match.media.key)}
        </Text>
        {library === undefined ? null : (
          <Text dataSet={{ mediaEmpty: match.media.kind }}>{mediaEmpty(match.media.kind, messages)}</Text>
        )}
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
          served={served}
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
        extensionId={extensionIdFromPath(match.route.path)}
        onOpenPath={onOpenPath}
        pluginSlots={pluginSlots}
        libraryFact={libraryFact}
        libraries={libraries}
        messages={messages.destinations}
        shellMessages={messages.shell}
        theme={theme}
        onThemeChange={onThemeChange}
        levelling={levelling}
        onLevelling={onLevelling}
        crossfadeSeconds={crossfadeSeconds}
        onCrossfade={onCrossfade}
        outputs={outputs}
        sinkId={sinkId}
        onOutput={onOutput}
        width={width}
      />
    </View>
  );
}

function mediaEmpty(kind: 'artist' | 'album' | 'track' | 'movie' | 'show', messages: MessageCatalogue): string {
  if (kind === 'movie') {
    return messages.destinations.mediaMovieEmpty;
  }
  if (kind === 'show') {
    return messages.destinations.mediaShowEmpty;
  }
  if (kind === 'artist') {
    return messages.destinations.mediaArtistEmpty;
  }
  if (kind === 'track') {
    return messages.destinations.mediaTrackEmpty;
  }
  return messages.destinations.mediaAlbumEmpty;
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
