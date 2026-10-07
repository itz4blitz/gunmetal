/**
 * The volume port: what the output level and the mute were last left at,
 * held by the composition root's store (the demo: localStorage, like the
 * pane widths). Only a preference travels here — never a token or history
 * (SEC-PRV-019, SEC-IAM-017). A store that cannot read or write simply is
 * not remembered (the layout store's rule, mirrored).
 */

export type VolumeStore = {
  read(): string | null;
  write(value: string): void;
};

export type VolumeMemory = { volume: number; muted: boolean };

/** Where the output starts when nothing was remembered. */
export const defaultVolumeMemory: VolumeMemory = { volume: 0.8, muted: false };

/** Read one written entry; anything unreadable is the default, not an error. */
export function parseVolume(raw: string | null): VolumeMemory {
  if (raw === null || raw === '') {
    return defaultVolumeMemory;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return defaultVolumeMemory;
  }
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    return defaultVolumeMemory;
  }
  const record = parsed as Record<string, unknown>;
  return {
    volume: parseLevel(record.volume),
    muted: typeof record.muted === 'boolean' ? record.muted : defaultVolumeMemory.muted,
  };
}

function parseLevel(value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return defaultVolumeMemory.volume;
  }
  return Math.max(0, Math.min(1, value));
}

/** Write one entry: the level and the mute, and nothing else. */
export function serializeVolume(memory: VolumeMemory): string {
  return JSON.stringify({ volume: memory.volume, muted: memory.muted });
}
