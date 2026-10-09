import {
  demoLibrary,
  demoLyricsLines,
  demoLyricsVerse,
  demoLocalFilter,
  hostServesCovers,
  noLyricsLine,
  pluginSlots,
  trackById,
  type DemoLibrary,
} from '../../../packages/fake-server/src/index.ts';
import type { LyricsResolver } from '../../../packages/ui/src/shell/content.ts';
import type { SyncedLine, TimedLyricsResolver } from '../../../packages/ui/src/shell/synced-lyrics.ts';

export type DemoCompose = {
  showDemoLabel: boolean;
  library: DemoLibrary;
  searchLibrary: typeof demoLocalFilter;
  /** Fixture verses through the resolver the shell expects (server lyrics later). */
  lyricsFor: LyricsResolver;
  /**
   * Timed lines for the same fixtures: the verse spread evenly across the
   * track's fixture duration, so the demo shows the line clock moving. The
   * server's real LRC rows replace this when CorePort lands.
   */
  timedLyricsFor: TimedLyricsResolver;
  /** The slot table the Settings Extensions page renders (server config later). */
  pluginSlots: ReturnType<typeof pluginSlots>;
};

/** One verse as timed lines: even steps across the track, first line off zero. */
export function timedVerse(verse: readonly string[], durationMs: number): readonly SyncedLine[] {
  const step = durationMs > 0 ? durationMs / (verse.length + 1) : 1_000;
  return verse.map((text, index) => ({ atMs: Math.round((index + 1) * step), text }));
}

export function composeDemo(library: DemoLibrary = demoLibrary()): DemoCompose {
  const served = library.kind === 'folder';
  return {
    showDemoLabel: !served,
    library,
    searchLibrary: demoLocalFilter,
    lyricsFor: (trackId, kind) => (served ? [noLyricsLine()] : demoLyricsLines(demoLyricsVerse(trackId, kind))),
    timedLyricsFor: (trackId, kind) => {
      if (served) {
        return [];
      }
      const durationMs = trackById(library, trackId)?.durationMs ?? 0;
      return timedVerse(demoLyricsLines(demoLyricsVerse(trackId, kind)), durationMs);
    },
    pluginSlots: pluginSlots(hostServesCovers(library.albums)),
  };
}
