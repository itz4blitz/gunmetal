import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { WatchArea } from './WatchArea.tsx';
import * as api from './video-api.ts';
import type { SourcesDocument, VideoDocument } from './video-types.ts';

vi.mock('./video-api.ts');
const mocked = vi.mocked(api);

afterEach(cleanup);

const emptySources: SourcesDocument = { sources: [] };
const oneSource: SourcesDocument = {
  sources: [{ id: 'a1a1a1a1a1a1a1a1', name: 'Movies', path: '/media/movies', titles: 2 }],
};
const emptyVideo: VideoDocument = { kind: 'video', titles: [] };
const twoTitles: VideoDocument = {
  kind: 'video',
  titles: [
    {
      id: 'c3c3c3c3c3c3c3c3',
      title: 'Harbour Lights',
      kind: 'movie',
      container: '.mp4',
      bytes: 4_000_000_000,
      modified: '2026-10-01T00:00:00.000Z',
    },
    {
      id: 'd4d4d4d4d4d4d4d4',
      title: 'Pilot',
      kind: 'episode',
      show: 'The Dock',
      season: 1,
      episode: 1,
      container: '.mkv',
      bytes: 500_000_000,
      modified: '2026-10-02T00:00:00.000Z',
    },
  ],
};

beforeEach(() => {
  mocked.loadSources.mockReset();
  mocked.loadVideoDocument.mockReset();
  mocked.addSource.mockReset();
  mocked.removeSource.mockReset();
  mocked.rescanSource.mockReset();
  mocked.loadSources.mockResolvedValue({ ok: true, value: emptySources });
  mocked.loadVideoDocument.mockResolvedValue({ ok: true, value: emptyVideo });
});

test('with no sources the area points at the settings libraries', async () => {
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('No libraries yet. Add one in Settings → Libraries.')).not.toBeNull();
  });
});

test('a source with no files yet says so', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('No video files found in the configured libraries yet.')).not.toBeNull();
  });
});

test('the titles list as cards with kind badges and meta', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  mocked.loadVideoDocument.mockResolvedValue({ ok: true, value: twoTitles });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('Harbour Lights')).not.toBeNull();
  });
  expect(screen.getByText('Film · 4.0 GB')).not.toBeNull();
  expect(screen.getByText('The Dock · S1 E1 · 500 MB')).not.toBeNull();
  expect(screen.getByText('S1 E1')).not.toBeNull();
});

test('a browser-playable title opens the player; a matroska one says why not', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  mocked.loadVideoDocument.mockResolvedValue({ ok: true, value: twoTitles });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('Harbour Lights')).not.toBeNull();
  });
  fireEvent.click(screen.getByRole('button', { name: /Harbour Lights/ }));
  const video = document.querySelector('#watch-player video');
  expect(video?.getAttribute('src')).toStrictEqual('/media/video/c3c3c3c3c3c3c3c3');
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  expect(document.querySelector('#watch-player')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: /Pilot/ }));
  expect(document.querySelector('#watch-player video')).toBeNull();
  expect(screen.getByText(/will not open in a browser directly/)).not.toBeNull();
});
