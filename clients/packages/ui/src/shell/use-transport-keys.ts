import { useEffect } from 'react';

/**
 * The transport keys (R1 keyboard proposal): Space plays and pauses, the
 * arrows seek by five seconds, Enter skips forward. They work from wherever
 * the listener is — unless the keystroke belongs to a field or to the very
 * control the focus is on, which answers first.
 */

export type TransportKeyAction = 'play-pause' | 'next' | 'seek-forward' | 'seek-back';

export type TransportKeyHandlers = {
  onPlayPause?: (() => void) | undefined;
  onNext?: (() => void) | undefined;
  /** Seek by a delta in milliseconds (the arrows: ±5000). */
  onSeekBy?: ((deltaMs: number) => void) | undefined;
};

/** How far each arrow moves the playing position. */
export const TRANSPORT_SEEK_MS = 5_000;

/** The transport's key table: every key it answers, and only those. */
export function transportKeyAction(key: string): TransportKeyAction | undefined {
  if (key === ' ') {
    return 'play-pause';
  }
  if (key === 'Enter') {
    return 'next';
  }
  if (key === 'ArrowRight') {
    return 'seek-forward';
  }
  if (key === 'ArrowLeft') {
    return 'seek-back';
  }
  return undefined;
}

/** A keystroke inside one of these belongs where it is, not to the transport. */
const OWNS_KEYS =
  'input, textarea, select, button, a[href], [contenteditable="true"], [role="button"], [role="slider"]';

/** Listen on the document for the transport keys and route them to the player. */
export function useTransportKeys(handlers: TransportKeyHandlers): void {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.isComposing) {
        return;
      }
      const target = event.target;
      if (target instanceof Element && target.closest(OWNS_KEYS) !== null) {
        return;
      }
      const action = transportKeyAction(event.key);
      if (action === undefined) {
        return;
      }
      event.preventDefault();
      if (action === 'play-pause') {
        handlers.onPlayPause?.();
      } else if (action === 'next') {
        handlers.onNext?.();
      } else if (action === 'seek-forward') {
        handlers.onSeekBy?.(TRANSPORT_SEEK_MS);
      } else {
        handlers.onSeekBy?.(-TRANSPORT_SEEK_MS);
      }
    };
    globalThis.document.addEventListener('keydown', onKey);
    return () => {
      globalThis.document.removeEventListener('keydown', onKey);
    };
  }, [handlers]);
}
