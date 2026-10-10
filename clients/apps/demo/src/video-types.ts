/**
 * The video walking-skeleton's document types, validated as strictly as
 * the folder library is (served-library.ts). The server's answers are
 * data from another process: anything that is not this shape is refused
 * and the caller keeps its empty state.
 */

export type LibraryKind = 'music' | 'movies' | 'shows';

export type VideoSource = {
  id: string;
  name: string;
  path: string;
  kind: LibraryKind;
  items: number;
};

export type SourcesDocument = {
  sources: readonly VideoSource[];
};

export type VideoTitle = {
  id: string;
  title: string;
  kind: 'movie' | 'episode';
  /** Present on episodes only: the show, season and episode numbers. */
  show?: string;
  season?: number;
  episode?: number;
  container: string;
  bytes: number;
  modified: string;
};

export type VideoDocument = {
  kind: 'video';
  titles: readonly VideoTitle[];
};

const HEX_ID = /^[a-f0-9]{16}$/;
const MAX_TEXT = 200;
const MAX_SOURCES = 64;
const MAX_TITLES = 20_000;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function textOf(value: unknown, max: number): string | undefined {
  if (typeof value !== 'string' || value === '' || value.length > max) {
    return undefined;
  }
  return value;
}

function countOf(value: unknown, max: number): number | undefined {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0 || value > max) {
    return undefined;
  }
  return value;
}

function sourceOf(entry: unknown): VideoSource | undefined {
  if (!isRecord(entry)) {
    return undefined;
  }
  const id = textOf(entry.id, 16);
  const name = textOf(entry.name, MAX_TEXT);
  const path = textOf(entry.path, MAX_TEXT);
  const items = countOf(entry.items, MAX_TITLES);
  const kind = libraryKind(entry.kind);
  if (
    id === undefined ||
    !HEX_ID.test(id) ||
    name === undefined ||
    path === undefined ||
    !path.startsWith('/') ||
    kind === undefined ||
    items === undefined
  ) {
    return undefined;
  }
  return { id, name, path, kind, items };
}

function libraryKind(value: unknown): LibraryKind | undefined {
  if (value === 'music' || value === 'movies' || value === 'shows') {
    return value;
  }
  return undefined;
}

/** Reads `/api/sources`. Anything not this shape is `undefined`. */
export function sourcesFromDocument(value: unknown): SourcesDocument | undefined {
  if (!isRecord(value) || !Array.isArray(value.sources) || value.sources.length > MAX_SOURCES) {
    return undefined;
  }
  const sources: VideoSource[] = [];
  const seen = new Set<string>();
  for (const entry of value.sources) {
    const source = sourceOf(entry);
    if (source === undefined || seen.has(source.id)) {
      return undefined;
    }
    seen.add(source.id);
    sources.push(source);
  }
  return { sources };
}

function titleOfEntry(entry: unknown): VideoTitle | undefined {
  if (!isRecord(entry)) {
    return undefined;
  }
  const id = textOf(entry.id, 16);
  const title = textOf(entry.title, MAX_TEXT);
  const container = textOf(entry.container, 8);
  const bytes = countOf(entry.bytes, Number.MAX_SAFE_INTEGER);
  const modified = textOf(entry.modified, 40);
  if (
    id === undefined ||
    !HEX_ID.test(id) ||
    title === undefined ||
    container === undefined ||
    bytes === undefined ||
    modified === undefined
  ) {
    return undefined;
  }
  if (entry.kind === 'movie') {
    return { id, title, kind: 'movie', container, bytes, modified };
  }
  if (entry.kind !== 'episode') {
    return undefined;
  }
  const show = textOf(entry.show, MAX_TEXT);
  const season = countOf(entry.season, 2100);
  const episode = countOf(entry.episode, 9999);
  if (show === undefined || season === undefined || episode === undefined) {
    return undefined;
  }
  return { id, title, kind: 'episode', show, season, episode, container, bytes, modified };
}

/** Reads `/video-library.json`. Anything not this shape is `undefined`. */
export function videoFromDocument(value: unknown): VideoDocument | undefined {
  if (!isRecord(value) || value.kind !== 'video' || !Array.isArray(value.titles) || value.titles.length > MAX_TITLES) {
    return undefined;
  }
  const titles: VideoTitle[] = [];
  const seen = new Set<string>();
  for (const entry of value.titles) {
    const title = titleOfEntry(entry);
    if (title === undefined || seen.has(title.id)) {
      return undefined;
    }
    seen.add(title.id);
    titles.push(title);
  }
  return { kind: 'video', titles };
}

/** Whether a browser can open this container itself. */
export function browserPlayable(container: string): boolean {
  return container === '.mp4' || container === '.m4v' || container === '.webm';
}
