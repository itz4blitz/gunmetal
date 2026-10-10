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

test('with no sources the area says how to add one', async () => {
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText(/No sources yet/)).not.toBeNull();
  });
  expect(screen.getByText('Add a source to see your media here.')).not.toBeNull();
});

test('a source lists its path and title count, and its files list as cards', async () => {
  mocked.loadSources.mockResolvedValue({
    ok: true,
    value: {
      sources: [
        ...oneSource.sources,
        { id: 'b2b2b2b2b2b2b2b2', name: 'Shows', path: '/media/tvshows', titles: 1 },
      ],
    },
  });
  mocked.loadVideoDocument.mockResolvedValue({ ok: true, value: twoTitles });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('Harbour Lights')).not.toBeNull();
  });
  expect(screen.getByText('Movies')).not.toBeNull();
  expect(screen.getByText('/media/movies')).not.toBeNull();
  expect(screen.getByText('2 titles')).not.toBeNull();
  expect(screen.getByText('1 title')).not.toBeNull();
  expect(screen.getByText('Film · 4.0 GB')).not.toBeNull();
  expect(screen.getByText('The Dock · S1 E1 · 500 MB')).not.toBeNull();
  expect(screen.getByText('S1 E1')).not.toBeNull();
});

test('a source with no files yet says so', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('No video files found in the configured sources yet.')).not.toBeNull();
  });
});

test('an unreachable server puts its reason on the page', async () => {
  mocked.loadSources.mockResolvedValue({ ok: false, error: 'the server could not be reached' });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual('the server could not be reached');
  });
});

test('a refused video document puts its reason on the page beside a good sources list', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: emptySources });
  mocked.loadVideoDocument.mockResolvedValue({ ok: false, error: 'the video list was not the shape this app reads' });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual(
      'the video list was not the shape this app reads',
    );
  });
});

test('adding a source posts the form and reloads', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: emptySources });
  mocked.addSource.mockResolvedValue({ ok: true, value: true });
  mocked.loadSources.mockResolvedValueOnce({ ok: true, value: emptySources });
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText(/No sources yet/)).not.toBeNull();
  });
  fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Movies' } });
  fireEvent.change(screen.getByLabelText('Path in this container'), { target: { value: '/media/movies' } });
  fireEvent.click(screen.getByRole('button', { name: 'Add source' }));
  await waitFor(() => {
    expect(screen.getByText('/media/movies')).not.toBeNull();
  });
  expect(mocked.addSource).toHaveBeenCalledWith('Movies', '/media/movies');
});

test('a refused add keeps the reason on the page', async () => {
  mocked.addSource.mockResolvedValue({ ok: false, error: 'the path is not a directory this container can see' });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Add source' })).not.toBeNull();
  });
  fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Movies' } });
  fireEvent.change(screen.getByLabelText('Path in this container'), { target: { value: '/nowhere' } });
  fireEvent.click(screen.getByRole('button', { name: 'Add source' }));
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual(
      'the path is not a directory this container can see',
    );
  });
});

test('a refused rescan keeps the reason on the page', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  mocked.rescanSource.mockResolvedValue({ ok: false, error: 'no such source' });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('Movies')).not.toBeNull();
  });
  fireEvent.click(screen.getByRole('button', { name: 'Rescan' }));
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual('no such source');
  });
});

test('removing and rescanning a source call their routes', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  mocked.removeSource.mockResolvedValue({ ok: true, value: true });
  mocked.rescanSource.mockResolvedValue({ ok: true, value: true });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('Movies')).not.toBeNull();
  });
  fireEvent.click(screen.getByRole('button', { name: 'Rescan' }));
  await waitFor(() => {
    expect(mocked.rescanSource).toHaveBeenCalledWith('a1a1a1a1a1a1a1a1');
  });
  fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
  await waitFor(() => {
    expect(mocked.removeSource).toHaveBeenCalledWith('a1a1a1a1a1a1a1a1');
  });
});

test('a refused remove keeps the reason on the page', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  mocked.removeSource.mockResolvedValue({ ok: false, error: 'no such source' });
  render(<WatchArea />);
  await waitFor(() => {
    expect(screen.getByText('Movies')).not.toBeNull();
  });
  fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual('no such source');
  });
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
