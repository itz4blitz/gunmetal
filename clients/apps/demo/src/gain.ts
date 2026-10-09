// Amplitude for an audio element from a gain tag the host already extracted.
// The caller passes the mode. The core owns album-versus-track policy beyond
// that fallback; this only turns the chosen tag into a 0..1 level.

/** 10^(dB/20). This does not clamp; outputVolume clamps the tag first. */
export function linearFromDb(db: number): number {
  return 10 ** (db / 20);
}

function finiteTag(value: number | undefined): number | undefined {
  if (!Number.isFinite(value)) {
    return undefined;
  }
  return value;
}

/** A level the audio element can take. NaN is not a level; the infinities clamp. */
function clampUnit(value: number): number {
  if (Number.isNaN(value)) {
    return 0;
  }
  return Math.min(1, Math.max(0, value));
}

export function outputVolume(
  userVolume: number,
  mode: 'off' | 'track' | 'album',
  trackDb: number | undefined,
  albumDb: number | undefined,
): number {
  if (mode === 'off') {
    return clampUnit(userVolume);
  }
  const track = finiteTag(trackDb);
  const album = finiteTag(albumDb);
  const tag = mode === 'track' ? (track ?? album ?? 0) : (album ?? track ?? 0);
  const clamped = Math.min(6, Math.max(-12, tag));
  return clampUnit(userVolume * linearFromDb(clamped));
}
