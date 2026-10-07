/**
 * The widths of the two resizable panes: the library sidebar and the queue
 * (design-language §7, CLI-060). Pure arithmetic and parsing — the Shell
 * applies the result and the composition root decides where it is kept.
 *
 * What is stored is layout only: two pixel counts. Nothing here is Activity,
 * Identity or Secret data (SEC-PRV-019), and a stored value is never trusted:
 * anything that does not parse to a finite number inside the limits falls
 * back to the default.
 */
export type PaneId = 'sidebar' | 'queue';

export type PaneWidths = { sidebar: number; queue: number };

export type PaneLimit = { min: number; max: number; fallback: number };

const LIMITS: Record<PaneId, PaneLimit> = {
  sidebar: { min: 200, max: 420, fallback: 256 },
  queue: { min: 280, max: 560, fallback: 340 },
};

export function paneLimit(pane: PaneId): PaneLimit {
  return LIMITS[pane];
}

export function defaultPaneWidths(): PaneWidths {
  return { sidebar: LIMITS.sidebar.fallback, queue: LIMITS.queue.fallback };
}

/** A whole pixel width inside the pane's limits; a non-number is the default. */
export function clampPaneWidth(pane: PaneId, widthPx: unknown): number {
  const limit = LIMITS[pane];
  if (typeof widthPx !== 'number' || !Number.isFinite(widthPx)) {
    return limit.fallback;
  }
  return Math.min(limit.max, Math.max(limit.min, Math.round(widthPx)));
}

/** Reads a stored layout. Any shape other than the one written is the default. */
export function parsePaneWidths(raw: string | null): PaneWidths {
  if (raw === null) {
    return defaultPaneWidths();
  }
  let stored: unknown;
  try {
    stored = JSON.parse(raw);
  } catch {
    return defaultPaneWidths();
  }
  if (typeof stored !== 'object' || stored === null) {
    return defaultPaneWidths();
  }
  const record = stored as Record<string, unknown>;
  return {
    sidebar: clampPaneWidth('sidebar', record['sidebar']),
    queue: clampPaneWidth('queue', record['queue']),
  };
}

export function serializePaneWidths(widths: PaneWidths): string {
  return JSON.stringify({
    sidebar: clampPaneWidth('sidebar', widths.sidebar),
    queue: clampPaneWidth('queue', widths.queue),
  });
}

/**
 * The separator's keyboard model (WAI-ARIA window splitter): arrows move the
 * edge by 16px (48px with Shift), Home and End jump to the limits and Enter
 * returns to the default. `growsToward` is the arrow that widens the pane —
 * right for the sidebar's trailing edge, left for the queue's leading edge.
 * Returns the new width, or undefined when the key is not a resize key.
 */
export function paneWidthForKey(
  pane: PaneId,
  widthPx: number,
  key: string,
  shift: boolean,
  growsToward: 'ArrowRight' | 'ArrowLeft',
): number | undefined {
  const limit = LIMITS[pane];
  const step = shift ? 48 : 16;
  if (key === 'ArrowLeft' || key === 'ArrowRight') {
    return clampPaneWidth(pane, key === growsToward ? widthPx + step : widthPx - step);
  }
  if (key === 'Home') {
    return limit.min;
  }
  if (key === 'End') {
    return limit.max;
  }
  if (key === 'Enter') {
    return limit.fallback;
  }
  return undefined;
}

/**
 * Where the composition root keeps the layout. apps/demo wires the browser's
 * localStorage; a test wires a literal. A store that cannot read or write
 * (blocked storage, private window) simply returns null and drops the write.
 */
export type LayoutStore = {
  read(): string | null;
  write(value: string): void;
};

/** The store of a shell nobody wired one for: remembers nothing. */
export function noLayoutStore(): LayoutStore {
  return {
    read: () => null,
    write: () => undefined,
  };
}
