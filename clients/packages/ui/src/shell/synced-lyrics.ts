/**
 * The synced-lyrics contract (MUS-154, MUS-155): a resolver may hand the
 * pane timestamped lines instead of plain ones. The pane owns the line
 * clock — which line is sounding at the playing position — and the follow
 * behaviour (manual scroll pauses it, an idle delay resumes it). The
 * `LyricsResolver` in content.ts keeps its plain shape; the composition
 * root passes the timed one beside it.
 */

export type SyncedLine = {
  /** When the line starts, in milliseconds into the track. */
  atMs: number;
  text: string;
};

/** A resolver with the track's timed lines, or undefined when it has none. */
export type TimedLyricsResolver = (trackId: string, kind: 'plain' | 'synced') => readonly SyncedLine[] | undefined;

/** How long the pane waits after a manual scroll before it follows again. */
export const FOLLOW_IDLE_MS = 4_000;

/**
 * The index of the line sounding at the position: the last line that
 * started at or before it, or -1 before the first one starts. An unsorted
 * sheet is read in its own order — the clock reports what the sheet says,
 * and a malformed sheet is the resolver's fault to fix.
 */
export function currentLineAt(lines: readonly SyncedLine[], positionMs: number): number {
  let current = -1;
  let startedAt = -1;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (line !== undefined && line.atMs <= positionMs && line.atMs >= startedAt) {
      startedAt = line.atMs;
      current = index;
    }
  }
  return current;
}
