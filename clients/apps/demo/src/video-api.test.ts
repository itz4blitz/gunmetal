import { expect, test, vi } from 'vitest';
import { addSource, loadSources, loadVideoDocument, removeSource, rescanSource } from './video-api.ts';
import { sourcesFromDocument, videoFromDocument } from './video-types.ts';

const sourcesBody = { sources: [{ id: 'a1a1a1a1a1a1a1a1', name: 'Movies', path: '/media/movies', titles: 1 }] };
const videoBody = {
  kind: 'video',
  titles: [
    {
      id: 'c3c3c3c3c3c3c3c3',
      title: 'Harbour Lights',
      kind: 'movie',
      container: '.mp4',
      bytes: 1,
      modified: '2026-10-01T00:00:00.000Z',
    },
  ],
};

function jsonResponse(body: unknown, status: number): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

test('the sources list is read on a good answer', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse(sourcesBody, 200));
  const read = await loadSources(fetchImpl);
  expect(read).toStrictEqual({ ok: true, value: sourcesFromDocument(sourcesBody)! });
  expect(fetchImpl).toHaveBeenCalledWith('/api/sources', { headers: { accept: 'application/json' } });
});

test('a refused answer names the status', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse({ sources: [] }, 500));
  expect(await loadSources(fetchImpl)).toStrictEqual({ ok: false, error: 'the server answered 500' });
});

test('an unreachable server says so', async () => {
  const fetchImpl = vi.fn().mockRejectedValue(new Error('offline'));
  expect(await loadSources(fetchImpl)).toStrictEqual({ ok: false, error: 'the server could not be reached' });
});

test('an answer that is not json says so', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(new Response('not json', { status: 200 }));
  expect(await loadSources(fetchImpl)).toStrictEqual({
    ok: false,
    error: 'the server answered something that is not json',
  });
});

test('a sources document of another shape is refused by name', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse({ sources: 'movies' }, 200));
  expect(await loadSources(fetchImpl)).toStrictEqual({
    ok: false,
    error: 'the sources list was not the shape this app reads',
  });
});

test('the video document is read on a good answer', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse(videoBody, 200));
  const read = await loadVideoDocument(fetchImpl);
  expect(read).toStrictEqual({ ok: true, value: videoFromDocument(videoBody)! });
});

test('a video document of another shape is refused by name', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse({ kind: 'music' }, 200));
  expect(await loadVideoDocument(fetchImpl)).toStrictEqual({
    ok: false,
    error: 'the video list was not the shape this app reads',
  });
});

test('a video document over a refused answer names the status', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(new Response('no', { status: 404 }));
  expect(await loadVideoDocument(fetchImpl)).toStrictEqual({ ok: false, error: 'the server answered 404' });
});

test('adding a source posts the name and path as json', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse({}, 201));
  expect(await addSource('Movies', '/media/movies', fetchImpl)).toStrictEqual({ ok: true, value: true });
  expect(fetchImpl).toHaveBeenCalledWith('/api/sources', {
    method: 'POST',
    headers: { accept: 'application/json', 'content-type': 'application/json' },
    body: JSON.stringify({ name: 'Movies', path: '/media/movies' }),
  });
});

test('a refused add carries the server reason when it has one', async () => {
  const fetchImpl = vi
    .fn()
    .mockResolvedValue(jsonResponse({ error: 'the path is not a directory this container can see' }, 400));
  expect(await addSource('Movies', '/nowhere', fetchImpl)).toStrictEqual({
    ok: false,
    error: 'the path is not a directory this container can see',
  });
});

test('a refused add without a reason names the status', async () => {
  const plain = vi.fn().mockResolvedValue(new Response('bad', { status: 400 }));
  expect(await addSource('Movies', '/nowhere', plain)).toStrictEqual({
    ok: false,
    error: 'the server answered 400',
  });
  const emptyJson = vi.fn().mockResolvedValue(jsonResponse({}, 409));
  expect(await addSource('Movies', '/media/movies', emptyJson)).toStrictEqual({
    ok: false,
    error: 'the server answered 409',
  });
});

test('an unreachable add says so', async () => {
  const fetchImpl = vi.fn().mockRejectedValue(new Error('offline'));
  expect(await addSource('Movies', '/media/movies', fetchImpl)).toStrictEqual({
    ok: false,
    error: 'the server could not be reached',
  });
});

test('an add that answers not-json says so', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(new Response('created', { status: 201 }));
  expect(await addSource('Movies', '/media/movies', fetchImpl)).toStrictEqual({
    ok: false,
    error: 'the server answered something that is not json',
  });
});

test('removing a source deletes its route', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse({ sources: [] }, 200));
  expect(await removeSource('a1a1a1a1a1a1a1a1', fetchImpl)).toStrictEqual({ ok: true, value: true });
  expect(fetchImpl).toHaveBeenCalledWith('/api/sources/a1a1a1a1a1a1a1a1', {
    method: 'DELETE',
    headers: { accept: 'application/json' },
    body: undefined,
  });
});

test('a refused remove carries the reason', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse({ error: 'no such source' }, 404));
  expect(await removeSource('b2b2b2b2b2b2b2b2', fetchImpl)).toStrictEqual({
    ok: false,
    error: 'no such source',
  });
});

test('rescanning a source posts its route', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse({}, 200));
  expect(await rescanSource('a1a1a1a1a1a1a1a1', fetchImpl)).toStrictEqual({ ok: true, value: true });
  expect(fetchImpl).toHaveBeenCalledWith('/api/sources/a1a1a1a1a1a1a1a1/rescan', {
    method: 'POST',
    headers: { accept: 'application/json' },
    body: undefined,
  });
});

test('a refused rescan carries the reason', async () => {
  const fetchImpl = vi.fn().mockResolvedValue(jsonResponse({ error: 'no such source' }, 404));
  expect(await rescanSource('b2b2b2b2b2b2b2b2', fetchImpl)).toStrictEqual({
    ok: false,
    error: 'no such source',
  });
});
