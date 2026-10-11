import { expect, test } from 'vitest';
import { browserPlayable, sourcesFromDocument, videoFromDocument } from './video-types.ts';

const source = { id: 'a1a1a1a1a1a1a1a1', name: 'Movies', path: '/media/movies', titles: 3 };

test('a sources document is read whole', () => {
  expect(sourcesFromDocument({ sources: [source] })).toStrictEqual({ sources: [source] });
  expect(sourcesFromDocument({ sources: [] })).toStrictEqual({ sources: [] });
});

test('anything not a sources document is refused', () => {
  expect(sourcesFromDocument(undefined)).toBeUndefined();
  expect(sourcesFromDocument(null)).toBeUndefined();
  expect(sourcesFromDocument('sources')).toBeUndefined();
  expect(sourcesFromDocument({})).toBeUndefined();
  expect(sourcesFromDocument({ sources: 'movies' })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [null] })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [{ ...source, id: 'not-hex' }] })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [{ ...source, id: '' }] })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [{ ...source, name: '' }] })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [{ ...source, path: 'media/milms' }] })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [{ ...source, titles: -1 }] })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [{ ...source, titles: 1.5 }] })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [{ ...source, titles: 'three' }] })).toBeUndefined();
  expect(sourcesFromDocument({ sources: [source, { ...source }] })).toBeUndefined();
  expect(
    sourcesFromDocument({ sources: Array.from({ length: 65 }, () => source) }),
  ).toBeUndefined();
});

const movie = {
  id: 'c3c3c3c3c3c3c3c3',
  title: 'Harbour Lights',
  kind: 'movie',
  container: '.mp4',
  bytes: 4_000_000_000,
  modified: '2026-10-01T00:00:00.000Z',
};

const episode = {
  id: 'd4d4d4d4d4d4d4d4',
  title: 'Pilot',
  kind: 'episode',
  show: 'The Dock',
  season: 1,
  episode: 1,
  container: '.mkv',
  bytes: 2_000_000_000,
  modified: '2026-10-02T00:00:00.000Z',
};

test('a video document is read whole', () => {
  expect(videoFromDocument({ kind: 'video', titles: [movie, episode] })).toStrictEqual({
    kind: 'video',
    titles: [movie, episode],
  });
});

test('anything not a video document is refused', () => {
  expect(videoFromDocument(undefined)).toBeUndefined();
  expect(videoFromDocument({ kind: 'music', titles: [] })).toBeUndefined();
  expect(videoFromDocument({ kind: 'video', titles: 'films' })).toBeUndefined();
  expect(videoFromDocument({ kind: 'video', titles: [null] })).toBeUndefined();
  expect(videoFromDocument({ kind: 'video', titles: [{ ...movie, kind: 'series' }] })).toBeUndefined();
  expect(videoFromDocument({ kind: 'video', titles: [{ ...movie, id: 'zz' }] })).toBeUndefined();
  expect(videoFromDocument({ kind: 'video', titles: [{ ...movie, title: '' }] })).toBeUndefined();
  expect(videoFromDocument({ kind: 'video', titles: [{ ...movie, container: '' }] })).toBeUndefined();
  expect(videoFromDocument({ kind: 'video', titles: [{ ...movie, bytes: -1 }] })).toBeUndefined();
  expect(videoFromDocument({ kind: 'video', titles: [{ ...movie, modified: 5 }] })).toBeUndefined();
  expect(
    videoFromDocument({
      kind: 'video',
      titles: [{ ...episode, show: undefined }],
    }),
  ).toBeUndefined();
  expect(
    videoFromDocument({
      kind: 'video',
      titles: [{ ...episode, season: 'one' }],
    }),
  ).toBeUndefined();
  expect(
    videoFromDocument({
      kind: 'video',
      titles: [{ ...episode, episode: 0.5 }],
    }),
  ).toBeUndefined();
  expect(
    videoFromDocument({
      kind: 'video',
      titles: [{ ...movie }, { ...movie, id: movie.id }],
    }),
  ).toBeUndefined();
});

test('a browser plays the containers it plays', () => {
  expect(browserPlayable('.mp4')).toStrictEqual(true);
  expect(browserPlayable('.m4v')).toStrictEqual(true);
  expect(browserPlayable('.webm')).toStrictEqual(true);
  expect(browserPlayable('.mkv')).toStrictEqual(false);
  expect(browserPlayable('.mov')).toStrictEqual(false);
});
