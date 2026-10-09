/**
 * Playback preferences the demo remembers: volume levelling, how long a
 * crossfade lasts, and which output to use. Only a preference travels
 * here — never a token or history (SEC-PRV-019, SEC-IAM-017). A stored
 * value is never trusted. Invalid JSON, and anything that is not the
 * written object, falls back entirely. Each field falls back on its own
 * when it is not an allowed value. A sink id longer than 200 code units,
 * or one containing NUL, is refused.
 */

export type Levelling = 'off' | 'track' | 'album';

export type CrossfadeSeconds = 0 | 2 | 4 | 6 | 8 | 12;

export type PlaybackPrefs = {
  levelling: Levelling;
  crossfadeSeconds: CrossfadeSeconds;
  sinkId: string;
};

/** Where playback starts when nothing was remembered: levelling off, no crossfade, no sink. */
export function defaultPlaybackPrefs(): PlaybackPrefs {
  return { levelling: 'off', crossfadeSeconds: 0, sinkId: '' };
}

/** Read one written entry. A field that is not allowed falls back alone. */
export function readPlaybackPrefs(raw: string | null): PlaybackPrefs {
  const stored = readStored(raw);
  if (stored === undefined) {
    return defaultPlaybackPrefs();
  }
  return {
    levelling: readLevelling(stored['levelling']),
    crossfadeSeconds: readCrossfade(stored['crossfadeSeconds']),
    sinkId: readSinkId(stored['sinkId']),
  };
}

function readStored(raw: string | null): Record<string, unknown> | undefined {
  if (raw === null) {
    return undefined;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return undefined;
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return undefined;
  }
  return parsed as Record<string, unknown>;
}

function readLevelling(value: unknown): Levelling {
  if (value === 'track') {
    return 'track';
  }
  if (value === 'album') {
    return 'album';
  }
  return 'off';
}

function readCrossfade(value: unknown): CrossfadeSeconds {
  if (value === 2) {
    return 2;
  }
  if (value === 4) {
    return 4;
  }
  if (value === 6) {
    return 6;
  }
  if (value === 8) {
    return 8;
  }
  if (value === 12) {
    return 12;
  }
  return 0;
}

function maxSinkIdLength(): number {
  return 200;
}

function readSinkId(value: unknown): string {
  if (typeof value !== 'string') {
    return '';
  }
  if (value.length > maxSinkIdLength()) {
    return '';
  }
  if (value.includes('\0')) {
    return '';
  }
  return value;
}

/** Write one entry: levelling, crossfade seconds and sink, and nothing else. */
export function serializePlaybackPrefs(prefs: PlaybackPrefs): string {
  return JSON.stringify({
    levelling: prefs.levelling,
    crossfadeSeconds: prefs.crossfadeSeconds,
    sinkId: prefs.sinkId,
  });
}
