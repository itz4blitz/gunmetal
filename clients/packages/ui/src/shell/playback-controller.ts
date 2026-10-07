import type { PlayerSnapshot } from '../../../ports/src/provisional/player.ts';

/**
 * The playback contract the shell renders against. The composition root
 * provides the implementation: apps/demo builds one from fixture tones and
 * an <audio> element; apps/web will build one from the real ports (the
 * core's queue verbs through CorePort, server sync through ServerPort).
 * The shell and the components know nothing about which is wired.
 */
export type PlaybackController = {
  /** The player snapshot the bar, full player and queue render. */
  state: PlayerSnapshot;
  /** Whether the full-screen player is open. */
  fullOpen: boolean;
  /** Output volume, 0..1. */
  volume: number;
  setVolume(volume: number): void;
  playAlbum(albumId: string): void;
  playTrack(albumId: string, trackId: string): void;
  playNextAlbum(albumId: string): void;
  addAlbumToQueue(albumId: string): void;
  playNextTrack(albumId: string, trackId: string): void;
  addTrackToQueue(albumId: string, trackId: string): void;
  playPause(): void;
  previous(): void;
  next(): void;
  seek(positionMs: number): void;
  toggleQueue(): void;
  closeQueue(): void;
  openFull(): void;
  closeFull(): void;
};
