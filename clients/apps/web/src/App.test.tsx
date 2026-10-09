import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';
import { App } from './App.tsx';

const ALBUM = 'a'.repeat(16);
const ARTIST = 'b'.repeat(16);
const TRACK = 'c'.repeat(16);

function folderLibrary(): DemoLibrary {
  return {
    kind: 'folder',
    albums: [
      {
        id: ALBUM,
        title: 'St. Elsewhere',
        artistName: 'Gnarls Barkley',
        artistKey: ARTIST,
        year: 2006,
        coverTone: '01',
        coverUrl: `/media/library/covers/${ALBUM}.jpg`,
        discs: [{ index: 1, title: '' }],
        hostile: false,
        tracks: [
          {
            id: TRACK,
            albumId: ALBUM,
            discIndex: 1,
            number: 1,
            title: 'Crazy',
            artistName: 'Gnarls Barkley',
            durationMs: 178_000,
            flag: 'ok',
            lyricsKind: 'none',
            mediaUrl: `/media/library/${TRACK}`,
          },
        ],
      },
    ],
    artists: [{ key: ARTIST, name: 'Gnarls Barkley', albumIds: [ALBUM] }],
  };
}

function atWidth(width: number): void {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: width });
}

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

test('a wide window is the three-pane player, and a phone opens the full player', () => {
  atWidth(1600);
  const wide = render(<App />);
  expect(document.getElementById('token-shell')?.getAttribute('data-width')).toStrictEqual('wide');
  expect(document.getElementById('token-shell')?.getAttribute('data-landmarks')).toStrictEqual(
    'nav-sidebar content right-pane player-bar',
  );
  expect(document.getElementById('nav-sidebar')).not.toBeNull();
  expect(document.getElementById('right-pane')).not.toBeNull();
  expect(document.getElementById('player-bar')).not.toBeNull();
  expect(document.getElementById('player-full')).toBeNull();
  wide.unmount();
  cleanup();

  atWidth(390);
  render(<App />);
  expect(document.getElementById('token-shell')?.getAttribute('data-width')).toStrictEqual('compact');
  expect(document.getElementById('token-shell')?.getAttribute('data-landmarks')).toStrictEqual(
    'content player-bar nav-tabs',
  );
  expect(document.getElementById('nav-sidebar')).toBeNull();
  expect(document.getElementById('nav-tabs')).not.toBeNull();
  fireEvent.click(document.querySelector('#home-spotlight-play') as HTMLElement);
  expect(document.getElementById('player-full')).not.toBeNull();
  expect(document.getElementById('player-title')?.textContent).toStrictEqual('Pier at Dusk');
});

test('the web client plays a same-origin folder library on desktop and on a phone', () => {
  atWidth(1600);
  const library = folderLibrary();
  const wide = render(<App library={library} />);
  expect(screen.queryByText('Demo data')).toBeNull();
  expect(document.getElementById('destination-headline')).toBeNull();
  expect(screen.getByRole('heading', { name: 'St. Elsewhere' }).textContent).toStrictEqual('St. Elsewhere');
  fireEvent.click(screen.getByRole('button', { name: 'St. Elsewhere' }));
  expect(document.getElementById('destination-headline')?.textContent).toStrictEqual('St. Elsewhere');
  fireEvent.click(document.getElementById('album-play') as HTMLElement);
  expect(document.getElementById('player-title')?.textContent).toStrictEqual('Crazy');
  expect(document.getElementById('player-artist')?.textContent).toStrictEqual('Gnarls Barkley · St. Elsewhere');
  expect(document.getElementById(`queue-line-${TRACK}`)).not.toBeNull();
  expect(document.body.textContent?.includes('192.168.1.120')).toStrictEqual(false);
  expect(document.body.textContent?.includes('http://')).toStrictEqual(false);
  fireEvent.click(screen.getByRole('button', { name: 'Go to artist' }));
  expect(document.getElementById('destination-artist')?.getAttribute('data-artist-key')).toStrictEqual(ARTIST);
  expect(document.getElementById('destination-headline')?.textContent).toStrictEqual('Gnarls Barkley');
  fireEvent.click(screen.getByRole('link', { name: 'Search' }));
  expect(document.getElementById('destination-headline')?.textContent).toStrictEqual('Search');
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), { target: { value: 'Crazy' } });
  expect(document.querySelector('[data-search-top-title="1"]')?.textContent).toStrictEqual('Crazy');
  fireEvent.click(screen.getByRole('link', { name: 'Library' }));
  expect(document.getElementById('destination-headline')?.textContent).toStrictEqual('Library');
  fireEvent.click(screen.getByRole('link', { name: 'Settings' }));
  expect(document.getElementById('destination-headline')?.textContent).toStrictEqual('Settings');
  fireEvent.click(document.getElementById('player-expand') as HTMLElement);
  expect(document.getElementById('player-full-lyrics-unavailable')?.textContent).toStrictEqual(
    'This file has no lyrics.',
  );
  wide.unmount();
  cleanup();

  atWidth(390);
  render(<App library={library} />);
  expect(document.getElementById('token-shell')?.getAttribute('data-width')).toStrictEqual('compact');
  fireEvent.click(screen.getByRole('button', { name: 'St. Elsewhere' }));
  fireEvent.click(document.getElementById('album-play') as HTMLElement);
  expect(document.getElementById('player-full')).not.toBeNull();
  expect(document.getElementById('player-title')?.textContent).toStrictEqual('Crazy');
});

test('fixture lyrics open from the player the web client mounts', () => {
  atWidth(1600);
  render(<App />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  fireEvent.click(screen.getByRole('button', { name: 'Low Tide Letter' }));
  expect(document.getElementById('player-title')?.textContent).toStrictEqual('Low Tide Letter');
  expect(document.getElementById('queue-line-demo-track-01-03')).not.toBeNull();
  fireEvent.click(document.getElementById('player-expand') as HTMLElement);
  fireEvent.click(document.querySelector('#player-full [aria-label="Lyrics"]') as HTMLElement);
  expect(screen.getByText('The harbour keeps the letter').textContent).toStrictEqual('The harbour keeps the letter');
});
