import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, test } from 'vitest';
import { stubPlayback, queuedSnapshot } from './test-playback.ts';
import { demoLibrary } from '../../../fake-server/src/catalogue.ts';
import { demoLocalFilter } from '../../../fake-server/src/filter.ts';
import { shellMessages } from '../messages/en/shell.ts';
import { landmarks } from './width.ts';
import { Shell } from './Shell.tsx';

const messages = shellMessages();

beforeEach(() => {
  window.localStorage.removeItem('gunmetal.pins');
});

afterEach(() => {
  cleanup();
  window.history.pushState(null, '', '/');
  window.localStorage.removeItem('gunmetal.pins');
});

function landmarkIds(root: HTMLElement): string[] {
  const order = ['nav-sidebar', 'nav-rail', 'content', 'right-pane', 'player-bar', 'nav-tabs'];
  return order.filter((id) => root.querySelector(`#${id}`) !== null);
}

function sidebarLabels(sidebar: Element | null): string[] {
  return [...(sidebar?.querySelectorAll('[data-nav-label]') ?? [])].map((node) => node.textContent ?? '');
}

test('wide shell landmarks match the literal list and show Home', () => {
  const { container } = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} showDemoLabel />);
  const root = container.querySelector('#token-shell');
  expect(root?.getAttribute('data-width')).toStrictEqual('wide');
  expect(root?.getAttribute('data-theme')).toStrictEqual('dark');
  expect(landmarkIds(root as HTMLElement)).toStrictEqual([...landmarks(1600)]);
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  expect(screen.getByText('Fixture library').id).toStrictEqual('demo-label');
  expect(screen.getByRole('navigation', { name: 'Primary' }).id).toStrictEqual('nav-sidebar');
  expect(screen.getByRole('complementary', { name: 'Queue' }).id).toStrictEqual('right-pane');
  expect(screen.getByRole('region', { name: 'Now playing' }).id).toStrictEqual('player-bar');
});

test('the brand lockup carries the nut mark beside the wordmark', () => {
  const { container } = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} showDemoLabel />);
  const lockup = container.querySelector('#shell-brand-lockup');
  expect(lockup).not.toBeNull();
  expect(lockup?.querySelector('svg[data-brand-mark="1"]')).not.toBeNull();
  expect(lockup?.querySelector('#shell-wordmark')?.textContent).toStrictEqual('Gunmetal');
  // The brass rule hangs below the lockup, inside the brand block.
  expect(container.querySelector('#shell-brand-mark #shell-brand-rule')).not.toBeNull();
});

test('compact, medium and expanded shells expose their landmark lists', () => {
  const compact = render(<Shell playback={stubPlayback().controller} path="/search" widthPx={360} />);
  expect(landmarkIds(compact.container.querySelector('#token-shell') as HTMLElement)).toStrictEqual([
    ...landmarks(360),
  ]);
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('navigation', { name: 'Primary' }).id).toStrictEqual('nav-tabs');
  compact.unmount();

  const medium = render(<Shell playback={stubPlayback().controller} path="/library" widthPx={800} />);
  expect(landmarkIds(medium.container.querySelector('#token-shell') as HTMLElement)).toStrictEqual([...landmarks(800)]);
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('navigation', { name: 'Primary' }).id).toStrictEqual('nav-rail');
  medium.unmount();

  const expanded = render(<Shell playback={stubPlayback().controller} path="/settings" widthPx={1200} />);
  expect(landmarkIds(expanded.container.querySelector('#token-shell') as HTMLElement)).toStrictEqual([
    ...landmarks(1200),
  ]);
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('navigation', { name: 'Primary' }).id).toStrictEqual('nav-sidebar');
  expanded.unmount();
});

test('an unknown path, fragment and query each show Not found', () => {
  const unknown = render(<Shell playback={stubPlayback().controller} path="/nope" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  unknown.unmount();

  const fragment = render(<Shell playback={stubPlayback().controller} path="/" hash="#x" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  fragment.unmount();

  const query = render(<Shell playback={stubPlayback().controller} path="/search" search="?q=1" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  query.unmount();

  const badId = render(
    <Shell playback={stubPlayback().controller} path="/" historyState={{ itemId: 'a/b' }} widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  badId.unmount();
});

test('nav links call the composition callbacks; themes switch inside Settings', () => {
  const navigated: string[] = [];
  const themes: string[] = [];
  render(
    <Shell
      playback={stubPlayback().controller}
      path="/"
      widthPx={1600}
      onNavigate={(next) => {
        navigated.push(next);
      }}
      onThemeChange={(next) => {
        themes.push(next);
      }}
    />,
  );
  fireEvent.click(screen.getByRole('link', { name: 'Search' }));
  fireEvent.click(screen.getByRole('link', { name: 'Library' }));
  fireEvent.click(screen.getByRole('link', { name: 'Home' }));
  fireEvent.click(screen.getByRole('link', { name: 'Settings' }));
  fireEvent.keyDown(screen.getByRole('link', { name: 'Search' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('link', { name: 'Library' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('link', { name: 'Settings' }), { key: 'Enter' });
  expect(navigated).toStrictEqual([
    '/search',
    '/library',
    '/',
    '/settings/appearance',
    '/search',
    '/library',
    '/settings/appearance',
  ]);
  expect(window.location.pathname).toStrictEqual('/settings/appearance');
  // The sidebar no longer hosts a theme stack; the switcher lives in Settings.
  expect(screen.queryByRole('button', { name: 'Light' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'OLED' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'High contrast' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Dark' })).toBeNull();
  expect(themes).toStrictEqual([]);
});

test('playback controls report levelling, crossfade and output to the composition root', () => {
  const levelling: string[] = [];
  const fades: number[] = [];
  const sinks: string[] = [];
  render(
    <Shell
      playback={stubPlayback().controller}
      path="/settings/playback"
      widthPx={1600}
      library={demoLibrary()}
      levelling="album"
      onLevelling={(value) => {
        levelling.push(value);
      }}
      crossfadeSeconds={8}
      onCrossfade={(value) => {
        fades.push(value);
      }}
      outputs={[{ id: 'speakers', label: 'Studio speakers' }]}
      sinkId="speakers"
      onOutput={(id) => {
        sinks.push(id);
      }}
    />,
  );
  expect(screen.getByRole('radio', { name: 'Album' }).getAttribute('aria-checked')).toStrictEqual('true');
  expect(screen.getByRole('radio', { name: '8 seconds' }).getAttribute('aria-checked')).toStrictEqual('true');
  expect(screen.getByRole('radio', { name: 'Studio speakers' }).getAttribute('aria-checked')).toStrictEqual('true');
  fireEvent.click(screen.getByRole('radio', { name: 'Track' }));
  fireEvent.click(screen.getByRole('radio', { name: '2 seconds' }));
  fireEvent.click(screen.getByRole('radio', { name: 'Default' }));
  expect(levelling).toStrictEqual(['track']);
  expect(fades).toStrictEqual([2]);
  expect(sinks).toStrictEqual(['']);
});

test('theme changes from the Settings destination reach the composition callback', () => {
  const themes: string[] = [];
  render(
    <Shell
      playback={stubPlayback().controller}
      path="/settings"
      theme="dark"
      widthPx={1600}
      library={demoLibrary()}
      onThemeChange={(next) => {
        themes.push(next);
      }}
    />,
  );
  // The theme is one radio group of preview cards (one control per theme).
  fireEvent.click(screen.getByRole('radio', { name: 'Light' }));
  fireEvent.click(screen.getByRole('radio', { name: 'OLED' }));
  fireEvent.click(screen.getByRole('radio', { name: 'High contrast' }));
  fireEvent.click(screen.getByRole('radio', { name: 'Dark' }));
  fireEvent.keyDown(screen.getByRole('radio', { name: 'Light' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('radio', { name: 'OLED' }), { key: ' ' });
  expect(themes).toStrictEqual(['light', 'oled', 'high-contrast', 'dark', 'light', 'oled']);
});

/** A MediaQueryList stand-in whose answer (and listeners) the test controls. */
function fakeSystemQuery(initial: boolean) {
  let matches = initial;
  const listeners: Array<() => void> = [];
  const mql = {
    get matches() {
      return matches;
    },
    addEventListener(_type: 'change', listener: () => void) {
      listeners.push(listener);
    },
    removeEventListener(_type: 'change', listener: () => void) {
      const at = listeners.indexOf(listener);
      if (at !== -1) {
        listeners.splice(at, 1);
      }
    },
  };
  return {
    query: () => mql,
    change(next: boolean) {
      matches = next;
      for (const listener of [...listeners]) {
        listener();
      }
    },
    listenerCount: () => listeners.length,
  };
}

function fakeSettingsStore(initial: string | null) {
  let stored = initial;
  return {
    store: {
      read: () => stored,
      write: (value: string) => {
        stored = value;
      },
    },
    value: () => stored,
  };
}

test('the system choice resolves through the injected media query and follows it live', () => {
  const system = fakeSystemQuery(false);
  const view = render(
    <Shell playback={stubPlayback().controller} path="/" widthPx={1600} systemThemeQuery={system.query} />,
  );
  const root = view.container.querySelector('#token-shell');
  // The choice and its resolution are two attributes: what was chosen, and
  // what the shell paints with.
  expect(root?.getAttribute('data-theme-choice')).toStrictEqual('system');
  expect(root?.getAttribute('data-theme')).toStrictEqual('light');
  act(() => {
    system.change(true);
  });
  expect(root?.getAttribute('data-theme')).toStrictEqual('dark');
  expect(root?.getAttribute('data-theme-choice')).toStrictEqual('system');
});

test('a named choice stops following the system, and returning re-arms the listener', () => {
  const system = fakeSystemQuery(false);
  render(
    <Shell
      playback={stubPlayback().controller}
      path="/settings"
      widthPx={1600}
      library={demoLibrary()}
      systemThemeQuery={system.query}
    />,
  );
  const root = document.querySelector('#token-shell');
  fireEvent.click(screen.getByRole('radio', { name: 'Dark' }));
  expect(root?.getAttribute('data-theme-choice')).toStrictEqual('dark');
  expect(root?.getAttribute('data-theme')).toStrictEqual('dark');
  expect(system.listenerCount()).toStrictEqual(0);
  // The OS flipping underneath no longer moves a named theme.
  act(() => {
    system.change(true);
  });
  expect(root?.getAttribute('data-theme')).toStrictEqual('dark');
  fireEvent.click(screen.getByRole('radio', { name: 'System' }));
  expect(root?.getAttribute('data-theme-choice')).toStrictEqual('system');
  expect(system.listenerCount()).toStrictEqual(1);
});

test('when the system query cannot be asked the shell falls back to dark', () => {
  const view = render(
    <Shell playback={stubPlayback().controller} path="/" widthPx={1600} systemThemeQuery={() => null} />,
  );
  const root = view.container.querySelector('#token-shell');
  expect(root?.getAttribute('data-theme-choice')).toStrictEqual('system');
  expect(root?.getAttribute('data-theme')).toStrictEqual('dark');
});

test('the theme choice is read from the settings store and written back on change', () => {
  const fake = fakeSettingsStore('{"theme":"oled"}');
  const view = render(
    <Shell
      playback={stubPlayback().controller}
      path="/settings"
      widthPx={1600}
      library={demoLibrary()}
      settingsStore={fake.store}
    />,
  );
  const root = view.container.querySelector('#token-shell');
  expect(root?.getAttribute('data-theme')).toStrictEqual('oled');
  expect(root?.getAttribute('data-theme-choice')).toStrictEqual('oled');
  expect(screen.getByRole('radio', { name: 'OLED' }).getAttribute('aria-checked')).toStrictEqual('true');
  fireEvent.click(screen.getByRole('radio', { name: 'Light' }));
  expect(fake.value()).toStrictEqual('{"theme":"light"}');
  expect(root?.getAttribute('data-theme')).toStrictEqual('light');
});

test('a broken settings store reads as the system default', () => {
  const fake = fakeSettingsStore('not json at all');
  const view = render(
    <Shell playback={stubPlayback().controller} path="/" widthPx={1600} settingsStore={fake.store} />,
  );
  const root = view.container.querySelector('#token-shell');
  expect(root?.getAttribute('data-theme-choice')).toStrictEqual('system');
  expect(root?.getAttribute('data-theme')).toStrictEqual('dark');
});

test('nav links answer Enter and Space and ignore every other key', () => {
  const navigated: string[] = [];
  render(
    <Shell
      playback={stubPlayback().controller}
      path="/"
      widthPx={1600}
      onNavigate={(next) => {
        navigated.push(next);
      }}
    />,
  );
  const home = screen.getByRole('link', { name: 'Home' });
  fireEvent.keyDown(home, { key: ' ' });
  fireEvent.keyDown(home, { key: 'Enter' });
  fireEvent.keyDown(home, { key: 'Tab' });
  expect(navigated).toStrictEqual(['/', '/']);
});

test('the arrow keys seek the playing position through the transport listener', () => {
  const { calls, controller } = stubPlayback();
  render(<Shell playback={controller} path="/" widthPx={1600} />);
  fireEvent.keyDown(document, { key: 'ArrowRight' });
  fireEvent.keyDown(document, { key: 'ArrowLeft' });
  fireEvent.keyDown(document, { key: 'ArrowDown' });
  expect(calls).toStrictEqual(['seek', 'seek']);
});

test('opening an album from search pushes its address, and back returns to the library', () => {
  const navigated: string[] = [];
  render(
    <Shell
      playback={stubPlayback().controller}
      path="/search"
      widthPx={1600}
      library={demoLibrary()}
      searchLibrary={demoLocalFilter}
      onNavigate={(next) => {
        navigated.push(next);
      }}
    />,
  );
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), { target: { value: 'Harbour' } });
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(navigated).toStrictEqual(['/music/albums/harbour-lights']);
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(navigated).toStrictEqual(['/music/albums/harbour-lights', '/library']);
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
});

test('on the phone, playing a track row opens the full player with it', () => {
  const { calls, controller } = stubPlayback();
  render(<Shell playback={controller} path="/library" widthPx={390} library={demoLibrary()} />);
  fireEvent.click(screen.getByRole('tab', { name: 'Tracks' }));
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(calls).toStrictEqual(['playTrack', 'openFull']);
});

test('on the phone the spotlight opens the full player, and see-all and back push history', () => {
  const { calls, controller } = stubPlayback();
  const phone = render(<Shell playback={controller} path="/" widthPx={390} library={demoLibrary()} />);
  fireEvent.click(document.querySelector('#home-spotlight-play') as HTMLElement);
  expect(calls).toStrictEqual(['playAlbum', 'openFull']);
  phone.unmount();

  const desk = render(<Shell playback={controller} path="/" widthPx={1600} library={demoLibrary()} />);
  fireEvent.click(screen.getByRole('button', { name: 'See all' }));
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  // On wide a track row plays in place: the full player stays closed.
  fireEvent.click(screen.getByRole('tab', { name: 'Tracks' }));
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(calls).toStrictEqual(['playAlbum', 'openFull', 'playTrack']);
  desk.unmount();
});

test('the full player rides the shell when the controller has it open, and only with a track', () => {
  const queued = stubPlayback(queuedSnapshot());
  const openWithTrack = { ...queued.controller, fullOpen: true };
  const withTrack = render(<Shell playback={openWithTrack} path="/" widthPx={1600} />);
  expect(screen.getByRole('dialog', { name: 'Full player' }).id).toStrictEqual('player-full');
  withTrack.unmount();

  const empty = stubPlayback();
  const openWithoutTrack = { ...empty.controller, fullOpen: true };
  render(<Shell playback={openWithoutTrack} path="/" widthPx={1600} />);
  expect(screen.queryByRole('dialog', { name: 'Full player' })).toBeNull();
});

test('opening a path renders that destination from location.pathname', () => {
  window.history.pushState(null, '', '/library');
  const library = render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  expect(window.location.pathname).toStrictEqual('/library');
  library.unmount();

  window.history.pushState(null, '', '/search');
  const search = render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  expect(window.location.pathname).toStrictEqual('/search');
  search.unmount();

  window.history.pushState(null, '', '/');
  render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  expect(window.location.pathname).toStrictEqual('/');
});

test('a parent path does not hide the library the address already names', () => {
  window.history.pushState(null, '', '/library');
  const hidden = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} library={demoLibrary()} />);
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  expect(window.location.pathname).toStrictEqual('/library');
  act(() => {
    window.history.pushState({ scrollY: 0 }, '', '/search');
    window.dispatchEvent(new PopStateEvent('popstate', { state: { scrollY: 0 } }));
  });
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  hidden.unmount();

  window.history.pushState(null, '', '/nope');
  render(<Shell playback={stubPlayback().controller} path="/library" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
});

test('clicking Library, Search or Home writes that path so a reload stays there', () => {
  window.history.pushState(null, '', '/');
  const first = render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  fireEvent.click(screen.getByRole('link', { name: 'Library' }));
  expect(window.location.pathname).toStrictEqual('/library');
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  first.unmount();

  const reloaded = render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  expect(window.location.pathname).toStrictEqual('/library');
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('link', { name: 'Search' }));
  expect(window.location.pathname).toStrictEqual('/search');
  fireEvent.click(screen.getByRole('link', { name: 'Home' }));
  expect(window.location.pathname).toStrictEqual('/');
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  reloaded.unmount();
});

test('settings sections are not sidebar links until the current path is pinned', () => {
  window.history.pushState(null, '', '/');
  const { container } = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  const sidebar = container.querySelector('#nav-sidebar');
  expect(sidebarLabels(sidebar)).toStrictEqual(['Home', 'Search', 'Library', 'Store', 'Settings']);
  expect(container.querySelector('#nav-item-settings')?.getAttribute('data-selected')).toStrictEqual('0');
  expect(container.querySelector('#nav-item-settings-playback')).toBeNull();
  expect(screen.queryByRole('link', { name: 'Appearance' })).toBeNull();
  expect(screen.queryByRole('link', { name: 'Playback' })).toBeNull();
  expect(screen.queryByRole('link', { name: 'Extensions / Plugins' })).toBeNull();
  expect(screen.queryByRole('link', { name: 'About this connection' })).toBeNull();
  expect(screen.queryByRole('link', { name: 'Privacy' })).toBeNull();
  fireEvent.click(screen.getByRole('link', { name: 'Settings' }));
  expect(window.location.pathname).toStrictEqual('/settings/appearance');
  expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(sidebarLabels(sidebar)).toStrictEqual(['Home', 'Search', 'Library', 'Store', 'Settings', 'Appearance']);
  expect(container.querySelector('#nav-item-settings-appearance')?.getAttribute('data-selected')).toStrictEqual('1');
  expect(container.querySelector('#nav-item-settings-appearance')?.getAttribute('data-nav-glyph')).toStrictEqual(
    'settings-appearance',
  );
  expect(container.querySelector('#nav-item-settings')?.getAttribute('data-selected')).toStrictEqual('0');
  expect(screen.getByRole('button', { name: messages.unpinFromSidebar }).id).toStrictEqual('destination-pin');
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/settings/appearance","label":"Appearance"}]',
  );
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  expect(container.querySelector('#settings-appearance')).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: messages.unpinFromSidebar }));
  expect(sidebarLabels(sidebar)).toStrictEqual(['Home', 'Search', 'Library', 'Store', 'Settings']);
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual('[]');
  expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
});

test('opening /settings replaces the address with appearance and keeps a refused address', () => {
  window.history.pushState(null, '', '/settings');
  const settings = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  expect(window.location.pathname).toStrictEqual('/settings/appearance');
  expect(settings.container.querySelector('#nav-item-settings-appearance')).toBeNull();
  expect(settings.container.querySelector('#nav-item-settings')?.getAttribute('data-selected')).toStrictEqual('1');
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  expect(settings.container.querySelector('#settings-appearance')).not.toBeNull();
  expect(settings.container.querySelector('#settings-playback')).toBeNull();
  settings.unmount();

  window.history.pushState(null, '', '/settings?x=1');
  const query = render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  expect(window.location.pathname).toStrictEqual('/settings');
  expect(window.location.search).toStrictEqual('?x=1');
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  query.unmount();

  window.history.pushState(null, '', '/settings#x');
  render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  expect(window.location.pathname).toStrictEqual('/settings');
  expect(window.location.hash).toStrictEqual('#x');
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
});

test('a controlled path does not follow popstate away from itself', () => {
  window.history.pushState(null, '', '/');
  render(<Shell playback={stubPlayback().controller} path="/search" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  act(() => {
    window.history.pushState(null, '', '/library');
    window.dispatchEvent(new PopStateEvent('popstate', { state: null }));
  });
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
});

test('without callbacks the shell pushes history and follows popstate and resize', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  const view = render(<Shell playback={stubPlayback().controller} showDemoLabel />);
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('link', { name: 'Search' }));
  expect(window.location.pathname).toStrictEqual('/search');
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  // Theme switching lives in the Settings destination (tested with the real
  // library above); this shell has no library, so the switcher is not mounted.
  fireEvent.click(screen.getByRole('link', { name: 'Settings' }));
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('link', { name: 'Home' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('link', { name: 'Settings' }), { key: 'Tab' });
  act(() => {
    window.history.pushState({ scrollY: 0 }, '', '/library');
    window.dispatchEvent(new PopStateEvent('popstate', { state: { scrollY: 0 } }));
  });
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  act(() => {
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 360 });
    window.dispatchEvent(new Event('resize'));
  });
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-width')).toStrictEqual('compact');
  view.unmount();
});

test('controlled theme and width props update the shell attributes', () => {
  const view = render(<Shell playback={stubPlayback().controller} path="/" widthPx={800} theme="light" />);
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-theme')).toStrictEqual('light');
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-width')).toStrictEqual('medium');
  view.rerender(<Shell playback={stubPlayback().controller} path="/" widthPx={1200} theme="oled" />);
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-theme')).toStrictEqual('oled');
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-width')).toStrictEqual('expanded');
});

function focusableElements(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>('[tabindex]')].filter((element) => {
    const value = element.getAttribute('tabindex');
    return value !== null && Number(value) >= 0;
  });
}

test('nav items expose CSS glyph keys and the sidebar hosts the brand rule', () => {
  const { container } = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} showDemoLabel />);
  expect(container.querySelector('#nav-item-home')?.getAttribute('data-nav-glyph')).toStrictEqual('home');
  expect(container.querySelector('#nav-item-search')?.getAttribute('data-nav-glyph')).toStrictEqual('search');
  expect(container.querySelector('#nav-item-library')?.getAttribute('data-nav-glyph')).toStrictEqual('library');
  // Each nav item draws its glyph as an inline SVG icon ahead of its label.
  expect(
    ['home', 'search', 'library', 'store', 'settings'].map((key) => {
      const item = container.querySelector(`#nav-item-${key}`);
      return [item?.firstElementChild?.getAttribute('data-icon'), item?.textContent];
    }),
  ).toStrictEqual([
    ['home', 'Home'],
    ['search', 'Search'],
    ['library', 'Library'],
    ['store', 'Store'],
    ['settings', 'Settings'],
  ]);
  expect(container.querySelector('#nav-sidebar #shell-brand') !== null).toStrictEqual(true);
  expect(container.querySelector('#shell-brand-rule') !== null).toStrictEqual(true);
  expect(screen.getByText('Gunmetal').id).toStrictEqual('shell-wordmark');
  expect(screen.getByText('Fixture library').id).toStrictEqual('demo-label');
});

test('shell sets data-art-tone from cover while playing and clears it when paused', () => {
  const library = demoLibrary();
  const base = {
    trackId: 'demo-track-01-01',
    albumId: 'demo-album-01',
    title: 'Pier at Dusk',
    artistName: 'Mira Sol',
    coverTone: '01',
    coverUrl: '/media/covers/fixture.svg',
    mediaUrl: '/media/audio/fixtures.wav',
    playing: true,
    positionMs: 0,
    durationMs: 214_000,
    lyricsKind: 'none' as const,
    queue: [],
    queueOpen: false,
  };
  const playing = render(
    <Shell path="/" widthPx={1600} library={library} playback={stubPlayback({ ...base }).controller} />,
  );
  expect(playing.container.querySelector('#token-shell')?.getAttribute('data-art-tone')).toStrictEqual('01');
  playing.unmount();
  const paused = render(
    <Shell path="/" widthPx={1600} library={library} playback={stubPlayback({ ...base, playing: false }).controller} />,
  );
  expect(paused.container.querySelector('#token-shell')?.getAttribute('data-art-tone')).toBeNull();
});

test('skip links are the first focusable items and move focus to content and player', () => {
  const { container } = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} />);
  const root = container.querySelector('#token-shell') as HTMLElement;
  const skipContent = screen.getByRole('link', { name: 'Skip to content' });
  const skipPlayer = screen.getByRole('link', { name: 'Skip to player' });
  expect(skipContent.id).toStrictEqual('skip-to-content');
  expect(skipPlayer.id).toStrictEqual('skip-to-player');
  expect(focusableElements(root).slice(0, 2)).toStrictEqual([skipContent, skipPlayer]);

  const content = root.querySelector('#content') as HTMLElement;
  const player = root.querySelector('#player-bar') as HTMLElement;
  expect(content.getAttribute('tabindex')).toStrictEqual('-1');
  expect(player.getAttribute('tabindex')).toStrictEqual('-1');

  fireEvent.click(skipContent);
  expect(document.activeElement).toBe(content);
  fireEvent.click(skipPlayer);
  expect(document.activeElement).toBe(player);
  fireEvent.keyDown(skipContent, { key: 'Enter' });
  expect(document.activeElement).toBe(content);
  fireEvent.keyDown(skipPlayer, { key: ' ' });
  expect(document.activeElement).toBe(player);
  fireEvent.keyDown(skipContent, { key: 'Tab' });
  expect(document.activeElement).toBe(player);
  fireEvent.keyDown(skipPlayer, { key: 'Escape' });
  expect(document.activeElement).toBe(player);
});

test('queue line play actions route through the playback controller', () => {
  const queued = queuedSnapshot();
  const { calls, controller } = stubPlayback(queued);
  render(<Shell playback={controller} path="/" widthPx={1600} />);
  fireEvent.click(screen.getByRole('button', { name: 'Play Salt Window' }));
  expect(calls).toStrictEqual(['playTrack']);
});

function paneVars(root: Element | null): (string | undefined)[] {
  const style = (root as HTMLElement).style;
  return [style.getPropertyValue('--gm-sidebar-w'), style.getPropertyValue('--gm-queue-w')];
}

test('each resizable pane gets a separator, and only the panes the width shows', () => {
  const wide = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} />);
  expect(screen.getAllByRole('separator').map((node) => [node.id, node.getAttribute('aria-label')])).toStrictEqual([
    ['pane-resizer-sidebar', 'Resize sidebar'],
    ['pane-resizer-queue', 'Resize queue'],
  ]);
  // Unstored layout: the defaults, written to the shell root for the grid to read.
  expect(paneVars(wide.container.querySelector('#token-shell'))).toStrictEqual(['256px', '340px']);
  expect(screen.getByRole('separator', { name: 'Resize sidebar' }).getAttribute('aria-valuenow')).toStrictEqual('256');
  expect(screen.getByRole('separator', { name: 'Resize queue' }).getAttribute('aria-valuenow')).toStrictEqual('340');
  wide.unmount();

  render(<Shell playback={stubPlayback().controller} path="/" widthPx={1200} />);
  expect(screen.getAllByRole('separator').map((node) => node.id)).toStrictEqual(['pane-resizer-sidebar']);
  cleanup();

  render(<Shell playback={stubPlayback().controller} path="/" widthPx={800} />);
  expect(screen.queryAllByRole('separator')).toStrictEqual([]);
  cleanup();

  render(<Shell playback={stubPlayback().controller} path="/" widthPx={360} />);
  expect(screen.queryAllByRole('separator')).toStrictEqual([]);
});

test('a stored layout is applied on mount and every committed resize is written back', () => {
  const written: string[] = [];
  let reads = 0;
  const { container } = render(
    <Shell
      playback={stubPlayback().controller}
      path="/"
      widthPx={1600}
      layoutStore={{
        read: () => {
          reads += 1;
          return '{"sidebar":300,"queue":420}';
        },
        write: (value) => {
          written.push(value);
        },
      }}
    />,
  );
  const root = container.querySelector('#token-shell');
  expect(reads).toStrictEqual(1);
  expect(paneVars(root)).toStrictEqual(['300px', '420px']);
  const sidebar = screen.getByRole('separator', { name: 'Resize sidebar' });
  const queue = screen.getByRole('separator', { name: 'Resize queue' });
  expect(sidebar.getAttribute('aria-valuenow')).toStrictEqual('300');
  expect(queue.getAttribute('aria-valuenow')).toStrictEqual('420');

  // Keyboard: one step wider on each pane, each written with the other kept.
  fireEvent.keyDown(sidebar, { key: 'ArrowRight' });
  expect(paneVars(root)).toStrictEqual(['316px', '420px']);
  expect(sidebar.getAttribute('aria-valuenow')).toStrictEqual('316');
  fireEvent.keyDown(queue, { key: 'ArrowLeft' });
  expect(paneVars(root)).toStrictEqual(['316px', '436px']);
  expect(queue.getAttribute('aria-valuenow')).toStrictEqual('436');
  expect(written).toStrictEqual(['{"sidebar":316,"queue":420}', '{"sidebar":316,"queue":436}']);

  // Pointer: the grid follows every move, but only the release is stored.
  fireEvent.pointerDown(sidebar, { clientX: 316 });
  fireEvent.pointerMove(window, { clientX: 360 });
  expect(paneVars(root)).toStrictEqual(['360px', '436px']);
  expect(sidebar.getAttribute('aria-valuenow')).toStrictEqual('316');
  expect(written).toHaveLength(2);
  fireEvent.pointerUp(window, { clientX: 380 });
  expect(paneVars(root)).toStrictEqual(['380px', '436px']);
  expect(sidebar.getAttribute('aria-valuenow')).toStrictEqual('380');

  fireEvent.pointerDown(queue, { clientX: 1164 });
  fireEvent.pointerMove(window, { clientX: 1200 });
  expect(paneVars(root)).toStrictEqual(['380px', '400px']);
  fireEvent.pointerUp(window, { clientX: 1264 });
  expect(paneVars(root)).toStrictEqual(['380px', '336px']);

  // Double-click returns a pane to its default.
  fireEvent.doubleClick(sidebar);
  expect(paneVars(root)).toStrictEqual(['256px', '336px']);
  expect(written).toStrictEqual([
    '{"sidebar":316,"queue":420}',
    '{"sidebar":316,"queue":436}',
    '{"sidebar":380,"queue":436}',
    '{"sidebar":380,"queue":336}',
    '{"sidebar":256,"queue":336}',
  ]);
  // The store is read once, at mount; later renders trust the state.
  expect(reads).toStrictEqual(1);
});

test('a broken stored layout falls back to the defaults', () => {
  const { container } = render(
    <Shell
      playback={stubPlayback().controller}
      path="/"
      widthPx={1600}
      layoutStore={{ read: () => '<script>', write: () => undefined }}
    />,
  );
  expect(paneVars(container.querySelector('#token-shell'))).toStrictEqual(['256px', '340px']);
});

test('moving to another page or item returns the content pane to its top', () => {
  const { container } = render(
    <Shell playback={stubPlayback().controller} path="/" widthPx={1600} library={demoLibrary()} />,
  );
  const content = container.querySelector('#content') as HTMLElement;
  const tops: number[] = [];
  Object.defineProperty(content, 'scrollTop', {
    configurable: true,
    get: () => 640,
    set: (value: number) => {
      tops.push(value);
    },
  });
  // Opening an album from Home is a new page: back to the top.
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  expect(tops).toStrictEqual([0]);
  // Album to its artist: another item on the same route.
  fireEvent.click(screen.getByRole('button', { name: 'Go to artist' }));
  expect(screen.getByRole('heading', { name: 'Mira Sol' }).id).toStrictEqual('destination-headline');
  expect(tops).toStrictEqual([0, 0]);
  // A change that is not a navigation (a pane resize) leaves the scroll alone.
  fireEvent.keyDown(screen.getByRole('separator', { name: 'Resize sidebar' }), { key: 'ArrowRight' });
  expect(tops).toStrictEqual([0, 0]);
  fireEvent.click(screen.getByRole('link', { name: 'Search' }));
  expect(tops).toStrictEqual([0, 0, 0]);
});

test('home, library and search each offer Pin, and a pin of a default route is not listed twice', () => {
  const paths = ['/', '/library', '/search'] as const;
  const labels = ['Home', 'Library', 'Search'] as const;
  for (const [index, path] of paths.entries()) {
    window.localStorage.removeItem('gunmetal.pins');
    const view = render(
      <Shell playback={stubPlayback().controller} path={path} widthPx={1600} library={demoLibrary()} />,
    );
    expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
    fireEvent.keyDown(screen.getByRole('button', { name: messages.pinToSidebar }), { key: 'Tab' });
    expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
    fireEvent.keyDown(screen.getByRole('button', { name: messages.pinToSidebar }), { key: 'Enter' });
    expect(screen.getByRole('button', { name: messages.unpinFromSidebar }).id).toStrictEqual('destination-pin');
    expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
      `[{"path":"${path}","label":"${labels[index]}"}]`,
    );
    expect(sidebarLabels(view.container.querySelector('#nav-sidebar'))).toStrictEqual([
      'Home',
      'Search',
      'Library',
      'Store',
      'Settings',
    ]);
    fireEvent.keyDown(screen.getByRole('button', { name: messages.unpinFromSidebar }), { key: ' ' });
    expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
    expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual('[]');
    view.unmount();
  }
});

test('a pinned settings section is a sidebar link and navigates to that path', () => {
  window.localStorage.setItem(
    'gunmetal.pins',
    '[{"path":"/settings/privacy","label":"Privacy"},{"path":"/settings/playback","label":"Playback"}]',
  );
  const view = render(
    <Shell playback={stubPlayback().controller} path="/settings/about" widthPx={1600} library={demoLibrary()} />,
  );
  expect(sidebarLabels(view.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
    'Privacy',
    'Playback',
  ]);
  expect(view.container.querySelector('#nav-item-settings')?.getAttribute('data-selected')).toStrictEqual('1');
  expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(sidebarLabels(view.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
    'Privacy',
    'Playback',
    'About this connection',
  ]);
  expect(view.container.querySelector('#nav-item-settings-about')?.getAttribute('data-selected')).toStrictEqual('1');
  expect(view.container.querySelector('#nav-item-settings')?.getAttribute('data-selected')).toStrictEqual('0');
  fireEvent.click(screen.getByRole('link', { name: 'Playback' }));
  expect(window.location.pathname).toStrictEqual('/settings/playback');
  expect(view.container.querySelector('#settings-playback')).not.toBeNull();
  expect(view.container.querySelector('#nav-item-settings-playback')?.getAttribute('data-selected')).toStrictEqual('1');
  expect(screen.getByRole('button', { name: messages.unpinFromSidebar }).id).toStrictEqual('destination-pin');
  fireEvent.keyDown(screen.getByRole('link', { name: 'Privacy' }), { key: 'Enter' });
  expect(window.location.pathname).toStrictEqual('/settings/privacy');
  expect(view.container.querySelector('#settings-privacy')).not.toBeNull();
});

test('the store door opens the catalogue and a pin of it is labeled Store', () => {
  window.history.pushState(null, '', '/');
  const home = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  fireEvent.click(screen.getByRole('link', { name: 'Store' }));
  expect(window.location.pathname).toStrictEqual('/store');
  expect(screen.getByRole('heading', { name: 'Store' }).id).toStrictEqual('destination-headline');
  expect(home.container.querySelector('#destination-store')).not.toBeNull();
  expect(home.container.querySelector('#destination-settings')).toBeNull();
  expect(home.container.querySelector('[data-store-card="cover-art"]')?.textContent).toContain('On this server');
  fireEvent.click(screen.getByRole('button', { name: 'Metadata and artwork' }));
  expect(window.location.pathname).toStrictEqual('/store/cover-art');
  expect(home.container.querySelector('#destination-store')).not.toBeNull();
  expect(home.container.querySelector('#destination-settings')).toBeNull();
  expect(screen.getByRole('button', { name: 'Back to store' })).not.toBeNull();
  home.unmount();

  window.history.pushState(null, '', '/store');
  const pinned = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual('[{"path":"/store","label":"Store"}]');
  expect(sidebarLabels(pinned.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
  ]);
  pinned.unmount();
});

test('pinning a store record stores it under its catalogue label', () => {
  window.history.pushState(null, '', '/');
  render(<Shell playback={stubPlayback().controller} path="/store/cover-art" widthPx={1600} library={demoLibrary()} />);
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(screen.getByRole('link', { name: 'Metadata and artwork' })).not.toBeNull();
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/store/cover-art","label":"Metadata and artwork"}]',
  );
});

test('pins survive a remount from localStorage and an injected store is not the layout document', () => {
  window.localStorage.setItem('gunmetal.pins', '[{"path":"/settings/playback","label":"Playback"}]');
  const first = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} />);
  expect(screen.getByRole('link', { name: 'Playback' })).not.toBeNull();
  first.unmount();

  const written: string[] = [];
  let pinsRaw: string | null = null;
  const view = render(
    <Shell
      playback={stubPlayback().controller}
      path="/settings/playback"
      widthPx={1600}
      library={demoLibrary()}
      layoutStore={{
        read: () => '{"sidebar":300,"queue":420}',
        write: (value) => {
          written.push(value);
        },
      }}
      pinsStore={{
        read: () => pinsRaw,
        write: (value) => {
          pinsRaw = value;
        },
      }}
    />,
  );
  expect(screen.queryByRole('link', { name: 'Playback' })).toBeNull();
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/settings/playback","label":"Playback"}]',
  );
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(pinsRaw).toStrictEqual('[{"path":"/settings/playback","label":"Playback"}]');
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/settings/playback","label":"Playback"}]',
  );
  expect(written).toStrictEqual([]);
  expect(screen.getByRole('link', { name: 'Playback' })).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: messages.unpinFromSidebar }));
  expect(pinsRaw).toStrictEqual('[]');
  expect(screen.queryByRole('link', { name: 'Playback' })).toBeNull();
  view.unmount();
});

test('a broken pin store reads as no pins and a blocked write still toggles for this visit', () => {
  const real = window.localStorage;
  const blocked = {
    getItem(): string | null {
      throw new Error('blocked');
    },
    setItem(): void {
      throw new Error('full');
    },
    removeItem(): void {
      throw new Error('blocked');
    },
    clear(): void {},
    key(): string | null {
      return null;
    },
    length: 0,
  };
  Object.defineProperty(window, 'localStorage', { configurable: true, value: blocked });
  try {
    const view = render(<Shell playback={stubPlayback().controller} path="/settings/privacy" widthPx={1600} />);
    expect(screen.queryByRole('link', { name: 'Privacy' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
    expect(screen.getByRole('link', { name: 'Privacy' })).not.toBeNull();
    expect(screen.getByRole('button', { name: messages.unpinFromSidebar })).not.toBeNull();
    view.unmount();
  } finally {
    Object.defineProperty(window, 'localStorage', { configurable: true, value: real });
  }
});

test('pinning an album or artist adds that page to the sidebar and opens it again', () => {
  window.history.pushState(null, '', '/');
  const view = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  expect(sidebarLabels(view.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
  ]);
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(sidebarLabels(view.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
    'Harbour Lights',
  ]);
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/music/albums/harbour-lights","label":"Harbour Lights"}]',
  );
  expect(
    view.container.querySelector('#nav-item-music-albums-harbour-lights')?.getAttribute('data-selected'),
  ).toStrictEqual('1');
  expect(view.container.querySelector('#nav-item-library')?.getAttribute('data-selected')).toStrictEqual('0');
  expect(screen.getByRole('button', { name: messages.unpinFromSidebar }).id).toStrictEqual('destination-pin');

  fireEvent.click(screen.getByRole('link', { name: 'Search' }));
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('link', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  expect(window.location.pathname).toStrictEqual('/music/albums/harbour-lights');

  fireEvent.click(screen.getByRole('button', { name: 'Go to artist' }));
  expect(screen.getByRole('heading', { name: 'Mira Sol' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(sidebarLabels(view.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
    'Harbour Lights',
    'Mira Sol',
  ]);
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/music/albums/harbour-lights","label":"Harbour Lights"},{"path":"/music/artists/mira-sol","label":"Mira Sol"}]',
  );

  fireEvent.click(screen.getByRole('link', { name: 'Home' }));
  fireEvent.keyDown(screen.getByRole('link', { name: 'Mira Sol' }), { key: 'Enter' });
  expect(screen.getByRole('heading', { name: 'Mira Sol' }).id).toStrictEqual('destination-headline');
  expect(view.container.querySelector('#nav-item-music-artists-mira-sol')?.getAttribute('data-selected')).toStrictEqual(
    '1',
  );

  fireEvent.click(screen.getByRole('button', { name: messages.unpinFromSidebar }));
  expect(sidebarLabels(view.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
    'Harbour Lights',
  ]);
  expect(screen.queryByRole('link', { name: 'Mira Sol' })).toBeNull();
  expect(screen.getByRole('button', { name: messages.pinToSidebar }).id).toStrictEqual('destination-pin');
});

test('a stored album pin is a sidebar link that opens that album', () => {
  window.localStorage.setItem('gunmetal.pins', '[{"path":"/library","label":"Night Shift","itemId":"demo-album-02"}]');
  const view = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} library={demoLibrary()} />);
  expect(sidebarLabels(view.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
    'Night Shift',
  ]);
  fireEvent.click(screen.getByRole('link', { name: 'Night Shift' }));
  expect(screen.getByRole('heading', { name: 'Night Shift' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('button', { name: messages.unpinFromSidebar }).id).toStrictEqual('destination-pin');
});

test('connected services and an extension page pin to the sidebar under their own names', () => {
  window.history.pushState(null, '', '/settings/connected');
  const connected = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(sidebarLabels(connected.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
    'Connected services',
  ]);
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/settings/connected","label":"Connected services"}]',
  );
  fireEvent.click(screen.getByRole('link', { name: 'Connected services' }));
  expect(window.location.pathname).toStrictEqual('/settings/connected');
  connected.unmount();
  window.localStorage.removeItem('gunmetal.pins');

  window.history.pushState(null, '', '/store/url-style');
  const extension = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(screen.getByRole('link', { name: 'Address style' })).not.toBeNull();
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/store/url-style","label":"Address style"}]',
  );
  extension.unmount();
});

test('a stored pin that is not a pin is dropped, and not-found has no pin button', () => {
  window.localStorage.setItem('gunmetal.pins', 'not json');
  const broken = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} />);
  expect(sidebarLabels(broken.container.querySelector('#nav-sidebar'))).toStrictEqual([
    'Home',
    'Search',
    'Library',
    'Store',
    'Settings',
  ]);
  broken.unmount();

  render(<Shell playback={stubPlayback().controller} path="/nope" widthPx={1600} />);
  expect(screen.queryByRole('button', { name: messages.pinToSidebar })).toBeNull();
  expect(screen.queryByRole('button', { name: messages.unpinFromSidebar })).toBeNull();
});

function folderLibrary() {
  const album = 'a'.repeat(16);
  const other = 'd'.repeat(16);
  const artist = 'b'.repeat(16);
  const track = 'c'.repeat(16);
  return {
    kind: 'folder' as const,
    albums: [
      {
        id: album,
        title: 'St. Elsewhere',
        artistName: 'Gnarls Barkley',
        artistKey: artist,
        year: 2006,
        coverTone: '01',
        coverUrl: `/media/library/covers/${album}.jpg`,
        discs: [{ index: 1, title: '' }],
        hostile: false,
        tracks: [
          {
            id: track,
            albumId: album,
            discIndex: 1,
            number: 1,
            title: 'Crazy',
            artistName: 'Gnarls Barkley',
            durationMs: 178_000,
            flag: 'ok' as const,
            lyricsKind: 'none' as const,
            mediaUrl: `/media/library/${track}`,
          },
        ],
      },
      {
        id: other,
        title: 'The Odd Couple',
        artistName: 'Gnarls Barkley',
        artistKey: artist,
        year: 2008,
        coverTone: '02',
        coverUrl: `/media/library/covers/${other}.jpg`,
        discs: [{ index: 1, title: '' }],
        hostile: false,
        tracks: [
          {
            id: 'e'.repeat(16),
            albumId: other,
            discIndex: 1,
            number: 1,
            title: 'Going On',
            artistName: 'Gnarls Barkley',
            durationMs: 180_000,
            flag: 'ok' as const,
            lyricsKind: 'none' as const,
            mediaUrl: `/media/library/${'e'.repeat(16)}`,
          },
        ],
      },
    ],
    artists: [{ key: artist, name: 'Gnarls Barkley', albumIds: [album, other] }],
  };
}

test('a folder library hides the demo label, serves search, and states the library size', () => {
  const folder = folderLibrary();
  const labeled = render(
    <Shell playback={stubPlayback().controller} path="/" widthPx={1600} showDemoLabel library={folder} />,
  );
  expect(labeled.container.querySelector('#demo-label')).toBeNull();
  expect(screen.queryByText('Fixture library')).toBeNull();
  labeled.unmount();

  const fixtures = render(
    <Shell playback={stubPlayback().controller} path="/" widthPx={1600} showDemoLabel library={demoLibrary()} />,
  );
  expect(screen.getByText('Fixture library').id).toStrictEqual('demo-label');
  fixtures.unmount();

  const search = render(
    <Shell
      playback={stubPlayback().controller}
      path="/search"
      widthPx={1600}
      library={folder}
      searchLibrary={demoLocalFilter}
    />,
  );
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), { target: { value: 'Elsewhere' } });
  expect(search.container.querySelector('#search-demo-notice')).toBeNull();
  expect(search.container.querySelector('#search-plugin-notice')?.textContent).toStrictEqual(
    'A signed catalog extension can find releases outside this library. None is loaded.',
  );
  search.unmount();

  const demoSearch = render(
    <Shell
      playback={stubPlayback().controller}
      path="/search"
      widthPx={1600}
      library={demoLibrary()}
      searchLibrary={demoLocalFilter}
    />,
  );
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), { target: { value: 'Harbour' } });
  expect(demoSearch.container.querySelector('#search-demo-notice')?.textContent).toStrictEqual('Search this library');
  demoSearch.unmount();

  const about = render(
    <Shell playback={stubPlayback().controller} path="/settings/about" widthPx={1600} library={folder} />,
  );
  expect(about.container.querySelector('[data-settings-fact="data"] [data-fact-value]')?.textContent).toStrictEqual(
    '2 albums · 1 artists',
  );
  about.unmount();

  const playback = render(
    <Shell playback={stubPlayback().controller} path="/settings/playback" widthPx={1600} library={folder} />,
  );
  expect(playback.container.querySelector('[data-settings-placeholder="playback"]')).toBeNull();
  playback.unmount();

  const demoAbout = render(
    <Shell playback={stubPlayback().controller} path="/settings/about" widthPx={1600} library={demoLibrary()} />,
  );
  expect(demoAbout.container.querySelector('[data-settings-fact="data"] [data-fact-value]')?.textContent).toStrictEqual(
    'This library',
  );
  demoAbout.unmount();

  const demoPlayback = render(
    <Shell playback={stubPlayback().controller} path="/settings/playback" widthPx={1600} library={demoLibrary()} />,
  );
  expect(demoPlayback.container.querySelector('[data-settings-placeholder="playback"]')?.textContent).toStrictEqual(
    "Playback uses this server's files.",
  );
});

test('with nothing loaded, play in the bar starts nothing', () => {
  const { calls, controller } = stubPlayback();
  render(<Shell playback={controller} path="/" widthPx={1600} library={demoLibrary()} />);
  const play = document.querySelector('#shell-play') as HTMLElement;
  expect(play.getAttribute('aria-label')).toStrictEqual('Play');
  expect(play.getAttribute('data-disabled')).toStrictEqual('1');
  fireEvent.click(play);
  fireEvent.keyDown(play, { key: 'Enter' });
  fireEvent.keyDown(play, { key: ' ' });
  expect(calls).toStrictEqual([]);
  // The hero's own play is the way to start something.
  fireEvent.click(document.querySelector('#home-spotlight-play') as HTMLElement);
  expect(calls).toStrictEqual(['playAlbum']);
});

test('an id address rewrites to the slug, and a slug or a missing watch address stays', () => {
  window.history.pushState(null, '', '/music/albums/demo-album-01');
  const id = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  expect(window.location.pathname).toStrictEqual('/music/albums/harbour-lights');
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  id.unmount();

  window.history.pushState(null, '', '/music/albums/harbour-lights');
  const slug = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  expect(window.location.pathname).toStrictEqual('/music/albums/harbour-lights');
  slug.unmount();

  window.history.pushState(null, '', '/watch/movies/inception');
  const movie = render(<Shell playback={stubPlayback().controller} widthPx={1600} library={demoLibrary()} />);
  expect(window.location.pathname).toStrictEqual('/watch/movies/inception');
  expect(screen.getByRole('heading', { name: 'Inception' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/watch/movies/inception","label":"Inception"}]',
  );
  movie.unmount();

  window.history.pushState(null, '', '/watch/shows/the-wire');
  window.localStorage.removeItem('gunmetal.pins');
  render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  expect(window.location.pathname).toStrictEqual('/watch/shows/the-wire');
  expect(screen.getByRole('heading', { name: 'The Wire' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: messages.pinToSidebar }));
  expect(window.localStorage.getItem('gunmetal.pins')).toStrictEqual(
    '[{"path":"/watch/shows/the-wire","label":"The Wire"}]',
  );
});

test('a stored item pin opens that album, and an unknown item falls back to the library', () => {
  window.localStorage.setItem('gunmetal.pins', '[{"path":"/library","label":"Missing","itemId":"missing-album"}]');
  const missing = render(
    <Shell playback={stubPlayback().controller} path="/search" widthPx={1600} library={demoLibrary()} />,
  );
  fireEvent.click(screen.getByRole('link', { name: 'Missing' }));
  expect(window.location.pathname).toStrictEqual('/library');
  expect(screen.getByRole('heading', { name: 'That album is not in the demo library' }).id).toStrictEqual(
    'destination-headline',
  );
  missing.unmount();

  window.history.pushState(null, '', '/search');
  window.localStorage.setItem('gunmetal.pins', '[{"path":"/library","label":"Night","itemId":"demo-album-02"}]');
  render(<Shell playback={stubPlayback().controller} widthPx={1600} />);
  fireEvent.click(screen.getByRole('link', { name: 'Night' }));
  expect(window.location.pathname).toStrictEqual('/library');
});
