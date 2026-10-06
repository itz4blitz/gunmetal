import { routes, type Route } from '../routes.ts';

export type MatchOk = {
  kind: 'ok';
  route: Route;
  history: HistoryState;
};

export type MatchNotFound = {
  kind: 'not-found';
};

export type MatchResult = MatchOk | MatchNotFound;

export type HistoryState = {
  scrollY: number;
  itemId: string | undefined;
};

export type AddressParts = {
  pathname: string;
  search: string;
  hash: string;
  state: unknown;
};

// Provisional until WP-235 generated ID types land: only undefined item IDs are accepted.
function parseItemId(value: unknown): { ok: true; itemId: string | undefined } | { ok: false } {
  if (value === undefined) {
    return { ok: true, itemId: undefined };
  }
  return { ok: false };
}

export function parseHistoryState(state: unknown): { ok: true; value: HistoryState } | { ok: false } {
  if (state === null || state === undefined) {
    return { ok: true, value: { scrollY: 0, itemId: undefined } };
  }
  if (typeof state !== 'object') {
    return { ok: false };
  }
  const record = state as { scrollY?: unknown; itemId?: unknown };
  const keys = Object.keys(record);
  for (const key of keys) {
    if (key !== 'scrollY' && key !== 'itemId') {
      return { ok: false };
    }
  }
  let scrollY = 0;
  if (record.scrollY !== undefined) {
    if (typeof record.scrollY !== 'number' || !Number.isFinite(record.scrollY) || record.scrollY < 0) {
      return { ok: false };
    }
    scrollY = record.scrollY;
  }
  const item = parseItemId(record.itemId);
  if (!item.ok) {
    return { ok: false };
  }
  return { ok: true, value: { scrollY, itemId: item.itemId } };
}

// Exact path match only. A query string or fragment refuses the address (CP-009).
export function matchAddress(parts: AddressParts): MatchResult {
  if (parts.search !== '' || parts.hash !== '') {
    return { kind: 'not-found' };
  }
  const route = routes().find((entry) => entry.path === parts.pathname);
  if (route === undefined) {
    return { kind: 'not-found' };
  }
  const history = parseHistoryState(parts.state);
  if (!history.ok) {
    return { kind: 'not-found' };
  }
  return { kind: 'ok', route, history: history.value };
}
