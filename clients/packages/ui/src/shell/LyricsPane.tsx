import { useEffect, useRef, useState } from 'react';
import { Text, View } from 'react-native-web';
import { currentLineAt, FOLLOW_IDLE_MS, type SyncedLine } from './synced-lyrics.ts';

/**
 * Distance from the current line beyond which the falloff saturates: the CSS
 * ramp in apps/demo/public/area-lyrics.css styles exactly these steps, and
 * the value is always a small integer, never free text.
 */
const FAR_DISTANCE = 4;

/**
 * How long a programmatic scroll owns the scroll events it causes. A smooth
 * scroll reports a whole burst of them while it runs; inside this window the
 * pane treats every scroll report as its own, so its own scroll can never
 * cancel the follow (the self-cancelling cycle of MUS-155).
 */
export const PROGRAMMATIC_SCROLL_GRACE_MS = 500;

export type LyricsPaneProps = {
  id: string;
  label: string;
  lines: readonly string[];
  synced: boolean;
  open: boolean;
  /** Synced only: index of the highlighted line when the sheet carries no timestamps. */
  currentLine?: number | undefined;
  /** True when the track has no lyrics (the resolver returned the placeholder): quiet centred chrome. */
  empty?: boolean | undefined;
  /**
   * The resolver's timestamped sheet (synced only): when present, the pane
   * owns the line clock and lights the line the position is sounding.
   */
  timedLines?: readonly SyncedLine[] | undefined;
  /** The playing position the line clock reads, in milliseconds. */
  positionMs?: number | undefined;
  /** How long a manual scroll pauses the follow (MUS-155). */
  followIdleMs?: number | undefined;
};

export function LyricsPane({
  id,
  label,
  lines,
  synced,
  open,
  currentLine = 0,
  empty = false,
  timedLines,
  positionMs = 0,
  followIdleMs = FOLLOW_IDLE_MS,
}: LyricsPaneProps) {
  const [following, setFollowing] = useState(true);
  const resumeRef = useRef<number | undefined>(undefined);
  const programmaticUntilRef = useRef(0);
  const scrollerRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    return () => {
      if (resumeRef.current !== undefined) {
        globalThis.clearTimeout(resumeRef.current);
      }
    };
  }, []);
  const pauseFollowing = () => {
    setFollowing(false);
    if (resumeRef.current !== undefined) {
      globalThis.clearTimeout(resumeRef.current);
    }
    resumeRef.current = globalThis.setTimeout(() => {
      setFollowing(true);
    }, followIdleMs) as unknown as number;
  };
  /* The timed sheet replaces the plain one: its texts are the lines and its
     timestamps are the clock. A plain pane ignores it. */
  const sheet = synced && timedLines !== undefined ? timedLines : null;
  const texts = sheet === null ? lines : sheet.map((line) => line.text);
  const active = sheet === null ? currentLine : currentLineAt(sheet, positionMs);
  /* A boolean, not the sheet itself: a parent may rebuild the array on every
     render and the identity must not re-run the follow effect. */
  const followsTimedSheet = sheet !== null;
  /* While following, the sounding line stays in view, about a third of the
     way down (MUS-155). The effect is keyed on the sounding line index, not
     the position: a tick inside one line has nothing new to bring into
     view, and scrolling on every tick would renew the ownership window on
     every tick. The scroll behaviour itself is the stylesheet's — smooth
     where motion is welcome, a jump under reduced motion. */
  useEffect(() => {
    if (!following || !followsTimedSheet) {
      return;
    }
    const scroller = scrollerRef.current;
    const line = scroller?.querySelector<HTMLElement>('[data-current="1"]') ?? null;
    if (line === null || typeof line.scrollIntoView !== 'function') {
      return;
    }
    programmaticUntilRef.current = Date.now() + PROGRAMMATIC_SCROLL_GRACE_MS;
    line.scrollIntoView({ block: 'center' });
  }, [following, followsTimedSheet, active, open]);
  if (!open) {
    return null;
  }
  return (
    <View
      id={id}
      accessibilityRole="region"
      accessibilityLabel={label}
      dataSet={{ lyricsPane: '1', synced: synced ? '1' : '0', empty: empty ? '1' : '0' }}
    >
      {/* The scroll container is plain DOM so a manual scroll (wheel, touch,
          scrollbar) can pause the follow; RNW's View has no such props. */}
      <div
        data-lyrics-scroll="1"
        data-following={following ? '1' : '0'}
        ref={scrollerRef}
        onWheel={() => {
          pauseFollowing();
        }}
        onTouchMove={() => {
          pauseFollowing();
        }}
        onScroll={() => {
          /* A scroll inside the ownership window is the pane's own: the
             burst a smooth scroll reports must not pause the follow. Only a
             later scroll, with no programmatic cause, is manual. */
          if (Date.now() < programmaticUntilRef.current) {
            return;
          }
          pauseFollowing();
        }}
      >
        {texts.map((line, index) => {
          const distance = synced ? Math.min(Math.abs(index - active), FAR_DISTANCE) : 0;
          return (
            <Text
              key={`${index}:${line}`}
              dataSet={{
                lyricsLine: '1',
                current: synced && index === active ? '1' : '0',
                distance: `${distance}`,
              }}
            >
              {line}
            </Text>
          );
        })}
      </div>
    </View>
  );
}
