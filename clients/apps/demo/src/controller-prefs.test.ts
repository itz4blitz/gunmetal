import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../packages/fake-server/src/catalogue.ts';
import type { ShellLibrary } from '../../../packages/ui/src/shell/library-types.ts';
import type { PlaybackPrefs } from './playback-prefs.ts';
import { useDemoPlayback } from './controller.ts';
import type { DemoAudioElement } from './demo-audio.ts';

const recorded = vi.hoisted(() => ({
  setGainDb: [] as Array<number | undefined>,
  setCrossfadeMs: [] as number[],
  setSinkId: [] as string[],
  armNext: [] as Array<[string, number]>,
}));

vi.mock('./demo-audio.ts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./demo-audio.ts')>();
  return {
    ...actual,
    createDemoAudio: (...args: Parameters<typeof actual.createDemoAudio>) => {
      const audio = actual.createDemoAudio(...args);
      return {
        ...audio,
        setGainDb(db: number | undefined) {
          recorded.setGainDb.push(db);
          audio.setGainDb(db);
        },
        setCrossfadeMs(ms: number) {
          recorded.setCrossfadeMs.push(ms);
          audio.setCrossfadeMs(ms);
        },
        setSinkId(id: string) {
          recorded.setSinkId.push(id);
          audio.setSinkId(id);
        },
        armNext(url: string, durationMs: number) {
          recorded.armNext.push([url, durationMs]);
          audio.armNext(url, durationMs);
        },
      };
    },
  };
});

type Listener = () => void;

class FakeAudio implements DemoAudioElement {
  src = '';
  loop = false;
  volume = 1;
  currentTime = 0;
  duration = 0;
  error: { message?: string } | null = null;
  listeners: Record<string, Listener[]> = {};

  play(): void {}

  pause(): void {}

  addEventListener(type: string, listener: Listener): void {
    (this.listeners[type] ??= []).push(listener);
  }

  removeEventListener(type: string, listener: Listener): void {
    this.listeners[type] = (this.listeners[type] ?? []).filter((entry) => entry !== listener);
  }
}

const ALBUM_URL = '/media/audio/demo-album-01.wav';

const trackPrefs: PlaybackPrefs = { levelling: 'track', crossfadeSeconds: 8, sinkId: 'dac-1' };

function libraryWithGains(): ShellLibrary {
  const library = demoLibrary();
  return {
    ...library,
    albums: library.albums.map((album) => {
      if (album.id !== 'demo-album-01') {
        return album;
      }
      return {
        ...album,
        tracks: album.tracks.map((track) => {
          if (track.id === 'demo-track-01-01') {
            return { ...track, trackGainDb: -6.5, albumGainDb: -3.25 };
          }
          if (track.id === 'demo-track-01-02') {
            return { ...track, trackGainDb: -1.5, albumGainDb: -3.25 };
          }
          if (track.id === 'demo-track-01-04') {
            return { ...track, trackGainDb: -8, albumGainDb: -3.25 };
          }
          return track;
        }),
      };
    }),
  };
}

function wire(element: FakeAudio, prefs: PlaybackPrefs | undefined) {
  return {
    createElement: () => element,
    prefs,
  } as Parameters<typeof useDemoPlayback>[1];
}

beforeEach(() => {
  recorded.setGainDb.length = 0;
  recorded.setCrossfadeMs.length = 0;
  recorded.setSinkId.length = 0;
  recorded.armNext.length = 0;
});

afterEach(() => {
  cleanup();
});

test('levelling, crossfade and sink are applied to the engine when prefs or the playing track change', () => {
  const element = new FakeAudio();
  const library = libraryWithGains();
  const hook = renderHook(({ prefs }: { prefs: PlaybackPrefs }) => useDemoPlayback(library, wire(element, prefs)), {
    initialProps: { prefs: trackPrefs },
  });

  // Nothing is playing yet: track gain is absent, and there is no successor to arm.
  expect(recorded.setGainDb).toStrictEqual([undefined]);
  expect(recorded.setCrossfadeMs).toStrictEqual([8000]);
  expect(recorded.setSinkId).toStrictEqual(['dac-1']);
  expect(recorded.armNext).toStrictEqual([]);

  act(() => {
    hook.result.current.playAlbum('demo-album-01');
  });
  expect(recorded.setGainDb).toStrictEqual([undefined, -6.5]);
  expect(recorded.armNext).toStrictEqual([[ALBUM_URL, 198_000]]);

  hook.rerender({ prefs: { levelling: 'album', crossfadeSeconds: 8, sinkId: 'dac-1' } });
  expect(recorded.setGainDb).toStrictEqual([undefined, -6.5, -3.25]);
  expect(recorded.setCrossfadeMs).toStrictEqual([8000, 8000, 8000]);
  expect(recorded.setSinkId).toStrictEqual(['dac-1', 'dac-1', 'dac-1']);
  expect(recorded.armNext).toStrictEqual([
    [ALBUM_URL, 198_000],
    [ALBUM_URL, 198_000],
  ]);

  hook.rerender({ prefs: { levelling: 'off', crossfadeSeconds: 2, sinkId: '' } });
  expect(recorded.setGainDb).toStrictEqual([undefined, -6.5, -3.25, undefined]);
  expect(recorded.setCrossfadeMs).toStrictEqual([8000, 8000, 8000, 2000]);
  expect(recorded.setSinkId).toStrictEqual(['dac-1', 'dac-1', 'dac-1', '']);
  hook.unmount();
});

test('the armed successor follows shuffle, repeat and the end of the queue', () => {
  const element = new FakeAudio();
  const hook = renderHook(() =>
    useDemoPlayback(libraryWithGains(), wire(element, { levelling: 'off', crossfadeSeconds: 0, sinkId: '' })),
  );
  act(() => {
    hook.result.current.playAlbum('demo-album-01');
  });
  expect(recorded.armNext).toStrictEqual([[ALBUM_URL, 198_000]]);

  act(() => {
    hook.result.current.toggleShuffle();
  });
  // Seed 1 walks [1, 2, 0, 3]: from Pier the next line is Beacon.
  expect(recorded.armNext).toStrictEqual([
    [ALBUM_URL, 198_000],
    [ALBUM_URL, 187_000],
  ]);

  act(() => {
    hook.result.current.toggleShuffle();
  });
  expect(recorded.armNext.at(-1)).toStrictEqual([ALBUM_URL, 198_000]);

  act(() => {
    hook.result.current.next();
  });
  // Salt Window's successor is Low Tide.
  expect(recorded.armNext.at(-1)).toStrictEqual([ALBUM_URL, 241_000]);
  act(() => {
    hook.result.current.next();
  });
  // Low Tide's successor is Beacon.
  expect(recorded.armNext.at(-1)).toStrictEqual([ALBUM_URL, 187_000]);
  act(() => {
    hook.result.current.next();
  });
  // Beacon is the tail and repeat is off, so the previous arm stays.
  expect(recorded.armNext).toStrictEqual([
    [ALBUM_URL, 198_000],
    [ALBUM_URL, 187_000],
    [ALBUM_URL, 198_000],
    [ALBUM_URL, 241_000],
    [ALBUM_URL, 187_000],
  ]);

  act(() => {
    hook.result.current.cycleRepeat();
  });
  expect(recorded.armNext.at(-1)).toStrictEqual([ALBUM_URL, 214_000]);

  act(() => {
    hook.result.current.cycleRepeat();
  });
  expect(recorded.armNext.at(-1)).toStrictEqual([ALBUM_URL, 187_000]);
  hook.unmount();
});

test('a track without gain tags, a missing library and a dropped track pass undefined', () => {
  const element = new FakeAudio();
  const plain = renderHook(() =>
    useDemoPlayback(demoLibrary(), wire(element, { levelling: 'track', crossfadeSeconds: 4, sinkId: 'speakers' })),
  );
  act(() => {
    plain.result.current.playAlbum('demo-album-01');
  });
  expect(recorded.setGainDb).toStrictEqual([undefined, undefined]);
  expect(recorded.setCrossfadeMs).toStrictEqual([4000, 4000]);
  plain.unmount();

  recorded.setGainDb.length = 0;
  recorded.setCrossfadeMs.length = 0;
  recorded.setSinkId.length = 0;
  recorded.armNext.length = 0;
  const missing = renderHook(() =>
    useDemoPlayback(undefined, wire(new FakeAudio(), { levelling: 'album', crossfadeSeconds: 6, sinkId: 'dac-1' })),
  );
  expect(recorded.setGainDb).toStrictEqual([undefined]);
  expect(recorded.setCrossfadeMs).toStrictEqual([6000]);
  expect(recorded.setSinkId).toStrictEqual(['dac-1']);
  expect(recorded.armNext).toStrictEqual([]);
  missing.unmount();

  recorded.setGainDb.length = 0;
  recorded.armNext.length = 0;
  const tagged = libraryWithGains();
  const dropped = renderHook(
    ({ library }: { library: ShellLibrary }) =>
      useDemoPlayback(library, wire(new FakeAudio(), { levelling: 'album', crossfadeSeconds: 12, sinkId: 'dac-1' })),
    {
      initialProps: { library: tagged },
    },
  );
  act(() => {
    dropped.result.current.playAlbum('demo-album-01');
  });
  expect(recorded.setGainDb).toStrictEqual([undefined, -3.25]);
  dropped.rerender({ library: { albums: [], artists: [] } });
  expect(recorded.setGainDb).toStrictEqual([undefined, -3.25, undefined]);
  dropped.unmount();
});
