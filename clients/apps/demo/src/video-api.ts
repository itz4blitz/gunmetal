/**
 * The video walking-skeleton's client: every answer is a typed result,
 * never a throw, so the surface can always say what happened in one
 * plain sentence.
 */

import {
  sourcesFromDocument,
  videoFromDocument,
  type SourcesDocument,
  type VideoDocument,
} from './video-types.ts';

export type ApiResult<T> = { ok: true; value: T } | { ok: false; error: string };

type FetchLike = typeof fetch;

async function getJson(url: string, fetchImpl: FetchLike): Promise<ApiResult<unknown>> {
  let response: Response;
  try {
    response = await fetchImpl(url, { headers: { accept: 'application/json' } });
  } catch {
    return { ok: false, error: 'the server could not be reached' };
  }
  if (!response.ok) {
    return { ok: false, error: `the server answered ${response.status}` };
  }
  try {
    return { ok: true, value: await response.json() };
  } catch {
    return { ok: false, error: 'the server answered something that is not json' };
  }
}

async function postJson(
  url: string,
  method: 'POST' | 'DELETE',
  body: unknown | undefined,
  fetchImpl: FetchLike,
): Promise<ApiResult<unknown>> {
  let response: Response;
  try {
    response = await fetchImpl(url, {
      method,
      headers: body === undefined ? { accept: 'application/json' } : { accept: 'application/json', 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    return { ok: false, error: 'the server could not be reached' };
  }
  if (!response.ok) {
    const reason = await response
      .json()
      .then((read) => (typeof read?.error === 'string' ? read.error : undefined))
      .catch(() => undefined);
    return { ok: false, error: reason ?? `the server answered ${response.status}` };
  }
  try {
    return { ok: true, value: await response.json() };
  } catch {
    return { ok: false, error: 'the server answered something that is not json' };
  }
}

/** Reads the configured sources. */
export async function loadSources(fetchImpl: FetchLike = globalThis.fetch): Promise<ApiResult<SourcesDocument>> {
  const read = await getJson('/api/sources', fetchImpl);
  if (!read.ok) {
    return read;
  }
  const parsed = sourcesFromDocument(read.value);
  return parsed === undefined
    ? { ok: false, error: 'the sources list was not the shape this app reads' }
    : { ok: true, value: parsed };
}

/** Adds a source: a name, and a path this container can see. */
export async function addSource(
  name: string,
  path: string,
  fetchImpl: FetchLike = globalThis.fetch,
): Promise<ApiResult<true>> {
  const read = await postJson('/api/sources', 'POST', { name, path }, fetchImpl);
  return read.ok ? { ok: true, value: true } : read;
}

/** Removes a source by its id. */
export async function removeSource(id: string, fetchImpl: FetchLike = globalThis.fetch): Promise<ApiResult<true>> {
  const read = await postJson(`/api/sources/${id}`, 'DELETE', undefined, fetchImpl);
  return read.ok ? { ok: true, value: true } : read;
}

/** Rescans a source by its id. */
export async function rescanSource(id: string, fetchImpl: FetchLike = globalThis.fetch): Promise<ApiResult<true>> {
  const read = await postJson(`/api/sources/${id}/rescan`, 'POST', undefined, fetchImpl);
  return read.ok ? { ok: true, value: true } : read;
}

/** Reads the video document the sources produced. */
export async function loadVideoDocument(fetchImpl: FetchLike = globalThis.fetch): Promise<ApiResult<VideoDocument>> {
  const read = await getJson('/video-library.json', fetchImpl);
  if (!read.ok) {
    return read;
  }
  const parsed = videoFromDocument(read.value);
  return parsed === undefined
    ? { ok: false, error: 'the video list was not the shape this app reads' }
    : { ok: true, value: parsed };
}
