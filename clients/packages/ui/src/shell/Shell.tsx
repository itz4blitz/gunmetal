import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import { catalogue } from '../messages/catalogue.ts';
import { matchAddress, type MatchResult } from '../router/match.ts';
import { pushPath } from '../router/navigate.ts';
import { Destination } from './destinations/Destination.tsx';
import type { ShellLibrary } from './library-types.ts';
import { Nav, navItems } from './Nav.tsx';
import { applyPlayback } from './demo-play.ts';
import {
  emptyPlayback,
  setQueueOpen,
  stepQueue,
  togglePlaying,
  type PlaybackSnapshot,
} from './playback.ts';
import { PlayerBar } from './PlayerBar.tsx';
import { QueuePane } from './QueuePane.tsx';
import { defaultTheme, type ThemeId } from './theme.ts';
import { ThemeSwitcher } from './ThemeSwitcher.tsx';
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
  onNavigate,
  onThemeChange,
}: ShellProps) {
  const messages = catalogue();
  const [theme, setTheme] = useState<ThemeId>(themeProp ?? defaultTheme());
  const [width, setWidth] = useState<WidthClass>(() => widthClass(widthPx ?? readWindowWidth()));
  const [playback, setPlayback] = useState<PlaybackSnapshot>(() => emptyPlayback());
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

  const playAlbum = (albumId: string) => {
    setPlayback(applyPlayback(library, albumId, undefined));
  };

  const playTrack = (albumId: string, trackId: string) => {
    setPlayback(applyPlayback(library, albumId, trackId));
  };

  const themeFooter = (
    <ThemeSwitcher messages={messages.shell} theme={theme} onThemeChange={changeTheme} />
  );

  const settingsLink = (
    <View
      id="nav-item-settings"
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
      <Text>{messages.shell.navSettings}</Text>
    </View>
  );

  const navFooter = (
    <View id="nav-footer">
      {settingsLink}
      {themeFooter}
    </View>
  );

  const showWideQueue = width === 'wide';
  const sidebarBrand = width === 'expanded' || width === 'wide';
  const artTone = playback.playing && playback.trackId !== undefined ? playback.coverTone : undefined;

  const brandBlock = (
    <View id="shell-brand">
      <View id="shell-brand-mark">
        <Text id="shell-wordmark">{messages.shell.wordmark}</Text>
        <View id="shell-brand-rule" accessibilityRole="none" />
      </View>
      {showDemoLabel ? <Text id="demo-label">{messages.shell.demoData}</Text> : null}
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
            match={match}
            messages={messages}
            library={library}
            itemId={itemId}
            theme={theme}
            onThemeChange={changeTheme}
            onOpenAlbum={openAlbum}
            onBackFromAlbum={backFromAlbum}
            onPlayAlbum={playAlbum}
            onPlayTrack={playTrack}
            onSeeAll={() => {
              navigate('/library');
            }}
          />
          {width === 'compact' ? navFooter : null}
        </View>
        {showWideQueue ? (
          <QueuePane messages={messages.shell} playback={playback} compactSheet={false} />
        ) : (
          <QueuePane
            messages={messages.shell}
            playback={playback}
            compactSheet
            onCloseSheet={() => {
              setPlayback((current) => setQueueOpen(current, false));
            }}
          />
        )}
        <PlayerBar
          messages={messages.shell}
          playback={playback}
          onPlayPause={() => {
            setPlayback((current) => togglePlaying(current));
          }}
          onPrevious={() => {
            setPlayback((current) => stepQueue(current, -1));
          }}
          onNext={() => {
            setPlayback((current) => stepQueue(current, 1));
          }}
          onToggleQueue={() => {
            setPlayback((current) => setQueueOpen(current, !current.queueOpen));
          }}
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
