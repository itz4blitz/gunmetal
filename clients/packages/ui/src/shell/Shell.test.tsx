import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { stubPlayback, queuedSnapshot } from './test-playback.ts';
import { demoLibrary } from '../../../fake-server/src/catalogue.ts';
import { landmarks } from './width.ts';
import { Shell } from './Shell.tsx';

afterEach(cleanup);

function landmarkIds(root: HTMLElement): string[] {
  const order = ['nav-sidebar', 'nav-rail', 'content', 'right-pane', 'player-bar', 'nav-tabs'];
  return order.filter((id) => root.querySelector(`#${id}`) !== null);
}

test('wide shell landmarks match the literal list and show Home', () => {
  const { container } = render(<Shell playback={stubPlayback().controller} path="/" widthPx={1600} showDemoLabel />);
  const root = container.querySelector('#token-shell');
  expect(root?.getAttribute('data-width')).toStrictEqual('wide');
  expect(root?.getAttribute('data-theme')).toStrictEqual('dark');
  expect(landmarkIds(root as HTMLElement)).toStrictEqual([...landmarks(1600)]);
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  expect(screen.getByText('Demo data').id).toStrictEqual('demo-label');
  expect(screen.getByRole('navigation', { name: 'Primary' }).id).toStrictEqual('nav-sidebar');
  expect(screen.getByRole('complementary', { name: 'Queue' }).id).toStrictEqual('right-pane');
  expect(screen.getByRole('region', { name: 'Now playing' }).id).toStrictEqual('player-bar');
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
  expect(navigated).toStrictEqual(['/search', '/library', '/', '/settings', '/search', '/library', '/settings']);
  // The sidebar no longer hosts a theme stack; the switcher lives in Settings.
  expect(screen.queryByRole('button', { name: 'Light' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'OLED' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'High contrast' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Dark' })).toBeNull();
  expect(themes).toStrictEqual([]);
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
    ['home', 'search', 'library', 'settings'].map((key) => {
      const item = container.querySelector(`#nav-item-${key}`);
      return [item?.firstElementChild?.getAttribute('data-icon'), item?.textContent];
    }),
  ).toStrictEqual([
    ['home', 'Home'],
    ['search', 'Search'],
    ['library', 'Library'],
    ['settings', 'Settings'],
  ]);
  expect(container.querySelector('#nav-sidebar #shell-brand')).toBeTruthy();
  expect(container.querySelector('#shell-brand-rule')).toBeTruthy();
  expect(screen.getByText('Gunmetal').id).toStrictEqual('shell-wordmark');
  expect(screen.getByText('Demo data').id).toStrictEqual('demo-label');
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
