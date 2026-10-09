import { routes, type Route } from '../routes.ts';
import { parseMediaPath, type MediaKind, type MediaRoute } from './media-path.ts';

export type MatchOk = {
  kind: 'ok';
  route: Route;
  history: HistoryState;
  /** Set when the path is a media address. Absent for a static page. */
  media?: MediaRoute;
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

export type ParentAddress = {
  path: string | undefined;
  search: string;
  hash: string;
  state: unknown;
};

// `/settings` is the appearance section. Every other path is already canonical.
export function canonicalPath(path: string): string {
  if (path === '/settings') {
    return '/settings/appearance';
  }
  return path;
}

function closedPath(path: string): boolean {
  return routes().some((entry) => entry.path === path) || parseMediaPath(path) !== undefined;
}

// A parent path is a controlled preview. It must not hide a closed route the
// browser is already on. `/` is also the document default, so a parent path
// still applies there and on an address that is not a route.
export function browserWins(livePath: string, parentPath: string | undefined): boolean {
  if (parentPath === undefined) {
    return true;
  }
  return livePath !== '/' && livePath !== parentPath && closedPath(livePath);
}

export function addressOnLoad(live: AddressParts, parent: ParentAddress): AddressParts {
  if (parent.path !== undefined && !browserWins(live.pathname, parent.path)) {
    return {
      pathname: canonicalPath(parent.path),
      search: parent.search,
      hash: parent.hash,
      state: parent.state,
    };
  }
  return {
    pathname: canonicalPath(live.pathname),
    search: live.search,
    hash: live.hash,
    state: live.state,
  };
}

// Provisional opaque IDs until WP-235 generated ID types land. Kept out of the URL (SEC-CLI-025).
function parseItemId(value: unknown): { ok: true; itemId: string | undefined } | { ok: false } {
  if (value === undefined) {
    return { ok: true, itemId: undefined };
  }
  if (typeof value !== 'string' || value.length === 0 || value.length > 128) {
    return { ok: false };
  }
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code < 0x21 || code > 0x7e) {
      return { ok: false };
    }
    const char = value[index];
    if (char === '/' || char === '\\' || char === '?' || char === '#') {
      return { ok: false };
    }
  }
  return { ok: true, itemId: value };
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

// Static paths match exactly. A media path is one closed parameter.
// A query string or fragment refuses the address (CP-009, SEC-CLI-025).
export function matchAddress(parts: AddressParts): MatchResult {
  if (parts.search !== '' || parts.hash !== '') {
    return { kind: 'not-found' };
  }
  const media = parseMediaPath(parts.pathname);
  const route =
    media === undefined
      ? routes().find((entry) => entry.path === parts.pathname)
      : { path: media.path, surface: surfaceFor(media.kind), needsSession: true, needsAdminSession: false };
  if (route === undefined) {
    return { kind: 'not-found' };
  }
  const history = parseHistoryState(parts.state);
  if (!history.ok) {
    return { kind: 'not-found' };
  }
  if (media === undefined) {
    return { kind: 'ok', route, history: history.value };
  }
  return { kind: 'ok', route, history: { scrollY: 0, itemId: undefined }, media };
}

function surfaceFor(kind: MediaKind): string {
  if (kind === 'artist') {
    return 'SUR-024';
  }
  if (kind === 'album') {
    return 'SUR-025';
  }
  if (kind === 'track') {
    return 'SUR-016';
  }
  if (kind === 'show') {
    return 'SUR-041';
  }
  return 'SUR-040';
}
