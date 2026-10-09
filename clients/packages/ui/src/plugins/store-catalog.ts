/**
 * The published store index. A merged pull request lists a record here.
 * Parsing it does not install or run anything (SEC-EXT-018).
 */

const ID = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const VERSION = /^(?:0|[1-9]\d{0,2})\.(?:0|[1-9]\d{0,2})\.(?:0|[1-9]\d{0,2})$/;
const SLOT = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const GRANT = /^[a-z0-9]+(?:-[a-z0-9]+)*:[a-z0-9]+(?:-[a-z0-9]+)*$/;
const ENTRY_KEYS = ['detail', 'grants', 'id', 'plane', 'slot', 'status', 'summary', 'title', 'version'] as const;

export type StoreListing = {
  id: string;
  title: string;
  version: string;
  plane: 'server' | 'client';
  slot: string;
  status: 'on' | 'not-in-build';
  summary: string;
  detail: readonly string[];
  grants: readonly string[];
};

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function sentence(value: unknown, limit: number): value is string {
  return typeof value === 'string' && value.trim().length > 0 && value.length <= limit;
}

function entry(value: unknown): StoreListing | undefined {
  if (!record(value)) {
    return undefined;
  }
  const keys = Object.keys(value).sort();
  if (keys.length !== ENTRY_KEYS.length || keys.some((key, index) => key !== ENTRY_KEYS[index])) {
    return undefined;
  }
  const id = value.id;
  const plane = value.plane;
  const status = value.status;
  const detail = value.detail;
  const grants = value.grants;
  if (typeof id !== 'string' || ID.test(id) === false || id.length > 64) {
    return undefined;
  }
  if (!sentence(value.title, 80) || !sentence(value.summary, 200)) {
    return undefined;
  }
  if (typeof value.version !== 'string' || VERSION.test(value.version) === false) {
    return undefined;
  }
  if (plane !== 'server' && plane !== 'client') {
    return undefined;
  }
  if (typeof value.slot !== 'string' || SLOT.test(value.slot) === false) {
    return undefined;
  }
  if (status !== 'on' && status !== 'not-in-build') {
    return undefined;
  }
  if (!Array.isArray(detail) || detail.length === 0 || detail.length > 8 || detail.some((line) => !sentence(line, 240))) {
    return undefined;
  }
  if (!Array.isArray(grants) || grants.length === 0 || grants.length > 8) {
    return undefined;
  }
  const seen = new Set<string>();
  for (const grant of grants) {
    if (typeof grant !== 'string' || GRANT.test(grant) === false || seen.has(grant)) {
      return undefined;
    }
    seen.add(grant);
  }
  return {
    id,
    title: value.title,
    version: value.version,
    plane,
    slot: value.slot,
    status,
    summary: value.summary,
    detail,
    grants,
  };
}

/** The records in a published catalog, or undefined when the text is not one. */
export function parseStoreCatalog(text: string): readonly StoreListing[] | undefined {
  let value: unknown;
  try {
    value = JSON.parse(text) as unknown;
  } catch {
    return undefined;
  }
  if (!record(value)) {
    return undefined;
  }
  const keys = Object.keys(value).sort();
  if (keys.length !== 3 || keys[0] !== 'extensions' || keys[1] !== 'id' || keys[2] !== 'version') {
    return undefined;
  }
  if (value.id !== 'gunmetal.extensions' || value.version !== '1') {
    return undefined;
  }
  if (!Array.isArray(value.extensions) || value.extensions.length === 0 || value.extensions.length > 64) {
    return undefined;
  }
  const listings: StoreListing[] = [];
  const seen = new Set<string>();
  for (const item of value.extensions) {
    const parsed = entry(item);
    if (parsed === undefined || seen.has(parsed.id)) {
      return undefined;
    }
    seen.add(parsed.id);
    listings.push(parsed);
  }
  return listings;
}

/** The same-origin store index. A failure leaves the built-in list in place. */
export async function loadPublishedStore(): Promise<readonly StoreListing[] | undefined> {
  if (typeof fetch !== 'function') {
    return undefined;
  }
  try {
    const response = await fetch('/extensions/catalog.json', { cache: 'no-store' });
    if (!response.ok) {
      return undefined;
    }
    return parseStoreCatalog(await response.text());
  } catch {
    return undefined;
  }
}
