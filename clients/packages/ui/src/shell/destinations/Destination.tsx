import { Text, View } from 'react-native-web';
import type { MessageCatalogue } from '../../messages/catalogue.ts';
import type { MatchResult } from '../../router/match.ts';
import type { ShellLibrary } from '../library-types.ts';
import type { ThemeId } from '../theme.ts';
import { findAlbum } from '../playback.ts';
import { AlbumDetail } from './AlbumDetail.tsx';
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
  onBackFromAlbum: () => void;
  onPlayAlbum: (albumId: string) => void;
  onPlayTrack: (albumId: string, trackId: string) => void;
  onSeeAll: () => void;
  currentTrackId?: string;
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
  onBackFromAlbum,
  onPlayAlbum,
  onPlayTrack,
  onSeeAll,
  currentTrackId,
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
    return (
      <View id="destination" key={enterKey} dataSet={{ pageEnter: '1' }}>
        <AlbumDetail
          album={findAlbum(library, itemId)}
          messages={messages.destinations}
          currentTrackId={currentTrackId}
          onBack={onBackFromAlbum}
          onPlayAlbum={onPlayAlbum}
          onPlayTrack={onPlayTrack}
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
          onPlayAlbum={onPlayAlbum}
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
          onPlayAlbum={onPlayAlbum}
          onPlayTrack={onPlayTrack}
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
          onPlayAlbum={onPlayAlbum}
          onPlayTrack={onPlayTrack}
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
