import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import type { DemoLibrary } from '../../../packages/fake-server/src/types.ts';
import { App } from './App.tsx';

afterEach(cleanup);

test('the demo app mounts the shell with fixture home rows and demo data', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  expect(screen.getByText('Gunmetal').id).toStrictEqual('shell-wordmark');
  expect(screen.getByText('Fixture library').id).toStrictEqual('demo-label');
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

function storeRows(): (string | null)[][] {
  return [...document.querySelectorAll('[data-store-card]')].map((node) => [
    node.getAttribute('data-store-card'),
    node.querySelector('[data-store-status]')?.textContent ?? null,
  ]);
}

test('settings keeps its five sections and the store lists the records', () => {
  window.localStorage.clear();
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(screen.getByRole('link', { name: 'Settings' }));
  expect(document.querySelector('#settings-extensions')).toBeNull();
  expect(screen.getAllByRole('tab').map((tab) => tab.textContent)).toStrictEqual([
    'Appearance',
    'Playback',
    'Connected services',
    'About this connection',
    'Privacy',
  ]);
  fireEvent.click(screen.getByRole('link', { name: 'Store' }));
  expect(document.querySelector('#destination-store')).not.toBeNull();
  expect(storeRows()).toStrictEqual([
    ['cover-art', 'On this server'],
    ['lyrics', 'In the store'],
    ['catalogue-search', 'In the store'],
    ['scrobble', 'In the store'],
    ['themes', 'In the store'],
    ['home-rows', 'In the store'],
    ['url-style', 'On this server'],
  ]);
  expect(document.querySelector('#destination-store [data-slot-version]')).toBeNull();
  // A record opens inside the store; back returns to the list.
  fireEvent.click(screen.getByRole('button', { name: 'Metadata and artwork' }));
  expect(document.querySelector('[data-store-detail="cover-art"]')).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Back to store' }));
  expect(document.querySelector('[data-store-detail="cover-art"]')).toBeNull();
  expect(storeRows().length).toStrictEqual(7);
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

test('playback preferences are read from gunmetal.playback and a control writes them back', () => {
  window.localStorage.clear();
  window.localStorage.setItem('gunmetal.playback', '{"levelling":"album","crossfadeSeconds":8,"sinkId":""}');
  window.history.pushState(null, '', '/settings/playback');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  const levelling = screen.getByRole('radiogroup', { name: 'Volume levelling' });
  const crossfade = screen.getByRole('radiogroup', { name: 'Crossfade' });
  expect(within(levelling).getByRole('radio', { name: 'Album' }).getAttribute('aria-checked')).toStrictEqual('true');
  expect(within(crossfade).getByRole('radio', { name: '8 seconds' }).getAttribute('aria-checked')).toStrictEqual(
    'true',
  );
  fireEvent.click(within(levelling).getByRole('radio', { name: 'Track' }));
  expect(window.localStorage.getItem('gunmetal.playback')).toStrictEqual(
    '{"levelling":"track","crossfadeSeconds":8,"sinkId":""}',
  );
  fireEvent.click(within(crossfade).getByRole('radio', { name: 'Off' }));
  expect(window.localStorage.getItem('gunmetal.playback')).toStrictEqual(
    '{"levelling":"track","crossfadeSeconds":0,"sinkId":""}',
  );
  fireEvent.click(
    within(screen.getByRole('radiogroup', { name: 'Output device' })).getByRole('radio', { name: 'Default' }),
  );
  expect(window.localStorage.getItem('gunmetal.playback')).toStrictEqual(
    '{"levelling":"track","crossfadeSeconds":0,"sinkId":""}',
  );
  window.localStorage.clear();
});

test('a blocked playback store is not remembered and does not throw', () => {
  const real = window.localStorage;
  const reads: string[] = [];
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem(key: string): string | null {
        reads.push(key);
        throw new DOMException('denied', 'SecurityError');
      },
      setItem(): void {
        throw new DOMException('full', 'QuotaExceededError');
      },
      removeItem(): void {},
      clear(): void {},
      key(): string | null {
        return null;
      },
      length: 0,
    },
  });
  try {
    window.history.pushState(null, '', '/settings/playback');
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
    const blockedRead = render(<App />);
    expect(reads[0]).toStrictEqual('gunmetal.playback');
    expect(
      within(screen.getByRole('radiogroup', { name: 'Volume levelling' }))
        .getByRole('radio', { name: 'Off' })
        .getAttribute('aria-checked'),
    ).toStrictEqual('true');
    blockedRead.unmount();
  } finally {
    Object.defineProperty(window, 'localStorage', { configurable: true, value: real });
  }

  const kept = new Map<string, string>([
    ['gunmetal.playback', '{"levelling":"track","crossfadeSeconds":4,"sinkId":""}'],
  ]);
  const writes: string[] = [];
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem(key: string): string | null {
        return kept.get(key) ?? null;
      },
      setItem(key: string, value: string): void {
        writes.push(`${key} ${value}`);
        throw new DOMException('full', 'QuotaExceededError');
      },
      removeItem(): void {},
      clear(): void {},
      key(): string | null {
        return null;
      },
      length: kept.size,
    },
  });
  try {
    render(<App />);
    fireEvent.click(
      within(screen.getByRole('radiogroup', { name: 'Volume levelling' })).getByRole('radio', { name: 'Album' }),
    );
    expect(
      within(screen.getByRole('radiogroup', { name: 'Volume levelling' }))
        .getByRole('radio', { name: 'Album' })
        .getAttribute('aria-checked'),
    ).toStrictEqual('true');
    expect(writes).toStrictEqual(['gunmetal.playback {"levelling":"album","crossfadeSeconds":4,"sinkId":""}']);
    expect(kept.get('gunmetal.playback')).toStrictEqual('{"levelling":"track","crossfadeSeconds":4,"sinkId":""}');
  } finally {
    Object.defineProperty(window, 'localStorage', { configurable: true, value: real });
  }
});

test('output devices are listed once and choosing one writes the sink', async () => {
  window.localStorage.clear();
  const calls: string[] = [];
  const watched: string[] = [];
  Object.defineProperty(navigator, 'mediaDevices', {
    configurable: true,
    value: {
      enumerateDevices: () => {
        calls.push('enumerate');
        return Promise.resolve([
          { kind: 'audioinput', deviceId: 'mic', label: 'Mic', groupId: 'g' },
          { kind: 'audiooutput', deviceId: 'speakers', label: 'Studio speakers', groupId: 'g' },
          { kind: 'videoinput', deviceId: 'cam', label: 'Camera', groupId: 'g' },
        ]);
      },
      addEventListener: (type: string) => {
        watched.push(type);
      },
      removeEventListener: (type: string) => {
        watched.push(`off ${type}`);
      },
    },
  });
  try {
    window.history.pushState(null, '', '/settings/playback');
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
    const view = render(<App />);
    const speakers = await screen.findByRole('radio', { name: 'Studio speakers' });
    expect(screen.queryByRole('radio', { name: 'Mic' })).toBeNull();
    expect(screen.queryByRole('radio', { name: 'Camera' })).toBeNull();
    expect(calls).toStrictEqual(['enumerate']);
    fireEvent.click(speakers);
    expect(window.localStorage.getItem('gunmetal.playback')).toStrictEqual(
      '{"levelling":"off","crossfadeSeconds":0,"sinkId":"speakers"}',
    );
    expect(calls).toStrictEqual(['enumerate']);
    view.unmount();
    expect(watched).toStrictEqual(['devicechange', 'off devicechange']);
  } finally {
    Reflect.deleteProperty(navigator, 'mediaDevices');
    window.localStorage.clear();
  }
});

test('a finished activity job reloads a served library, and a zero count does not', async () => {
  const album = 'a'.repeat(16);
  const artist = 'b'.repeat(16);
  const track = 'c'.repeat(16);
  const folder = {
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
  let done = 1;
  let libraryOk = false;
  vi.useFakeTimers();
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string) => {
      if (url === '/activity.json') {
        return {
          ok: true,
          json: async () => ({ jobs: [{ id: 'artwork', label: 'Fetching album art', done, total: 4 }] }),
        };
      }
      return { ok: libraryOk, json: async () => folder };
    }),
  );
  try {
    window.history.pushState(null, '', '/');
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
    render(<App />);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByRole('heading', { name: 'Harbour Lights' })).not.toBeNull();

    done = 0;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(screen.getByRole('heading', { name: 'Harbour Lights' })).not.toBeNull();

    done = 2;
    libraryOk = true;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(screen.getByRole('heading', { name: 'St. Elsewhere' })).not.toBeNull();
  } finally {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  }
});

test('a device list that arrives after unmount is ignored', async () => {
  window.localStorage.clear();
  let resolveDevices: (devices: MediaDeviceInfo[]) => void = () => {};
  const unwatched: string[] = [];
  Object.defineProperty(navigator, 'mediaDevices', {
    configurable: true,
    value: {
      enumerateDevices: () =>
        new Promise<MediaDeviceInfo[]>((resolve) => {
          resolveDevices = resolve;
        }),
      addEventListener: () => undefined,
      removeEventListener: (type: string) => {
        unwatched.push(type);
      },
    },
  });
  try {
    window.history.pushState(null, '', '/');
    const view = render(<App />);
    view.unmount();
    resolveDevices([{ kind: 'audiooutput', deviceId: 'late', label: 'Late', groupId: 'g' } as MediaDeviceInfo]);
    await act(async () => {
      await Promise.resolve();
    });
    expect(document.querySelector('#token-shell')).toBeNull();
    expect(unwatched).toStrictEqual(['devicechange']);
  } finally {
    Reflect.deleteProperty(navigator, 'mediaDevices');
  }
});

test('a devicechange re-lists the outputs, and unmount removes the listener', async () => {
  window.localStorage.clear();
  let devices: MediaDeviceInfo[] = [
    { kind: 'audiooutput', deviceId: 'speakers', label: 'Studio speakers', groupId: 'g' } as MediaDeviceInfo,
    { kind: 'audiooutput', deviceId: 'quiet', label: '', groupId: 'g' } as MediaDeviceInfo,
  ];
  const listeners: Array<{ type: string; listener: () => void }> = [];
  const removed: Array<{ type: string; listener: () => void }> = [];
  Object.defineProperty(navigator, 'mediaDevices', {
    configurable: true,
    value: {
      enumerateDevices: () => Promise.resolve(devices),
      addEventListener: (type: string, listener: () => void) => {
        listeners.push({ type, listener });
      },
      removeEventListener: (type: string, listener: () => void) => {
        removed.push({ type, listener });
      },
    },
  });
  try {
    window.history.pushState(null, '', '/settings/playback');
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
    const view = render(<App />);
    expect(await screen.findByRole('radio', { name: 'Studio speakers' })).not.toBeNull();
    // The browser left the second output unnamed: its place names it.
    expect(screen.getByRole('radio', { name: 'Output 2' })).not.toBeNull();
    expect(listeners.map((entry) => entry.type)).toStrictEqual(['devicechange']);

    devices = [{ kind: 'audiooutput', deviceId: 'headphones', label: 'Headphones', groupId: 'g' } as MediaDeviceInfo];
    await act(async () => {
      listeners[0]?.listener();
      await Promise.resolve();
    });
    expect(await screen.findByRole('radio', { name: 'Headphones' })).not.toBeNull();
    expect(screen.queryByRole('radio', { name: 'Studio speakers' })).toBeNull();
    expect(screen.queryByRole('radio', { name: 'Output 2' })).toBeNull();

    view.unmount();
    expect(removed).toStrictEqual([{ type: 'devicechange', listener: listeners[0]?.listener }]);
  } finally {
    Reflect.deleteProperty(navigator, 'mediaDevices');
    window.localStorage.clear();
  }
});
