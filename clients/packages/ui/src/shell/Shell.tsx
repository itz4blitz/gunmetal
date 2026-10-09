import { useCallback, useEffect, useLayoutEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import { catalogue, type MessageCatalogue } from '../messages/catalogue.ts';
import { extensionTitle, storeIdFromPath } from '../plugins/repository.ts';
import { BrandMark } from './brand-mark.tsx';
import { itemAddress, resolveMedia } from '../router/catalogue-address.ts';
import { labelFromKey, parseMediaPath } from '../router/media-path.ts';
import { addressOnLoad, browserWins, canonicalPath, matchAddress, type MatchResult } from '../router/match.ts';
import { pushPath, replacePath } from '../router/navigate.ts';
import { noLyrics, type LibrarySearch, type LyricsResolver } from './content.ts';
import { Destination } from './destinations/Destination.tsx';
import { Icon } from './Icon.tsx';
import type { PluginSlot, SettingsCrossfadeSeconds, SettingsLevelling } from './destinations/settings.ts';
import type { ShellLibrary } from './library-types.ts';
import type { TimedLyricsResolver } from './synced-lyrics.ts';
import { useTransportKeys } from './use-transport-keys.ts';
import { Nav, navItems, withPins } from './Nav.tsx';
import { readPins, serializePins, togglePin, type Pin } from './pins.ts';
import { PaneResizer } from './PaneResizer.tsx';
import {
  noLayoutStore,
  parsePaneWidths,
  serializePaneWidths,
  type LayoutStore,
  type PaneId,
  type PaneWidths,
} from './pane-widths.ts';
import { PlayerBar } from './PlayerBar.tsx';
import { PlayerFull } from './PlayerFull.tsx';
import { QueuePane } from './QueuePane.tsx';
import type { PlaybackController } from './playback-controller.ts';
import {
  defaultSystemThemeQuery,
  noSettingsStore,
  parseThemeChoice,
  resolveTheme,
  serializeThemeChoice,
  SYSTEM_THEME_QUERY,
  type SettingsStore,
  type SystemThemeQuery,
  type ThemeId,
} from './theme.ts';
import { landmarksForClass, widthClass, type WidthClass } from './width.ts';

export type ShellProps = {
  path?: string;
  search?: string;
  hash?: string;
  historyState?: unknown;
  widthPx?: number;
  theme?: ThemeId;
  showDemoLabel?: boolean;
  library?: (ShellLibrary & { kind?: string | undefined }) | undefined;
  /** The composition root's playback implementation (see PlaybackController). */
  playback: PlaybackController;
  /** Library search read (demo filter today, ServerPort search later). */
  searchLibrary?: LibrarySearch | undefined;
  /** Lyrics content resolver (fixture verses today, server lyrics later). */
  lyricsFor?: LyricsResolver | undefined;
  /** Timed lyrics for synced tracks (line timestamps today, the server's LRC later). */
  timedLyricsFor?: TimedLyricsResolver | undefined;
  /** Plugin-slot list for the Settings page (server config later). */
  pluginSlots?: readonly PluginSlot[] | undefined;
  /** Where the pane widths are kept between visits (localStorage in apps/demo). */
  layoutStore?: LayoutStore | undefined;
  /** Where the theme choice is kept between visits (localStorage in apps/demo). */
  settingsStore?: SettingsStore | undefined;
  /**
   * Where sidebar pins are kept. Same read/write pair as the settings store.
   * When omitted, pins live in localStorage under `gunmetal.pins`.
   */
  pinsStore?: SettingsStore | undefined;
  /**
   * How the shell asks the operating system about its colour scheme
   * (design-language §4). Defaults to the runtime's matchMedia; null answers
   * "cannot be asked" and resolves the system choice to dark.
   */
  systemThemeQuery?: (query: string) => SystemThemeQuery | null;
  onNavigate?: (path: string) => void;
  onThemeChange?: (theme: ThemeId) => void;
  /** Volume levelling. The settings pane calls `onLevelling` when this is passed. */
  levelling?: SettingsLevelling | undefined;
  onLevelling?: ((levelling: SettingsLevelling) => void) | undefined;
  /** Crossfade length in seconds. The settings pane calls `onCrossfade` when this is passed. */
  crossfadeSeconds?: SettingsCrossfadeSeconds | undefined;
  onCrossfade?: ((seconds: SettingsCrossfadeSeconds) => void) | undefined;
  /** Output devices this browser can play through. */
  outputs?: readonly { id: string; label: string }[] | undefined;
  /** The chosen output. Empty is the browser default. */
  sinkId?: string | undefined;
  onOutput?: ((id: string) => void) | undefined;
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

/* A new page starts at its top: the content pane is the scroller, and it
   outlives the page inside it. */
function scrollContentToTop(): void {
  globalThis.document.querySelectorAll('#content').forEach((content) => {
    content.scrollTop = 0;
  });
}

/** Pins are navigation only. The key is not a layout or theme document. */
const PINS_STORAGE_KEY = 'gunmetal.pins';

/**
 * The settings-store shape, backed by localStorage when the composition root
 * did not pass a pins store. A blocked or full store remembers nothing and
 * does not throw: the pin still applies for this visit.
 */
function browserPinsStore(): SettingsStore {
  return {
    read: () => {
      try {
        return globalThis.localStorage.getItem(PINS_STORAGE_KEY);
      } catch {
        return null;
      }
    },
    write: (value) => {
      try {
        globalThis.localStorage.setItem(PINS_STORAGE_KEY, value);
      } catch {
        // Nothing to do: the pin still applies for this visit.
      }
    },
  };
}

function pinKey(pin: Pin, library: ShellLibrary | undefined): string {
  if (pin.itemId !== undefined) {
    return `item:${pin.itemId}`;
  }
  const media = parseMediaPath(pin.path);
  if (media === undefined || library === undefined) {
    return `path:${pin.path}`;
  }
  const resolved = resolveMedia(library, media);
  if (resolved === undefined) {
    return `path:${pin.path}`;
  }
  return `item:${resolved.itemId}`;
}

function commitPins(
  current: readonly Pin[],
  target: Pin,
  store: SettingsStore,
  library: ShellLibrary | undefined,
): readonly Pin[] {
  const matches = current.some((pin) => samePinnedPage(pin, target, library));
  const without = current.filter((pin) => !samePinnedPage(pin, target, library));
  const next = matches ? without : togglePin(without, target);
  store.write(serializePins(next));
  return next;
}

/** A destination the person can pin. Anything else (not found) has no button. */
function pinForRoute(path: string, messages: MessageCatalogue): Pin | undefined {
  if (path === '/') {
    return { path, label: messages.shell.navHome };
  }
  if (path === '/search') {
    return { path, label: messages.shell.navSearch };
  }
  if (path === '/library') {
    return { path, label: messages.shell.navLibrary };
  }
  if (path === '/store') {
    return { path, label: messages.shell.navStore };
  }
  const storeId = storeIdFromPath(path);
  if (storeId !== undefined) {
    return { path, label: extensionTitle(storeId) };
  }
  if (path === '/settings/appearance') {
    return { path, label: messages.destinations.settingsAppearance };
  }
  if (path === '/settings/playback') {
    return { path, label: messages.destinations.settingsPlayback };
  }
  if (path === '/settings/connected') {
    return { path, label: messages.destinations.settingsConnected };
  }
  if (path === '/settings/about') {
    return { path, label: messages.destinations.settingsAbout };
  }
  if (path === '/settings/privacy') {
    return { path, label: messages.destinations.settingsPrivacy };
  }
  return undefined;
}

/** The open album or artist, or the page itself when nothing is open. */
function pinForPlace(
  path: string,
  itemId: string | undefined,
  library: ShellLibrary | undefined,
  messages: MessageCatalogue,
): Pin | undefined {
  const media = parseMediaPath(path);
  if (media !== undefined) {
    if (library !== undefined) {
      const resolved = resolveMedia(library, media);
      if (resolved !== undefined) {
        return { path: resolved.path, label: resolved.label };
      }
    }
    return { path: media.path, label: labelFromKey(media.key) };
  }
  if (itemId !== undefined && library !== undefined) {
    const artist = library.artists.find((entry) => entry.key === itemId);
    if (artist !== undefined) {
      return { path: '/library', label: artist.name, itemId };
    }
    const album = library.albums.find((entry) => entry.id === itemId);
    if (album !== undefined) {
      return { path: '/library', label: album.title, itemId };
    }
  }
  return pinForRoute(path, messages);
}

function samePinnedPage(left: Pin, right: Pin, library: ShellLibrary | undefined): boolean {
  return pinKey(left, library) === pinKey(right, library);
}

/**
 * Nav matches a path exactly. A settings section that is not itself pinned
 * still marks Settings, which is the page those sections belong to.
 */
function navActivePath(activePath: string, items: readonly { path: string }[]): string {
  if (items.some((item) => item.path === activePath)) {
    return activePath;
  }
  if (activePath.startsWith('/store')) {
    return '/store';
  }
  if (activePath.startsWith('/settings')) {
    return '/settings';
  }
  return activePath;
}

function librarySizeFact(library: (ShellLibrary & { kind?: string | undefined }) | undefined): string | undefined {
  if (library === undefined || library.kind !== 'folder') {
    return undefined;
  }
  return `${library.albums.length} albums · ${library.artists.length} artists`;
}

/* The frame's grid reads the pane widths from two custom properties on the
   shell root; they are written through the CSSOM, never a style attribute. */
function applyPaneWidth(pane: PaneId, widthPx: number): void {
  const shell = globalThis.document.getElementById('token-shell');
  shell?.style.setProperty(`--gm-${pane}-w`, `${widthPx}px`);
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
  timedLyricsFor,
  pluginSlots,
  layoutStore,
  settingsStore,
  pinsStore,
  systemThemeQuery = defaultSystemThemeQuery,
  onNavigate,
  onThemeChange,
  levelling,
  onLevelling,
  crossfadeSeconds,
  onCrossfade,
  outputs,
  sinkId,
  onOutput,
}: ShellProps) {
  const messages = catalogue();
  const [store] = useState<LayoutStore>(() => layoutStore ?? noLayoutStore());
  const [paneWidths, setPaneWidths] = useState<PaneWidths>(() => parsePaneWidths(store.read()));
  const [settings] = useState<SettingsStore>(() => settingsStore ?? noSettingsStore());
  const [pinsKept] = useState<SettingsStore>(() => pinsStore ?? browserPinsStore());
  const [pins, setPins] = useState<readonly Pin[]>(() => readPins(pinsKept.read()));
  /* The choice (what was picked, `system` included) and the OS's answer are
     two states: only their meeting decides what `data-theme` paints. */
  const [themeChoice, setThemeChoice] = useState<ThemeId>(() => themeProp ?? parseThemeChoice(settings.read()));
  const [systemDark, setSystemDark] = useState<boolean>(() => systemThemeQuery(SYSTEM_THEME_QUERY)?.matches ?? true);
  const [width, setWidth] = useState<WidthClass>(() => widthClass(widthPx ?? readWindowWidth()));
  const [location, setLocation] = useState(() =>
    addressOnLoad(readLocation(), { path, search, hash, state: historyState }),
  );

  useEffect(() => {
    if (themeProp !== undefined) {
      setThemeChoice(themeProp);
    }
  }, [themeProp]);

  /* While the choice is `system` the shell follows the operating system
     live: a change event re-resolves the theme without a reload. A named
     choice unsubscribes; choosing System again re-arms. */
  useEffect(() => {
    if (themeChoice !== 'system') {
      return;
    }
    const query = systemThemeQuery(SYSTEM_THEME_QUERY);
    if (query === null) {
      return;
    }
    const onSchemeChange = () => {
      setSystemDark(query.matches);
    };
    onSchemeChange();
    query.addEventListener('change', onSchemeChange);
    return () => {
      query.removeEventListener('change', onSchemeChange);
    };
  }, [themeChoice, systemThemeQuery]);

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
    const live = readLocation();
    const parent = { path, search, hash, state: historyState };
    const urlWins = browserWins(live.pathname, path);
    if (!urlWins) {
      setLocation(addressOnLoad(live, parent));
      return;
    }
    if (path !== undefined) {
      setLocation(addressOnLoad(live, parent));
    }
    const onPop = () => {
      const next = readLocation();
      setLocation(addressOnLoad(next, { path: undefined, search: '', hash: '', state: next.state }));
    };
    globalThis.addEventListener('popstate', onPop);
    return () => {
      globalThis.removeEventListener('popstate', onPop);
    };
  }, [path, search, hash, historyState]);

  /* `/settings` is appearance. Replace it so the address bar matches the page.
     A query or a fragment is a refused address and stays as the browser has it. */
  useLayoutEffect(() => {
    if (path !== undefined) {
      return;
    }
    const live = readLocation();
    const next = canonicalPath(live.pathname);
    if (next === live.pathname || live.search !== '' || live.hash !== '') {
      return;
    }
    replacePath(globalThis.history, next);
  }, [path, location.pathname]);

  /* An id address opens the same page as its slug. The address bar shows the
     style this build emits. A controlled preview path is left alone. */
  useLayoutEffect(() => {
    if (path !== undefined || library === undefined) {
      return;
    }
    const media = parseMediaPath(location.pathname);
    if (media === undefined) {
      return;
    }
    const resolved = resolveMedia(library, media);
    if (resolved === undefined || resolved.path === location.pathname) {
      return;
    }
    replacePath(globalThis.history, resolved.path);
    setLocation({
      pathname: resolved.path,
      search: '',
      hash: '',
      state: { scrollY: 0, itemId: undefined },
    });
  }, [path, library, location.pathname]);

  useLayoutEffect(() => {
    applyPaneWidth('sidebar', paneWidths.sidebar);
    applyPaneWidth('queue', paneWidths.queue);
  }, [paneWidths]);

  const resizeSidebar = useCallback((widthPx: number) => {
    applyPaneWidth('sidebar', widthPx);
  }, []);
  const resizeQueue = useCallback((widthPx: number) => {
    applyPaneWidth('queue', widthPx);
  }, []);
  const commitPane = useCallback(
    (pane: PaneId, widthPx: number) => {
      setPaneWidths((current) => {
        const next = { ...current, [pane]: widthPx };
        store.write(serializePaneWidths(next));
        return next;
      });
    },
    [store],
  );
  const commitSidebar = useCallback(
    (widthPx: number) => {
      commitPane('sidebar', widthPx);
    },
    [commitPane],
  );
  const commitQueue = useCallback(
    (widthPx: number) => {
      commitPane('queue', widthPx);
    },
    [commitPane],
  );

  /* The transport keys answer from wherever the listener is (R1 keyboard
     proposal): Space, the arrows and Enter, unless a field owns the key. */
  useTransportKeys({
    onPlayPause: playback.playPause,
    onNext: playback.next,
    onSeekBy: (deltaMs) => {
      playback.seek(playback.state.positionMs + deltaMs);
    },
  });

  const match = matchAddress(location);
  const activePath = match.kind === 'ok' ? match.route.path : '';
  const resolvedMedia = match.kind === 'ok' && match.media !== undefined && library !== undefined ? resolveMedia(library, match.media) : undefined;
  const itemId =
    resolvedMedia !== undefined
      ? resolvedMedia.itemId
      : match.kind === 'ok'
        ? match.history.itemId
        : undefined;
  const pageId = `${location.pathname} ${itemId ?? ''}`;
  const resolvedTheme = resolveTheme(themeChoice, systemDark);

  useLayoutEffect(() => {
    scrollContentToTop();
  }, [pageId]);
  const items = withPins(navItems(messages.shell, messages.destinations), pins);
  const currentLandmarks = landmarksForClass(width);
  const navActive = navActivePath(activePath, items);
  const pinTarget = pinForPlace(activePath, itemId, library, messages);
  const pinPressed = pinTarget !== undefined && pins.some((pin) => samePinnedPage(pin, pinTarget, library));
  const pinName = pinPressed ? 'Unpins the page from the sidebar' : 'Pins the page to the sidebar';
  const servedLibrary = library?.kind === 'folder';
  const libraryFact = librarySizeFact(library);

  const navigate = (next: string) => {
    const pathname = canonicalPath(next);
    pushPath(globalThis.history, pathname);
    setLocation({ pathname, search: '', hash: '', state: { scrollY: 0, itemId: undefined } });
    if (onNavigate !== undefined) {
      onNavigate(pathname);
    }
  };

  const openAlbum = (albumId: string) => {
    const nextPath = itemAddress(library, albumId) ?? detailPath(match);
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

  const openFromNav = (path: string, navItemId?: string) => {
    if (navItemId !== undefined) {
      openAlbum(navItemId);
      return;
    }
    navigate(path);
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
    setThemeChoice(next);
    settings.write(serializeThemeChoice(next));
    onThemeChange?.(next);
  };

  const showWideQueue = width === 'wide';
  const sidebarBrand = width === 'expanded' || width === 'wide';
  const state = playback.state;
  const artTone = state.playing && state.trackId !== undefined ? state.coverTone : undefined;
  const playingAlbumTitle =
    library !== undefined && state.albumId !== undefined ? findAlbumTitle(library, state.albumId) : undefined;

  const brandBlock = (
    <View id="shell-brand">
      <View id="shell-brand-mark">
        <View id="shell-brand-lockup">
          <BrandMark size={22} />
          <Text id="shell-wordmark">{messages.shell.wordmark}</Text>
        </View>
        <View id="shell-brand-rule" accessibilityRole="none" />
      </View>
      {showDemoLabel && !servedLibrary ? <Text id="demo-label">{messages.shell.demoData}</Text> : null}
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
        theme: resolvedTheme,
        themeChoice,
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
            activePath={navActive}
            activeItemId={itemId}
            onNavigate={openFromNav}
          />
        ) : null}
        {sidebarBrand ? (
          <Nav
            id="nav-sidebar"
            label={messages.shell.primaryNav}
            items={items}
            activePath={navActive}
            activeItemId={itemId}
            onNavigate={openFromNav}
            brand={brandBlock}
          />
        ) : null}
        {sidebarBrand ? (
          <PaneResizer
            pane="sidebar"
            label={messages.shell.resizeSidebar}
            widthPx={paneWidths.sidebar}
            onResize={resizeSidebar}
            onCommit={commitSidebar}
          />
        ) : null}
        <View id="content" accessibilityRole="main" tabIndex={-1}>
          {pinTarget === undefined ? null : (
            <View
              id="destination-pin"
              accessibilityRole="button"
              accessibilityLabel={pinName}
              aria-pressed={pinPressed}
              dataSet={{ pressed: pinPressed ? '1' : '0', tip: pinName }}
              tabIndex={0}
              onClick={() => {
                setPins((current) => commitPins(current, pinTarget, pinsKept, library));
              }}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  setPins((current) => commitPins(current, pinTarget, pinsKept, library));
                }
              }}
            >
              <Icon name="pin" size={16} />
            </View>
          )}
          <Destination
            searchLibrary={searchLibrary}
            lyricsFor={lyricsFor}
            pluginSlots={pluginSlots ?? []}
            match={match}
            messages={messages}
            library={library}
            itemId={itemId}
            theme={themeChoice}
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
            onOpenPath={navigate}
            levelling={levelling}
            onLevelling={onLevelling}
            crossfadeSeconds={crossfadeSeconds}
            onCrossfade={onCrossfade}
            outputs={outputs}
            sinkId={sinkId}
            onOutput={onOutput}
            currentTrackId={state.trackId}
            playingAlbumId={state.albumId}
            served={library?.kind === 'folder'}
            libraryFact={libraryFact}
            width={width}
          />
        </View>
        {showWideQueue ? (
          <PaneResizer
            pane="queue"
            label={messages.shell.resizeQueue}
            widthPx={paneWidths.queue}
            onResize={resizeQueue}
            onCommit={commitQueue}
          />
        ) : null}
        {showWideQueue ? (
          <QueuePane
            messages={messages.shell}
            playback={state}
            compactSheet={false}
            onPlayLine={playback.playTrack}
            onRemoveLine={playback.removeQueueLine}
          />
        ) : (
          <QueuePane
            messages={messages.shell}
            playback={state}
            compactSheet
            onCloseSheet={playback.closeQueue}
            onPlayLine={playback.playTrack}
            onRemoveLine={playback.removeQueueLine}
          />
        )}
        <PlayerBar
          messages={messages.shell}
          playback={state}
          albumTitle={playingAlbumTitle}
          compact={width === 'compact'}
          volume={playback.volume}
          muted={playback.muted}
          onMuted={playback.setMuted}
          clock={playback.clock}
          onVolume={playback.setVolume}
          onSeek={playback.seek}
          onPlayPause={playback.playPause}
          onPrevious={playback.previous}
          onNext={playback.next}
          onToggleShuffle={playback.toggleShuffle}
          onCycleRepeat={playback.cycleRepeat}
          onToggleQueue={playback.toggleQueue}
          onOpenFull={playback.openFull}
        />
        <PlayerFull
          lyricsFor={lyricsFor}
          timedLyricsFor={timedLyricsFor}
          messages={messages.shell}
          playback={state}
          open={playback.fullOpen && state.trackId !== undefined}
          placement={width === 'compact' ? 'overlay' : 'pane'}
          albumTitle={playingAlbumTitle}
          volume={playback.volume}
          muted={playback.muted}
          onMuted={playback.setMuted}
          clock={playback.clock}
          onVolume={playback.setVolume}
          onSeek={playback.seek}
          onClose={playback.closeFull}
          onPlayPause={playback.playPause}
          onPrevious={playback.previous}
          onNext={playback.next}
          onToggleShuffle={playback.toggleShuffle}
          onCycleRepeat={playback.cycleRepeat}
          onToggleQueue={playback.toggleQueue}
        />
        {width === 'compact' ? (
          <Nav
            id="nav-tabs"
            label={messages.shell.primaryNav}
            items={items}
            activePath={navActive}
            activeItemId={itemId}
            onNavigate={openFromNav}
          />
        ) : null}
      </View>
    </View>
  );
}

function findAlbumTitle(library: ShellLibrary, albumId: string): string | undefined {
  return library.albums.find((album) => album.id === albumId)?.title;
}
