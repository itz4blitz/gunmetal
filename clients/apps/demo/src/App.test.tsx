import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { App } from './App.tsx';

afterEach(cleanup);

test('the demo app mounts the shell with fixture home rows and demo data', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  expect(screen.getByText('Gunmetal').id).toStrictEqual('shell-wordmark');
  expect(screen.getByText('Demo data').id).toStrictEqual('demo-label');
  expect(document.querySelector('#destination-headline')).toBeNull();
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(document.querySelector('#home-spotlight')).toBeTruthy();
  expect(document.querySelector('#home-spotlight [data-spotlight-eyebrow="1"]')?.textContent).toStrictEqual('Featured');
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Recently added' })).toBeTruthy();
  // History rows stay hidden until plays and loves exist (C2): no placeholder
  // cards on the first screen.
  expect(screen.queryByRole('heading', { name: 'Recently played' })).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Continue listening' })).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Loved' })).toBeNull();
  expect(screen.queryByText('Nothing played yet')).toBeNull();
  expect(screen.getByRole('button', { name: 'Harbour Lights' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'See all' })).toBeTruthy();
});

test('playing a fixture album fills the player bar from demo-local state', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
  expect(document.querySelector('#player-artist')?.textContent).toStrictEqual('Mira Sol · Harbour Lights');
  expect(document.querySelector('#shell-play')?.getAttribute('aria-label')).toStrictEqual('Pause');
  expect(document.querySelector('#shell-play')?.getAttribute('data-playing')).toStrictEqual('1');
  expect(document.querySelector('#queue-line-demo-track-01-01')).toBeTruthy();
  expect(document.querySelector('#player-full')).toBeNull();
  // The album merged into the credit line: artist · album.
  expect(document.querySelector('#player-album')).toBeNull();
  expect(document.querySelector('#nav-sidebar')).toBeTruthy();
  expect(document.querySelector('#player-bar')).toBeTruthy();
});

test('album page play fills the bar and history returns to it', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
  expect(document.querySelector('#shell-play')?.getAttribute('aria-label')).toStrictEqual('Pause');
  // The queue pane carries the album in fixture order.
  expect(document.querySelectorAll('#queue-list [data-queue-line]').length).toBeGreaterThanOrEqual(4);
});

test('skip advances to the next queue line and pause holds position', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  fireEvent.click(screen.getByRole('button', { name: 'Next' }));
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Salt Window');
  fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
  expect(document.querySelector('#shell-play')?.getAttribute('aria-label')).toStrictEqual('Play');
});

test('context menus play next and add to queue without shuffling', () => {
  window.history.pushState(null, '', '/library');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(document.querySelector('#album-tile-demo-album-01 [data-album-play]')!);
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  const before = [...document.querySelectorAll('#queue-list [data-queue-line]')].map((node) => node.id);
  fireEvent.contextMenu(document.querySelector('#album-tile-demo-album-02')!);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  const after = [...document.querySelectorAll('#queue-list [data-queue-line]')].map((node) => node.id);
  // Night Shift lands right after the current track, in its own order.
  expect(after.slice(0, 1 + 2)).toStrictEqual([
    before[0],
    'queue-line-demo-track-02-01',
    'queue-line-demo-track-02-02',
  ]);
});

test('compact play opens the overlay full player while wide play stays in the bar', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 390 });
  const compact = render(<App />);
  fireEvent.click(document.querySelector('#home-spotlight-play')!);
  expect(document.querySelector('#player-full')).toBeTruthy();
  compact.unmount();
  cleanup();
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(document.querySelector('#home-spotlight-play')!);
  // Wide keeps the full player closed: the bar carries playback.
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
});
