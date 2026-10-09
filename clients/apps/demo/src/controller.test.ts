import { act, renderHook } from '@testing-library/react';
import { expect, test } from 'vitest';
import { demoLibrary } from '../../../packages/fake-server/src/catalogue.ts';
import type { MediaSessionAction, MediaSessionBridge } from '../../../packages/ui/src/shell/media-session.ts';
import { findAlbum } from './playback.ts';
import { serializeSessionPlayback } from './session-playback.ts';
import { useDemoPlayback } from './controller.ts';
import type { DemoAudioElement } from './demo-audio.ts';

type Listener = () => void;

/** A scripted stand-in for the demo's <audio> element. */
class FakeAudio implements DemoAudioElement {
  src = '';
  loop = false;
  volume = 1;
  currentTime = 0;
  duration = 0;
  error: { message?: string } | null = null;
  playCalls = 0;
  pauseCalls = 0;
  listeners: Record<string, Listener[]> = {};

  play(): void {
    this.playCalls += 1;
  }

  pause(): void {
    this.pauseCalls += 1;
  }

  addEventListener(type: string, listener: Listener): void {
    (this.listeners[type] ??= []).push(listener);
  }

  removeEventListener(type: string, listener: Listener): void {
    this.listeners[type] = (this.listeners[type] ?? []).filter((entry) => entry !== listener);
  }

  fire(type: string): void {
    for (const listener of this.listeners[type] ?? []) {
      listener();
    }
  }

  tick(seconds: number): void {
    this.currentTime += seconds;
    this.fire('timeupdate');
  }
}

function recordingMediaSession() {
  const metadata: unknown[] = [];
  const playbackStates: string[] = [];
  const states: Array<Record<string, number>> = [];
  const handlers = new Map<MediaSessionAction, unknown>();
  const bridge: MediaSessionBridge = {
    setMetadata: (value) => metadata.push(value),
    setPlaybackState: (value) => playbackStates.push(value),
    setPositionState: (state) => states.push(state),
    setActionHandler: (action, handler) => {
      handlers.set(action, handler);
    },
  };
  return { bridge, metadata, playbackStates, states, handlers };
}

function setup(input?: Parameters<typeof useDemoPlayback>[1]) {
  const element = new FakeAudio();
  const library = demoLibrary();
  const hook = renderHook(() => useDemoPlayback(library, { ...input, createElement: () => element }));
  return { element, library, ...hook };
}

test('playing an album loads the element, walks the clock and hands over at the end', () => {
  const { element, result } = setup();
  expect(result.current.state.trackId).toStrictEqual(undefined);
  expect(result.current.clock).not.toStrictEqual(undefined);
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-01');
  expect(result.current.state.playing).toStrictEqual(true);
  expect(result.current.state.durationMs).toStrictEqual(214_000);
  expect(element.src).toStrictEqual('/media/audio/demo-album-01.wav');
  expect(element.loop).toStrictEqual(true);
  expect(result.current.clock?.positionMs()).toStrictEqual(0);
  act(() => {
    element.tick(3);
  });
  expect(result.current.state.positionMs).toStrictEqual(3000);
  expect(result.current.clock?.positionMs()).toStrictEqual(3000);
  // At the catalogue duration the queue hands over to the next line. The
  // fixtures keep one media file per album, so the element stays on it.
  act(() => {
    element.tick(400);
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-02');
  expect(result.current.state.positionMs).toStrictEqual(0);
  expect(result.current.state.durationMs).toStrictEqual(198_000);
  expect(element.src).toStrictEqual('/media/audio/demo-album-01.wav');
  // Pausing mirrors the transport onto the element.
  act(() => {
    result.current.playPause();
  });
  expect(element.pauseCalls).toBeGreaterThanOrEqual(1);
  expect(result.current.state.playing).toStrictEqual(false);
  // Seek rides through the engine and the snapshot, clamped into the track.
  act(() => {
    result.current.seek(999_999);
  });
  expect(result.current.state.positionMs).toStrictEqual(198_000);
  act(() => {
    result.current.next();
  });
  act(() => {
    result.current.previous();
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-02');
});

test('repeat one replays the track and repeat all wraps the queue at the ends', () => {
  const { element, result } = setup();
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  expect(result.current.state.repeatMode).toStrictEqual(undefined);
  act(() => {
    result.current.cycleRepeat();
  });
  expect(result.current.state.repeatMode).toStrictEqual('all');
  // Walk to the queue's end, then past it: the wrap lands on the head.
  act(() => {
    result.current.next();
    result.current.next();
    result.current.next();
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-04');
  act(() => {
    result.current.next();
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-01');
  // The natural end wraps too.
  act(() => {
    element.tick(300);
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-02');
  act(() => {
    result.current.cycleRepeat();
  });
  expect(result.current.state.repeatMode).toStrictEqual('one');
  act(() => {
    element.tick(300);
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-02');
  expect(result.current.state.positionMs).toStrictEqual(0);
  expect(result.current.state.playing).toStrictEqual(true);
  act(() => {
    result.current.cycleRepeat();
  });
  expect(result.current.state.repeatMode).toStrictEqual('off');
});

test('shuffle walks next and previous in the seeded order', () => {
  const { result } = setup();
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  expect(result.current.state.shuffleOn).toStrictEqual(undefined);
  act(() => {
    result.current.toggleShuffle();
  });
  expect(result.current.state.shuffleOn).toStrictEqual(true);
  expect(result.current.state.shuffleSeed).toStrictEqual(1);
  // Seed 1 over four lines orders [1, 2, 0, 3]: from Pier the next is Beacon.
  act(() => {
    result.current.next();
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-04');
  act(() => {
    result.current.previous();
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-01');
  // Toggling off returns the walk to the queue's own order.
  act(() => {
    result.current.toggleShuffle();
  });
  expect(result.current.state.shuffleOn).toStrictEqual(false);
  expect(result.current.state.shuffleSeed).toStrictEqual(2);
  act(() => {
    result.current.next();
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-02');
});

test('removing a queue line drops it; removing the current one promotes the next', () => {
  const { result } = setup();
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  act(() => {
    result.current.removeQueueLine('demo-track-01-02');
  });
  expect(result.current.state.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-01-03',
    'demo-track-01-04',
  ]);
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-01');
  act(() => {
    result.current.removeQueueLine('demo-track-01-01');
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-03');
  expect(result.current.state.playing).toStrictEqual(true);
  expect(result.current.state.positionMs).toStrictEqual(0);
});

test('buffering and element errors become honest snapshot state', () => {
  const { element, result } = setup();
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  expect(result.current.state.buffering).toStrictEqual(undefined);
  expect(result.current.state.playbackError).toStrictEqual(undefined);
  act(() => {
    element.fire('waiting');
  });
  expect(result.current.state.buffering).toStrictEqual(true);
  act(() => {
    element.fire('playing');
  });
  expect(result.current.state.buffering).toStrictEqual(false);
  element.error = { message: 'DEMUXER_ERROR_COULD_NOT_OPEN' };
  act(() => {
    element.fire('error');
  });
  expect(result.current.state.playbackError).toStrictEqual('DEMUXER_ERROR_COULD_NOT_OPEN');
  // A new play leaves the old failure behind.
  act(() => {
    result.current.playAlbum('demo-album-02');
  });
  expect(result.current.state.playbackError).toStrictEqual(undefined);
});

test('the volume comes from the store, is clamped, and the mute silences the element', () => {
  const writes: string[] = [];
  const store = {
    read: () => '{"volume":0.25,"muted":true}',
    write: (value: string) => writes.push(value),
  };
  const { element, result } = setup({ volumeStore: store });
  expect(result.current.volume).toStrictEqual(0.25);
  expect(result.current.muted).toStrictEqual(true);
  expect(element.volume).toStrictEqual(0);
  act(() => {
    result.current.setMuted(false);
  });
  expect(element.volume).toBeCloseTo(0.25, 5);
  expect(writes).toStrictEqual(['{"volume":0.25,"muted":false}']);
  act(() => {
    result.current.setVolume(2);
  });
  expect(result.current.volume).toStrictEqual(1);
  expect(element.volume).toStrictEqual(1);
  expect(writes.at(-1)).toStrictEqual('{"volume":1,"muted":false}');
  act(() => {
    result.current.setVolume(-1);
  });
  expect(result.current.volume).toStrictEqual(0);
});

test('the media session bridge receives the snapshot and drives the verbs', () => {
  const session = recordingMediaSession();
  const { result } = setup({ mediaSession: session.bridge });
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  // The handler wired while playing still answers once the player is empty:
  // a stop on an idle player leaves it alone.
  const stopWhileWired = session.handlers.get('stop') as () => void;
  act(() => {
    result.current.playAlbum('missing-album');
  });
  expect(result.current.state.trackId).toStrictEqual(undefined);
  act(() => {
    stopWhileWired();
  });
  expect(result.current.state.trackId).toStrictEqual(undefined);
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  expect(session.metadata.at(-1)).toStrictEqual({
    title: 'Pier at Dusk',
    artist: 'Mira Sol',
    album: 'Harbour Lights',
    artwork: [{ src: '/media/covers/demo-album-01.svg', sizes: 'any', type: 'image/svg+xml' }],
  });
  expect(session.playbackStates.at(-1)).toStrictEqual('playing');
  expect(session.states.at(-1)).toStrictEqual({ duration: 214, position: 0, playbackRate: 1 });
  act(() => {
    result.current.playPause();
  });
  expect(session.playbackStates.at(-1)).toStrictEqual('paused');
  const fire = (action: MediaSessionAction, details?: { seekTime?: number; seekOffset?: number }) => {
    act(() => {
      (session.handlers.get(action) as (input?: typeof details) => void)(details);
    });
  };
  fire('play');
  expect(result.current.state.playing).toStrictEqual(true);
  fire('pause');
  expect(result.current.state.playing).toStrictEqual(false);
  fire('nexttrack');
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-02');
  fire('previoustrack');
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-01');
  fire('seekto', { seekTime: 30 });
  expect(result.current.state.positionMs).toStrictEqual(30_000);
  fire('seekforward');
  expect(result.current.state.positionMs).toStrictEqual(35_000);
  fire('seekbackward', { seekOffset: 10 });
  expect(result.current.state.positionMs).toStrictEqual(25_000);
  fire('stop');
  expect(result.current.state.playing).toStrictEqual(false);
});

test('without a wired bridge the browser detection answers nothing and playback still works', () => {
  const { element, result } = setup();
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  act(() => {
    element.tick(1);
  });
  expect(result.current.state.positionMs).toStrictEqual(1000);
  expect(result.current.volume).toStrictEqual(0.8);
  expect(result.current.muted).toStrictEqual(false);
});

test('an album that cannot start keeps the player empty and the verbs honest', () => {
  const { element, result } = setup();
  act(() => {
    result.current.playAlbum('missing-album');
  });
  expect(result.current.state.trackId).toStrictEqual(undefined);
  expect(result.current.state.playing).toStrictEqual(false);
  // Engine events with nothing loaded leave the snapshot alone.
  act(() => {
    element.fire('waiting');
    element.fire('error');
    element.fire('loadedmetadata');
    element.tick(1);
  });
  expect(result.current.state.buffering).toStrictEqual(undefined);
  expect(result.current.state.playbackError).toStrictEqual(undefined);
  expect(result.current.state.positionMs).toStrictEqual(0);
  expect(() => {
    act(() => {
      result.current.playPause();
      result.current.next();
      result.current.previous();
      result.current.seek(5_000);
    });
  }).not.toThrow();
  expect(result.current.state.positionMs).toStrictEqual(0);
  // The queue verbs on an idle player start from the album, like the shell does.
  act(() => {
    result.current.addAlbumToQueue('demo-album-01');
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-01');
  const album = findAlbum(demoLibrary(), 'demo-album-01');
  expect(album?.title).toStrictEqual('Harbour Lights');
});

test('the metadata of a loaded file never overrides the catalogue duration', () => {
  const { element, result } = setup();
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  element.duration = 8; // the fixture tone is 8 seconds
  act(() => {
    element.fire('loadedmetadata');
  });
  // The engine keeps the catalogue's virtual scale (see demo-audio.ts), and
  // loadedmetadata reads as ready rather than buffering.
  expect(result.current.state.durationMs).toStrictEqual(214_000);
  expect(result.current.state.buffering).toStrictEqual(false);
});

test("the clock can be given a scheduler of the composition root's choosing", () => {
  let requested = 0;
  const scheduler = {
    request: () => {
      requested += 1;
      return requested;
    },
    cancel: () => undefined,
  };
  const { element, result } = setup({ scheduler });
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  // Nothing listens yet: playing does not put a frame on the wire.
  expect(requested).toStrictEqual(0);
  let stop: (() => void) | undefined;
  act(() => {
    stop = result.current.clock?.subscribe(() => {});
  });
  // Playing with a listener: the injected scheduler carries the frame.
  expect(requested).toStrictEqual(1);
  act(() => {
    element.tick(1);
  });
  expect(result.current.state.positionMs).toStrictEqual(1000);
  act(() => {
    stop?.();
  });
});

test('without any wiring the hook still builds a real element and stays honest', () => {
  const hook = renderHook(() => useDemoPlayback(demoLibrary()));
  expect(hook.result.current.state.trackId).toStrictEqual(undefined);
  expect(() => {
    act(() => {
      hook.result.current.playPause();
      hook.result.current.seek(1_000);
    });
  }).not.toThrow();
  act(() => {
    hook.result.current.playAlbum('demo-album-01');
  });
  expect(hook.result.current.state.trackId).toStrictEqual('demo-track-01-01');
  hook.unmount();
});

test('a track change while paused reloads without playing', () => {
  const { element, result } = setup();
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  act(() => {
    result.current.playPause();
  });
  expect(result.current.state.playing).toStrictEqual(false);
  // Removing the current line promotes the next one but keeps the pause:
  // the engine re-loads without being told to play.
  act(() => {
    result.current.removeQueueLine('demo-track-01-01');
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-02');
  expect(result.current.state.playing).toStrictEqual(false);
  expect(element.pauseCalls).toBeGreaterThanOrEqual(1);
  act(() => {
    result.current.playPause();
  });
  expect(result.current.state.playing).toStrictEqual(true);
});

test('a file with its own length teaches the player its duration', () => {
  const library = demoLibrary();
  const album = findAlbum(library, 'demo-album-01');
  if (album === undefined || album.tracks[0] === undefined) {
    throw new Error('the fixture library lost its first album');
  }
  const unknownLength = { ...album, tracks: [{ ...album.tracks[0], durationMs: 0 }] };
  const element = new FakeAudio();
  element.duration = 8;
  const hook = renderHook(() =>
    useDemoPlayback({ ...library, albums: [unknownLength, ...library.albums] }, { createElement: () => element }),
  );
  act(() => {
    hook.result.current.playAlbum('demo-album-01');
  });
  expect(hook.result.current.state.durationMs).toStrictEqual(0);
  act(() => {
    element.fire('loadedmetadata');
  });
  // The engine adopted the file's length; the snapshot says so.
  expect(hook.result.current.state.durationMs).toStrictEqual(8_000);
});

test('every remaining verb routes through the queue rules and the view state', () => {
  const { result, unmount } = setup();
  act(() => {
    result.current.playAlbum('demo-album-01');
  });
  act(() => {
    result.current.playTrack('demo-album-01', 'demo-track-01-03');
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-03');
  act(() => {
    result.current.playNextAlbum('demo-album-02');
  });
  // Playing the third track rotated album 01 to start there; Night Shift
  // lands right after the current line.
  expect(result.current.state.queue.map((line) => line.trackId)).toStrictEqual([
    'demo-track-01-03',
    'demo-track-02-01',
    'demo-track-02-02',
    'demo-track-02-03',
    'demo-track-01-04',
    'demo-track-01-01',
    'demo-track-01-02',
  ]);
  act(() => {
    result.current.playNextTrack('demo-album-01', 'demo-track-01-02');
  });
  expect(result.current.state.queue[1]?.trackId).toStrictEqual('demo-track-01-02');
  act(() => {
    result.current.addTrackToQueue('demo-album-01', 'demo-track-01-01');
  });
  expect(result.current.state.queue.at(-1)?.trackId).toStrictEqual('demo-track-01-01');
  act(() => {
    result.current.addAlbumToQueue('demo-album-02');
  });
  expect(result.current.state.queue.length).toStrictEqual(12);
  act(() => {
    result.current.previous();
  });
  expect(result.current.state.trackId).toStrictEqual('demo-track-01-03');
  // The queue sheet and the full player are the view's own state.
  expect(result.current.fullOpen).toStrictEqual(false);
  act(() => {
    result.current.openFull();
  });
  expect(result.current.fullOpen).toStrictEqual(true);
  act(() => {
    result.current.closeFull();
  });
  expect(result.current.fullOpen).toStrictEqual(false);
  act(() => {
    result.current.toggleQueue();
  });
  expect(result.current.state.queueOpen).toStrictEqual(true);
  // A new play keeps the sheet open, as the listener left it.
  act(() => {
    result.current.playAlbum('demo-album-02');
  });
  expect(result.current.state.queueOpen).toStrictEqual(true);
  act(() => {
    result.current.closeQueue();
  });
  expect(result.current.state.queueOpen).toStrictEqual(false);
  // Unmounting the hook detaches the engine and the clock without a fuss.
  unmount();
});

test('a refresh restores the queue and the place in the song, then keeps the new place', () => {
  const stored = serializeSessionPlayback({
    trackId: 'demo-track-01-01',
    albumId: 'demo-album-01',
    title: 'Pier at Dusk',
    artistName: 'Mira Sol',
    coverTone: '01',
    coverUrl: '/media/covers/demo-album-01.svg',
    mediaUrl: '/media/audio/demo-album-01.wav',
    playing: true,
    positionMs: 42_000,
    durationMs: 214_000,
    lyricsKind: 'none',
    queue: [
      {
        trackId: 'demo-track-01-01',
        albumId: 'demo-album-01',
        title: 'Pier at Dusk',
        artistName: 'Mira Sol',
        coverTone: '01',
        coverUrl: '/media/covers/demo-album-01.svg',
        mediaUrl: '/media/audio/demo-album-01.wav',
        durationMs: 214_000,
        lyricsKind: 'none',
      },
      {
        trackId: 'demo-track-01-02',
        albumId: 'demo-album-01',
        title: 'Harbour Glass',
        artistName: 'Mira Sol',
        coverTone: '01',
        coverUrl: '/media/covers/demo-album-01.svg',
        mediaUrl: '/media/audio/demo-album-01.wav',
        durationMs: 198_000,
        lyricsKind: 'none',
      },
    ],
    queueOpen: false,
    shuffleOn: false,
    shuffleSeed: 0,
    repeatMode: 'off',
  });
  const written: string[] = [];
  const element = new FakeAudio();
  const hook = renderHook(() =>
    useDemoPlayback(demoLibrary(), {
      createElement: () => element,
      sessionStore: {
        read: () => stored,
        write: (value) => {
          written.push(value);
        },
      },
    }),
  );
  expect(hook.result.current.state.trackId).toStrictEqual('demo-track-01-01');
  expect(hook.result.current.state.queue.map((entry) => entry.trackId)).toStrictEqual([
    'demo-track-01-01',
    'demo-track-01-02',
  ]);
  expect(hook.result.current.state.positionMs).toStrictEqual(42_000);
  expect(hook.result.current.state.playing).toStrictEqual(false);
  expect(element.src).toStrictEqual('/media/audio/demo-album-01.wav');
  expect(element.playCalls).toStrictEqual(0);
  element.duration = 214;
  act(() => {
    element.fire('loadedmetadata');
  });
  expect(element.currentTime).toBeCloseTo(42, 5);
  // A reload resets the element to 0. That report must not move the saved place.
  element.currentTime = 0;
  act(() => {
    element.fire('timeupdate');
  });
  expect(hook.result.current.state.positionMs).toStrictEqual(42_000);
  expect(hook.result.current.state.playing).toStrictEqual(false);
  act(() => {
    window.dispatchEvent(new Event('pagehide'));
  });
  const saved = written.at(-1) ?? '';
  expect(saved).toContain('"positionMs":42000');
  expect(saved).toContain('"playing":false');
  const again = renderHook(() =>
    useDemoPlayback(demoLibrary(), {
      createElement: () => new FakeAudio(),
      sessionStore: {
        read: () => saved,
        write: () => undefined,
      },
    }),
  );
  expect(again.result.current.state.positionMs).toStrictEqual(42_000);
  expect(again.result.current.state.playing).toStrictEqual(false);
  expect(again.result.current.state.trackId).toStrictEqual('demo-track-01-01');
  hook.unmount();
  again.unmount();
});

test('hiding a fresh visit writes the clock and removes the listeners', () => {
  const written: string[] = [];
  const hook = renderHook(() =>
    useDemoPlayback(demoLibrary(), {
      createElement: () => new FakeAudio(),
      sessionStore: {
        read: () => null,
        write: (value) => {
          written.push(value);
        },
      },
    }),
  );
  act(() => {
    window.dispatchEvent(new Event('beforeunload'));
  });
  expect(written.at(-1)).toStrictEqual('{"queue":[]}');
  hook.unmount();
});
