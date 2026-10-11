import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { SourcesPane } from './SourcesPane.tsx';
import * as api from './video-api.ts';
import type { SourcesDocument } from './video-types.ts';

vi.mock('./video-api.ts');
const mocked = vi.mocked(api);

afterEach(cleanup);

const emptySources: SourcesDocument = { sources: [] };
const oneSource: SourcesDocument = {
  sources: [{ id: 'a1a1a1a1a1a1a1a1', name: 'Movies', path: '/media/movies', titles: 2 }],
};

beforeEach(() => {
  mocked.loadSources.mockReset();
  mocked.addSource.mockReset();
  mocked.removeSource.mockReset();
  mocked.rescanSource.mockReset();
  mocked.loadSources.mockResolvedValue({ ok: true, value: emptySources });
});

test('with no libraries the pane says how to add one', async () => {
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByText(/No libraries yet/)).not.toBeNull();
  });
});

test('a library lists its path and title count', async () => {
  mocked.loadSources.mockResolvedValue({
    ok: true,
    value: {
      sources: [
        ...oneSource.sources,
        { id: 'b2b2b2b2b2b2b2b2', name: 'Shows', path: '/media/tvshows', titles: 1 },
      ],
    },
  });
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByText('Movies')).not.toBeNull();
  });
  expect(screen.getByText('/media/movies')).not.toBeNull();
  expect(screen.getByText('2 titles')).not.toBeNull();
  expect(screen.getByText('1 title')).not.toBeNull();
});

test('an unreachable server puts its reason on the pane', async () => {
  mocked.loadSources.mockResolvedValue({ ok: false, error: 'the server could not be reached' });
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual('the server could not be reached');
  });
});

test('adding a library posts the form and reloads', async () => {
  mocked.addSource.mockResolvedValue({ ok: true, value: true });
  mocked.loadSources.mockResolvedValueOnce({ ok: true, value: emptySources });
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Add library' })).not.toBeNull();
  });
  fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Movies' } });
  fireEvent.change(screen.getByLabelText('Path in this container'), { target: { value: '/media/movies' } });
  fireEvent.click(screen.getByRole('button', { name: 'Add library' }));
  await waitFor(() => {
    expect(screen.getByText('/media/movies')).not.toBeNull();
  });
  expect(mocked.addSource).toHaveBeenCalledWith('Movies', '/media/movies');
});

test('a refused add keeps the reason on the pane', async () => {
  mocked.addSource.mockResolvedValue({ ok: false, error: 'the path is not a directory this container can see' });
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Add library' })).not.toBeNull();
  });
  fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Movies' } });
  fireEvent.change(screen.getByLabelText('Path in this container'), { target: { value: '/nowhere' } });
  fireEvent.click(screen.getByRole('button', { name: 'Add library' }));
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual(
      'the path is not a directory this container can see',
    );
  });
});

test('removing and rescanning a library call their routes', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  mocked.removeSource.mockResolvedValue({ ok: true, value: true });
  mocked.rescanSource.mockResolvedValue({ ok: true, value: true });
  render(<SourcesPane />);
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

test('a refused remove and a refused rescan keep their reasons', async () => {
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  mocked.removeSource.mockResolvedValue({ ok: false, error: 'no such source' });
  mocked.rescanSource.mockResolvedValue({ ok: false, error: 'no such source' });
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByText('Movies')).not.toBeNull();
  });
  fireEvent.click(screen.getByRole('button', { name: 'Rescan' }));
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual('no such source');
  });
  fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual('no such source');
  });
});
