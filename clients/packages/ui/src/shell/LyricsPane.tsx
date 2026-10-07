import { useEffect, useRef, useState } from 'react';
import { Text, View } from 'react-native-web';
import { currentLineAt, FOLLOW_IDLE_MS, type SyncedLine } from './synced-lyrics.ts';

/**
 * Distance from the current line beyond which the falloff saturates: the CSS
 * ramp in apps/demo/public/area-lyrics.css styles exactly these steps, and
 * the value is always a small integer, never free text.
 */
const FAR_DISTANCE = 4;

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
  const ownScrollRef = useRef(false);
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
  /* While following, the sounding line stays in view, about a third of the
     way down (MUS-155). The scroll behaviour itself is the stylesheet's —
     smooth where motion is welcome, a jump under reduced motion. */
  useEffect(() => {
    if (!following || !synced || timedLines === undefined) {
      return;
    }
    const scroller = scrollerRef.current;
    const line = scroller?.querySelector<HTMLElement>('[data-current="1"]') ?? null;
    if (line === null || typeof line.scrollIntoView !== 'function') {
      return;
    }
    ownScrollRef.current = true;
    line.scrollIntoView({ block: 'center' });
  }, [following, synced, timedLines, positionMs]);
  if (!open) {
    return null;
  }
  /* The timed sheet replaces the plain one: its texts are the lines and its
     timestamps are the clock. A plain pane ignores it. */
  const sheet = synced && timedLines !== undefined ? timedLines : null;
  const texts = sheet === null ? lines : sheet.map((line) => line.text);
  const active = sheet === null ? currentLine : currentLineAt(sheet, positionMs);
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
          if (ownScrollRef.current) {
            ownScrollRef.current = false;
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
