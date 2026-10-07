import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import { catalogue } from '../messages/catalogue.ts';
import { matchAddress, type MatchResult } from '../router/match.ts';
import { pushPath } from '../router/navigate.ts';
import { noLyrics, type LibrarySearch, type LyricsResolver } from './content.ts';
import { Destination } from './destinations/Destination.tsx';
import type { PluginSlot } from './destinations/settings.ts';
import type { ShellLibrary } from './library-types.ts';
import { Nav, navItems } from './Nav.tsx';
import { PlayerBar } from './PlayerBar.tsx';
import { PlayerFull } from './PlayerFull.tsx';
import { QueuePane } from './QueuePane.tsx';
import type { PlaybackController } from './playback-controller.ts';
import { defaultTheme, type ThemeId } from './theme.ts';
import { landmarksForClass, widthClass, type WidthClass } from './width.ts';

export type ShellProps = {
  path?: string;
  search?: string;
  hash?: string;
  historyState?: unknown;
  widthPx?: number;
  theme?: ThemeId;
  showDemoLabel?: boolean;
  library?: ShellLibrary;
  /** The composition root's playback implementation (see PlaybackController). */
  playback: PlaybackController;
  /** Library search read (demo filter today, ServerPort search later). */
  searchLibrary?: LibrarySearch | undefined;
  /** Lyrics content resolver (fixture verses today, server lyrics later). */
  lyricsFor?: LyricsResolver | undefined;
  /** Plugin-slot list for the Settings page (server config later). */
  pluginSlots?: readonly PluginSlot[] | undefined;
  onNavigate?: (path: string) => void;
  onThemeChange?: (theme: ThemeId) => void;
};

function readWindowWidth(): number {
  return globalThis.innerWidth;
}

function readLocation(): { pathname: string; search: string; hash: string; state: unknown } {
  return {
    pathname: globalThis.location.pathname,
    search: globalThis.location.search,
    hash: globalThis.location.hash,
    state: globalThis.history.state,
  };
}

function detailPath(match: MatchResult): string {
  if (match.kind === 'ok' && (match.route.path === '/' || match.route.path === '/library')) {
    return match.route.path;
  }
  return '/library';
}

function focusLandmark(id: 'content' | 'player-bar'): void {
  const target = globalThis.document.getElementById(id);
  target?.focus();
}

export function Shell({
  path,
  search = '',
  hash = '',
  historyState = null,
  widthPx,
  theme: themeProp,
  showDemoLabel = false,
  library,
  playback,
  searchLibrary = () => ({ albums: [], tracks: [] }),
  lyricsFor = () => noLyrics,
  pluginSlots,
  onNavigate,
  onThemeChange,
}: ShellProps) {
  const messages = catalogue();
  const [theme, setTheme] = useState<ThemeId>(themeProp ?? defaultTheme());
  const [width, setWidth] = useState<WidthClass>(() => widthClass(widthPx ?? readWindowWidth()));
  const [location, setLocation] = useState(() => {
    if (path !== undefined) {
      return { pathname: path, search, hash, state: historyState };
    }
    return readLocation();
  });

  useEffect(() => {
    if (themeProp !== undefined) {
      setTheme(themeProp);
    }
  }, [themeProp]);

  useEffect(() => {
    if (widthPx !== undefined) {
      setWidth(widthClass(widthPx));
      return;
    }
    const onResize = () => {
      setWidth(widthClass(readWindowWidth()));
    };
    globalThis.addEventListener('resize', onResize);
    return () => {
      globalThis.removeEventListener('resize', onResize);
    };
  }, [widthPx]);

  useEffect(() => {
    if (path !== undefined) {
      setLocation({ pathname: path, search, hash, state: historyState });
      return;
    }
    const onPop = () => {
      setLocation(readLocation());
    };
    globalThis.addEventListener('popstate', onPop);
    return () => {
      globalThis.removeEventListener('popstate', onPop);
    };
  }, [path, search, hash, historyState]);

  const match = matchAddress(location);
  const activePath = match.kind === 'ok' ? match.route.path : '';
  const itemId = match.kind === 'ok' ? match.history.itemId : undefined;
  const items = navItems(messages.shell);
  const currentLandmarks = landmarksForClass(width);

  const navigate = (next: string) => {
    if (onNavigate !== undefined) {
      onNavigate(next);
      return;
    }
    pushPath(globalThis.history, next);
    setLocation({ pathname: next, search: '', hash: '', state: { scrollY: 0, itemId: undefined } });
  };

  const openAlbum = (albumId: string) => {
    const nextPath = detailPath(match);
    if (onNavigate !== undefined) {
      onNavigate(nextPath);
      setLocation({
        pathname: nextPath,
        search: '',
        hash: '',
        state: { scrollY: 0, itemId: albumId },
      });
      return;
    }
    pushPath(globalThis.history, nextPath, 0, albumId);
    setLocation({
      pathname: nextPath,
      search: '',
      hash: '',
      state: { scrollY: 0, itemId: albumId },
    });
  };

  const backFromAlbum = () => {
    const nextPath = detailPath(match);
    if (onNavigate !== undefined) {
      onNavigate(nextPath);
      setLocation({
        pathname: nextPath,
        search: '',
        hash: '',
        state: { scrollY: 0, itemId: undefined },
      });
      return;
    }
    pushPath(globalThis.history, nextPath);
    setLocation({
      pathname: nextPath,
      search: '',
      hash: '',
      state: { scrollY: 0, itemId: undefined },
    });
  };

  const changeTheme = (next: ThemeId) => {
    setTheme(next);
    onThemeChange?.(next);
  };

  const settingsLink = (
    <View
      id="nav-item-settings"
      dataSet={{ navGlyph: 'settings', selected: activePath === '/settings' ? '1' : '0' }}
      accessibilityRole="link"
      accessibilityLabel={messages.shell.navSettings}
      accessibilityState={{ selected: activePath === '/settings' }}
      tabIndex={0}
      onClick={() => {
        navigate('/settings');
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          navigate('/settings');
        }
      }}
    >
      <Text dataSet={{ navLabel: '1' }}>{messages.shell.navSettings}</Text>
    </View>
  );

  /* The theme switcher lives in Settings; the rail and the brand bar stay quiet. */
  const navFooter = <View id="nav-footer">{settingsLink}</View>;

  const showWideQueue = width === 'wide';
  const sidebarBrand = width === 'expanded' || width === 'wide';
  const topBarBrand = !sidebarBrand;
  const state = playback.state;
  const artTone = state.playing && state.trackId !== undefined ? state.coverTone : undefined;
  const playingAlbumTitle =
    library !== undefined && state.albumId !== undefined ? findAlbumTitle(library, state.albumId) : undefined;

  const brandBlock = (
    <View id="shell-brand">
      <View id="shell-brand-mark">
        <Text id="shell-wordmark">{messages.shell.wordmark}</Text>
        <View id="shell-brand-rule" accessibilityRole="none" />
      </View>
      {showDemoLabel ? <Text id="demo-label">{messages.shell.demoData}</Text> : null}
      {topBarBrand ? settingsLink : null}
    </View>
  );

  const skipToContent = (
    <View
      id="skip-to-content"
      accessibilityRole="link"
      accessibilityLabel={messages.shell.skipToContent}
      tabIndex={0}
      onClick={() => {
        focusLandmark('content');
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          focusLandmark('content');
        }
      }}
    >
      <Text>{messages.shell.skipToContent}</Text>
    </View>
  );

  const skipToPlayer = (
    <View
      id="skip-to-player"
      accessibilityRole="link"
      accessibilityLabel={messages.shell.skipToPlayer}
      tabIndex={0}
      onClick={() => {
        focusLandmark('player-bar');
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          focusLandmark('player-bar');
        }
      }}
    >
      <Text>{messages.shell.skipToPlayer}</Text>
    </View>
  );

  /* On the phone, playing opens the full player (SUR-010). */
  const playAlbum = (albumId: string) => {
    playback.playAlbum(albumId);
    if (width === 'compact') {
      playback.openFull();
    }
  };
  const playTrack = (albumId: string, trackId: string) => {
    playback.playTrack(albumId, trackId);
    if (width === 'compact') {
      playback.openFull();
    }
  };

  return (
    <View
      id="token-shell"
      dataSet={{
        theme,
        width,
        landmarks: currentLandmarks.join(' '),
        ...(artTone !== undefined ? { artTone } : {}),
      }}
    >
      <View id="skip-links">
        {skipToContent}
        {skipToPlayer}
      </View>
      <View id="shell-frame">
        {sidebarBrand ? null : brandBlock}
        {width === 'medium' ? (
          <Nav
            id="nav-rail"
            label={messages.shell.primaryNav}
            items={items}
            activePath={activePath}
            onNavigate={navigate}
            footer={navFooter}
          />
        ) : null}
        {sidebarBrand ? (
          <Nav
            id="nav-sidebar"
            label={messages.shell.primaryNav}
            items={items}
            activePath={activePath}
            onNavigate={navigate}
            brand={brandBlock}
            footer={navFooter}
          />
        ) : null}
        <View id="content" accessibilityRole="main" tabIndex={-1}>
          <Destination
            searchLibrary={searchLibrary}
            lyricsFor={lyricsFor}
            pluginSlots={pluginSlots ?? []}
            match={match}
            messages={messages}
            library={library}
            itemId={itemId}
            theme={theme}
            onThemeChange={changeTheme}
            onOpenAlbum={openAlbum}
            onOpenArtist={openAlbum}
            onBackFromAlbum={backFromAlbum}
            onPlayAlbum={playAlbum}
            onPlayTrack={playTrack}
            onPlayNextAlbum={playback.playNextAlbum}
            onAddAlbumToQueue={playback.addAlbumToQueue}
            onPlayNextTrack={playback.playNextTrack}
            onAddTrackToQueue={playback.addTrackToQueue}
            onSeeAll={() => {
              navigate('/library');
            }}
            currentTrackId={state.trackId}
            width={width}
          />
        </View>
        {showWideQueue ? (
          <QueuePane messages={messages.shell} playback={state} compactSheet={false} />
        ) : (
          <QueuePane messages={messages.shell} playback={state} compactSheet onCloseSheet={playback.closeQueue} />
        )}
        <PlayerBar
          messages={messages.shell}
          playback={state}
          albumTitle={playingAlbumTitle}
          compact={width === 'compact'}
          volume={playback.volume}
          onVolume={playback.setVolume}
          onSeek={playback.seek}
          onPlayPause={playback.playPause}
          onPrevious={playback.previous}
          onNext={playback.next}
          onToggleQueue={playback.toggleQueue}
          onOpenFull={playback.openFull}
        />
        <PlayerFull
          lyricsFor={lyricsFor}
          messages={messages.shell}
          playback={state}
          open={playback.fullOpen && state.trackId !== undefined}
          placement={width === 'compact' ? 'overlay' : 'pane'}
          albumTitle={playingAlbumTitle}
          onClose={playback.closeFull}
          onPlayPause={playback.playPause}
          onPrevious={playback.previous}
          onNext={playback.next}
          onToggleQueue={playback.toggleQueue}
        />
        {width === 'compact' ? (
          <Nav
            id="nav-tabs"
            label={messages.shell.primaryNav}
            items={items}
            activePath={activePath}
            onNavigate={navigate}
          />
        ) : null}
      </View>
    </View>
  );
}

function findAlbumTitle(library: ShellLibrary, albumId: string): string | undefined {
  return library.albums.find((album) => album.id === albumId)?.title;
}
