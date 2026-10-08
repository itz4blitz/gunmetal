import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';
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
  expect(document.querySelector('#home-spotlight')).not.toBeNull();
  expect(document.querySelector('#home-spotlight [data-spotlight-eyebrow="1"]')?.textContent).toStrictEqual('Featured');
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).not.toBeNull();
  expect(screen.getByRole('heading', { name: 'Recently added' })).not.toBeNull();
  // History rows stay hidden until plays and loves exist (C2): no placeholder
  // cards on the first screen.
  expect(screen.queryByRole('heading', { name: 'Recently played' })).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Continue listening' })).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Loved' })).toBeNull();
  expect(screen.queryByText('Nothing played yet')).toBeNull();
  expect(screen.getByRole('button', { name: 'Harbour Lights' })).not.toBeNull();
  expect(screen.getByRole('button', { name: 'See all' })).not.toBeNull();
});

test('a folder library replaces the fixture home and drops the demo label', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  const album = 'a'.repeat(16);
  const artist = 'b'.repeat(16);
  const track = 'c'.repeat(16);
  const library: DemoLibrary = {
    kind: 'folder',
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
            flag: 'ok',
            lyricsKind: 'none',
            mediaUrl: `/media/library/${track}`,
          },
        ],
      },
    ],
    artists: [{ key: artist, name: 'Gnarls Barkley', albumIds: [album] }],
  };
  render(<App library={library} />);
  expect(screen.queryByText('Demo data')).toBeNull();
  expect(screen.getByRole('heading', { name: 'St. Elsewhere' })).not.toBeNull();
  expect(screen.queryByRole('heading', { name: 'Harbour Lights' })).toBeNull();
});

test('playing a fixture album fills the player bar from demo-local state', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  fireEvent.click(document.querySelector('#album-play') as HTMLElement);
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
  expect(document.querySelector('#player-artist')?.textContent).toStrictEqual('Mira Sol · Harbour Lights');
  expect(document.querySelector('#shell-play')?.getAttribute('aria-label')).toStrictEqual('Pause');
  expect(document.querySelector('#shell-play')?.getAttribute('data-playing')).toStrictEqual('1');
  expect(document.querySelector('#queue-line-demo-track-01-01')).not.toBeNull();
  expect(document.querySelector('#player-full')).toBeNull();
  // The album merged into the credit line: artist · album.
  expect(document.querySelector('#player-album')).toBeNull();
  expect(document.querySelector('#nav-sidebar')).not.toBeNull();
  expect(document.querySelector('#player-bar')).not.toBeNull();
});

test('album page play fills the bar and history returns to it', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  fireEvent.click(document.querySelector('#album-play') as HTMLElement);
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
  fireEvent.click(document.querySelector('#album-play') as HTMLElement);
  fireEvent.click(screen.getByRole('button', { name: 'Next' }));
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Salt Window');
  fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
  expect(document.querySelector('#shell-play')?.getAttribute('aria-label')).toStrictEqual('Play');
});

test('context menus play next and add to queue without shuffling', () => {
  window.history.pushState(null, '', '/library');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(document.querySelector('#album-tile-demo-album-01 [data-album-play]') as HTMLElement);
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  const before = [...document.querySelectorAll('#queue-list [data-queue-line]')].map((node) => node.id);
  fireEvent.contextMenu(document.querySelector('#album-tile-demo-album-02') as HTMLElement);
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
  fireEvent.click(document.querySelector('#home-spotlight-play') as HTMLElement);
  expect(document.querySelector('#player-full')).not.toBeNull();
  compact.unmount();
  cleanup();
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(document.querySelector('#home-spotlight-play') as HTMLElement);
  // Wide keeps the full player closed: the bar carries playback.
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
});

test('pane widths persist in localStorage and come back on the next visit', () => {
  window.localStorage.clear();
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  const first = render(<App />);
  const shellStyle = () => (document.querySelector('#token-shell') as HTMLElement).style;
  expect(shellStyle().getPropertyValue('--gm-sidebar-w')).toStrictEqual('256px');
  expect(window.localStorage.getItem('gunmetal.layout.v1')).toStrictEqual(null);
  fireEvent.keyDown(screen.getByRole('separator', { name: 'Resize sidebar' }), { key: 'ArrowRight', shiftKey: true });
  fireEvent.keyDown(screen.getByRole('separator', { name: 'Resize queue' }), { key: 'End' });
  expect(window.localStorage.getItem('gunmetal.layout.v1')).toStrictEqual('{"sidebar":304,"queue":560}');
  first.unmount();

  render(<App />);
  expect(shellStyle().getPropertyValue('--gm-sidebar-w')).toStrictEqual('304px');
  expect(shellStyle().getPropertyValue('--gm-queue-w')).toStrictEqual('560px');
  // Only layout is stored: one key, two pixel counts.
  expect(window.localStorage.length).toStrictEqual(1);
  window.localStorage.clear();
});

function extensionJobRows(): (string | null)[][] {
  return [...document.querySelectorAll('#settings-plugin-slots [data-settings-row]')].map((row) => [
    row.getAttribute('data-settings-row'),
    row.getAttribute('data-job-status'),
    row.querySelector('[data-slot-title]')?.textContent ?? null,
    row.querySelector('[data-job-detail]')?.textContent ?? null,
    row.querySelector('[data-settings-row-status]')?.textContent ?? null,
  ]);
}

test('settings extensions lists first-party jobs, and fixture art is not Cover Art Archive on', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(screen.getByRole('link', { name: 'Settings' }));
  fireEvent.click(screen.getByRole('tab', { name: 'Extensions / Plugins' }));
  expect(document.querySelector('#settings-extensions')).not.toBeNull();
  expect(document.querySelector('#settings-extensions [data-settings-badge]')).toBeNull();
  expect(document.querySelector('[data-slot-version]')).toBeNull();
  expect(extensionJobRows()).toStrictEqual([
    [
      'metadata-provider',
      'not-serving',
      'Metadata and artwork',
      'Built into this library host. Cover Art Archive.',
      null,
    ],
    ['lyrics-provider', 'not-in-build', 'Lyrics lookup', null, 'Not in this build'],
    ['search-provider', 'not-in-build', 'Catalogue search', null, 'Not in this build'],
    ['scrobbler', 'not-in-build', 'Scrobblers', null, 'Not in this build'],
    [
      'theme-pack',
      'not-a-plugin',
      'Themes',
      'Not a separate plugin. Themes are the settings appearance control.',
      null,
    ],
    ['home-row', 'not-a-plugin', 'Home rows', 'Not a separate plugin.', null],
  ]);
});

test('settings extensions says Cover Art Archive is on when the library host serves covers', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  const album = 'a'.repeat(16);
  const artist = 'b'.repeat(16);
  const track = 'c'.repeat(16);
  const library: DemoLibrary = {
    kind: 'folder',
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
            flag: 'ok',
            lyricsKind: 'none',
            mediaUrl: `/media/library/${track}`,
          },
        ],
      },
    ],
    artists: [{ key: artist, name: 'Gnarls Barkley', albumIds: [album] }],
  };
  render(<App library={library} />);
  fireEvent.click(screen.getByRole('link', { name: 'Settings' }));
  fireEvent.click(screen.getByRole('tab', { name: 'Extensions / Plugins' }));
  expect(extensionJobRows()).toStrictEqual([
    ['metadata-provider', 'on', 'Metadata and artwork', 'Built into this library host. Cover Art Archive.', 'On'],
    ['lyrics-provider', 'not-in-build', 'Lyrics lookup', null, 'Not in this build'],
    ['search-provider', 'not-in-build', 'Catalogue search', null, 'Not in this build'],
    ['scrobbler', 'not-in-build', 'Scrobblers', null, 'Not in this build'],
    [
      'theme-pack',
      'not-a-plugin',
      'Themes',
      'Not a separate plugin. Themes are the settings appearance control.',
      null,
    ],
    ['home-row', 'not-a-plugin', 'Home rows', 'Not a separate plugin.', null],
  ]);
  expect(document.querySelector('#settings-extensions')?.textContent?.includes('Not loaded')).toStrictEqual(false);
  expect(document.querySelector('#settings-extensions')?.textContent?.includes('1.0.0')).toStrictEqual(false);
  expect(document.querySelector('#settings-extensions [data-settings-badge]')).toBeNull();
});

test('playing never opens the queue sheet; the queue control does, and it stays open across plays', () => {
  window.localStorage.clear();
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1100 });
  render(<App />);
  const sheetOpen = () => document.querySelector('#queue-sheet')?.getAttribute('data-queue-open');
  expect(sheetOpen()).toStrictEqual('0');
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  fireEvent.click(document.querySelector('#album-play') as HTMLElement);
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
  expect(sheetOpen()).toStrictEqual('0');
  expect(document.querySelector('#queue-scrim')).toStrictEqual(null);
  // Queuing more does not open it either.
  fireEvent.click(document.querySelector('#album-more-by [data-item-more="1"]') as HTMLElement);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(document.querySelectorAll('#queue-list [data-queue-line]')).toHaveLength(7);
  expect(sheetOpen()).toStrictEqual('0');
  // The queue control opens it, and choosing a line to play leaves it open.
  fireEvent.click(document.querySelector('#player-queue') as HTMLElement);
  expect(sheetOpen()).toStrictEqual('1');
  fireEvent.click(document.querySelector('#queue-play-demo-track-01-03') as HTMLElement);
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Low Tide Letter');
  expect(sheetOpen()).toStrictEqual('1');
  fireEvent.click(document.querySelector('#player-queue') as HTMLElement);
  expect(sheetOpen()).toStrictEqual('0');
});
