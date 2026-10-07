/**
 * PROVISIONAL player display types (client plan CP-005: ports/src/provisional/).
 *
 * The UI's playback state shape until WP-025 (queue document) and WP-030
 * (player state) reach TypeScript through WP-236's generated declarations.
 * This is display state for the shell's components — queue verbs, shuffle
 * orders and the state machine itself are the core's rules and are NOT
 * modelled here.
 */

import type { CatalogueLyricsKind } from './catalogue.ts';

export type PlayerQueueLine = {
  trackId: string;
  albumId: string;
  title: string;
  artistName: string;
  coverTone: string;
  coverUrl: string;
  mediaUrl: string;
  durationMs: number;
  lyricsKind: CatalogueLyricsKind;
};

export type PlayerSnapshot = {
  trackId: string | undefined;
  albumId: string | undefined;
  title: string;
  artistName: string;
  coverTone: string;
  coverUrl: string;
  mediaUrl: string;
  playing: boolean;
  positionMs: number;
  durationMs: number;
  lyricsKind: CatalogueLyricsKind;
  queue: readonly PlayerQueueLine[];
  queueOpen: boolean;
};
