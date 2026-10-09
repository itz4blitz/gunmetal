/**
 * Sidebar pins: a path from the closed route list, a short label, and for
 * an album or artist the id of that item. What is stored is navigation
 * only — never Activity, Identity or Secret data (SEC-PRV-019). A stored
 * value is never trusted. Invalid JSON, unknown paths, duplicate pins,
 * and labels over 80 characters or containing NUL are dropped. Paths are
 * matched exactly, so a pin cannot name an address outside the shell.
 */

import { extensionPath, extensionRepository } from '../plugins/repository.ts';
import { parseMediaPath } from '../router/media-path.ts';

export type Pin = {
  path: string;
  label: string;
  /** An album or artist on `/library`. Absent for a page pin. */
  itemId?: string;
};

/** The routes a pin may name. `/settings` is the appearance address, as in `routes()`. */
function pinPaths(): readonly string[] {
  return [
    '/',
    '/search',
    '/library',
    '/store',
    '/settings',
    '/settings/appearance',
    '/settings/playback',
    '/settings/connected',
    '/settings/extensions',
    ...extensionRepository().extensions.map((entry) => extensionPath(entry.id)),
    '/settings/about',
    '/settings/privacy',
  ];
}

function maxPinLabelLength(): number {
  return 80;
}

function isPinPath(path: string): boolean {
  return pinPaths().some((allowed) => allowed === path) || parseMediaPath(path) !== undefined;
}

function isPinLabel(label: string): boolean {
  return label.length <= maxPinLabelLength() && !label.includes('\0');
}

function isItemId(value: string): boolean {
  return /^[a-z0-9-]{1,64}$/.test(value);
}

function samePin(left: Pin, right: Pin): boolean {
  return left.path === right.path && left.itemId === right.itemId;
}

function storedPin(entry: unknown): Pin | undefined {
  if (typeof entry !== 'object' || entry === null) {
    return undefined;
  }
  const record = entry as Record<string, unknown>;
  const path = record['path'];
  const label = record['label'];
  const itemId = record['itemId'];
  if (typeof path !== 'string' || typeof label !== 'string') {
    return undefined;
  }
  if (!isPinPath(path) || !isPinLabel(label)) {
    return undefined;
  }
  if (itemId === undefined) {
    return { path, label };
  }
  if (typeof itemId !== 'string' || !isItemId(itemId) || path !== '/library') {
    return undefined;
  }
  return { path, label, itemId };
}

/** Read one stored list. Anything unreadable or not a pin is dropped, not an error. */
export function readPins(raw: string | null): readonly Pin[] {
  if (raw === null) {
    return [];
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return [];
  }
  if (!Array.isArray(parsed)) {
    return [];
  }
  const kept: Pin[] = [];
  for (const entry of parsed) {
    const pin = storedPin(entry);
    if (pin === undefined) {
      continue;
    }
    if (kept.some((existing) => samePin(existing, pin))) {
      continue;
    }
    kept.push(pin);
  }
  return kept;
}

/**
 * Add the pin when that page is absent, or remove the one that matches.
 * A page pin matches on path. An album or artist also matches on item id,
 * so two albums are two pins. A path outside the closed list is refused:
 * the result has the same pins, and the input array is not written.
 */
export function togglePin(pins: readonly Pin[], pin: Pin): readonly Pin[] {
  if (!isPinPath(pin.path)) {
    return pins.slice();
  }
  if (pin.itemId !== undefined && (pin.path !== '/library' || !isItemId(pin.itemId))) {
    return pins.slice();
  }
  const without = pins.filter((entry) => !samePin(entry, pin));
  if (without.length === pins.length) {
    const next: Pin = { path: pin.path, label: pin.label };
    if (pin.itemId !== undefined) {
      next.itemId = pin.itemId;
    }
    return without.concat([next]);
  }
  return without;
}

/** Write the list as a JSON array of path, label, and item id, and nothing else. */
export function serializePins(pins: readonly Pin[]): string {
  const entries: Pin[] = [];
  for (const pin of pins) {
    const next: Pin = { path: pin.path, label: pin.label };
    if (pin.itemId !== undefined) {
      next.itemId = pin.itemId;
    }
    entries.push(next);
  }
  return JSON.stringify(entries);
}
