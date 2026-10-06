import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { landmarks } from './width.ts';
import { Shell } from './Shell.tsx';

afterEach(cleanup);

function landmarkIds(root: HTMLElement): string[] {
  const order = ['nav-sidebar', 'nav-rail', 'content', 'right-pane', 'player-bar', 'nav-tabs'];
  return order.filter((id) => root.querySelector(`#${id}`) !== null);
}

test('wide shell landmarks match the literal list and show Home', () => {
  const { container } = render(<Shell path="/" widthPx={1600} showDemoLabel />);
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
  const compact = render(<Shell path="/search" widthPx={360} />);
  expect(landmarkIds(compact.container.querySelector('#token-shell') as HTMLElement)).toStrictEqual([
    ...landmarks(360),
  ]);
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('navigation', { name: 'Primary' }).id).toStrictEqual('nav-tabs');
  compact.unmount();

  const medium = render(<Shell path="/library" widthPx={800} />);
  expect(landmarkIds(medium.container.querySelector('#token-shell') as HTMLElement)).toStrictEqual([
    ...landmarks(800),
  ]);
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('navigation', { name: 'Primary' }).id).toStrictEqual('nav-rail');
  medium.unmount();

  const expanded = render(<Shell path="/settings" widthPx={1200} />);
  expect(landmarkIds(expanded.container.querySelector('#token-shell') as HTMLElement)).toStrictEqual([
    ...landmarks(1200),
  ]);
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('navigation', { name: 'Primary' }).id).toStrictEqual('nav-sidebar');
  expanded.unmount();
});

test('an unknown path, fragment and query each show Not found', () => {
  const unknown = render(<Shell path="/nope" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  unknown.unmount();

  const fragment = render(<Shell path="/" hash="#x" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  fragment.unmount();

  const query = render(<Shell path="/search" search="?q=1" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  query.unmount();

  const badId = render(<Shell path="/" historyState={{ itemId: 'a/b' }} widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  badId.unmount();
});

test('nav links and theme buttons call the composition callbacks', () => {
  const navigated: string[] = [];
  const themes: string[] = [];
  render(
    <Shell
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
  fireEvent.click(screen.getByRole('button', { name: 'Light' }));
  fireEvent.click(screen.getByRole('button', { name: 'OLED' }));
  fireEvent.click(screen.getByRole('button', { name: 'High contrast' }));
  fireEvent.click(screen.getByRole('button', { name: 'Dark' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Light' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'OLED' }), { key: ' ' });
  expect(navigated).toStrictEqual(['/search', '/library', '/', '/settings', '/search', '/library', '/settings']);
  expect(themes).toStrictEqual(['light', 'oled', 'high-contrast', 'dark', 'light', 'oled']);
});

test('without callbacks the shell pushes history and follows popstate and resize', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  const view = render(<Shell showDemoLabel />);
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('link', { name: 'Search' }));
  expect(window.location.pathname).toStrictEqual('/search');
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: 'Light' }));
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-theme')).toStrictEqual('light');
  fireEvent.keyDown(screen.getByRole('link', { name: 'Home' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Dark' }), { key: 'Tab' });
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
  const view = render(<Shell path="/" widthPx={800} theme="light" />);
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-theme')).toStrictEqual('light');
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-width')).toStrictEqual('medium');
  view.rerender(<Shell path="/" widthPx={1200} theme="oled" />);
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-theme')).toStrictEqual('oled');
  expect(view.container.querySelector('#token-shell')?.getAttribute('data-width')).toStrictEqual('expanded');
});

function focusableElements(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>('[tabindex]')].filter((element) => {
    const value = element.getAttribute('tabindex');
    return value !== null && Number(value) >= 0;
  });
}

test('skip links are the first focusable items and move focus to content and player', () => {
  const { container } = render(<Shell path="/" widthPx={1600} />);
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
