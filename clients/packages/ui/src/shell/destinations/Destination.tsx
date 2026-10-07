import { Text, View } from 'react-native-web';
import type { MessageCatalogue } from '../../messages/catalogue.ts';
import type { MatchResult } from '../../router/match.ts';
import type { ShellLibrary } from '../library-types.ts';
import type { ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';
import { findAlbum, findArtist } from '../playback.ts';
import { AlbumDetail } from './AlbumDetail.tsx';
import { ArtistDetail } from './ArtistDetail.tsx';
import { Home } from './Home.tsx';
import { Library } from './Library.tsx';
import { Search } from './Search.tsx';
import { Settings } from './Settings.tsx';

export type DestinationProps = {
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
  currentTrackId?: string;
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
  width,
  onPlayNextAlbum,
  onAddAlbumToQueue,
  onPlayNextTrack,
  onAddTrackToQueue,
}: DestinationProps) {
  const enterKey = pageKey(match, itemId);

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
    const artist = findArtist(library, itemId);
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
          />
        </View>
      );
    }
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <AlbumDetail
          album={findAlbum(library, itemId)}
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
        />
      </View>
    );
  }
  if (match.route.path === '/search') {
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <Search
          messages={messages.destinations}
          library={library}
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
