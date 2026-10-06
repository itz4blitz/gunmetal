import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import { catalogue } from '../messages/catalogue.ts';
import { matchAddress, type MatchResult } from '../router/match.ts';
import { pushPath } from '../router/navigate.ts';
import { Destination } from './destinations/Destination.tsx';
import { Nav, navItems } from './Nav.tsx';
import { PlayerBar } from './PlayerBar.tsx';
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
  onNavigate?: (path: string) => void;
  onThemeChange?: (theme: ThemeId) => void;
};

function headlineFor(match: MatchResult, messages: ReturnType<typeof catalogue>): string {
  if (match.kind === 'not-found') {
    return messages.destinations.notFoundHeadline;
  }
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

export function Shell({
  path,
  search = '',
  hash = '',
  historyState = null,
  widthPx,
  theme: themeProp,
  showDemoLabel = false,
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

  const changeTheme = (next: ThemeId) => {
    setTheme(next);
    onThemeChange?.(next);
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

  return (
    <View
      id="token-shell"
      dataSet={{
        theme,
        width,
        landmarks: currentLandmarks.join(' '),
      }}
    >
      <View id="shell-frame">
        <View id="shell-brand">
          <Text id="shell-wordmark">{messages.shell.wordmark}</Text>
          {showDemoLabel ? <Text id="demo-label">{messages.shell.demoData}</Text> : null}
        </View>
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
        {width === 'expanded' || width === 'wide' ? (
          <Nav
            id="nav-sidebar"
            label={messages.shell.primaryNav}
            items={items}
            activePath={activePath}
            onNavigate={navigate}
            footer={navFooter}
          />
        ) : null}
        <View id="content" accessibilityRole="main">
          <Destination headline={headlineFor(match, messages)} />
          {width === 'compact' ? navFooter : null}
        </View>
        {width === 'wide' ? (
          <View id="right-pane" accessibilityRole="complementary" accessibilityLabel={messages.shell.rightPane} />
        ) : null}
        <PlayerBar label={messages.shell.playerRegion} emptyLabel={messages.shell.playerEmpty} />
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
