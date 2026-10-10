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
  sources: [{ id: 'a1a1a1a1a1a1a1a1', name: 'Movies', path: '/media/movies', kind: 'movies', items: 2 }],
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
        { id: 'b2b2b2b2b2b2b2b2', name: 'Shows', path: '/media/tvshows', kind: 'shows', items: 1 },
        { id: 'c3c3c3c3c3c3c3c3', name: 'Songs', path: '/media/music', kind: 'music', items: 4 },
      ],
    },
  });
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByText('/media/movies')).not.toBeNull();
  });
  expect(document.querySelector('[data-source-kind="movies"]')?.textContent).toStrictEqual('Movies');
  expect(document.querySelector('[data-source-kind="shows"]')?.textContent).toStrictEqual('TV shows');
  expect(document.querySelector('[data-source-kind="music"]')?.textContent).toStrictEqual('Music');
  expect(screen.getByText('2 titles')).not.toBeNull();
  expect(screen.getByText('1 title')).not.toBeNull();
  // Music is counted in tracks.
  expect(screen.getByText('4 tracks')).not.toBeNull();
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
  fireEvent.change(screen.getByLabelText('Type'), { target: { value: 'nope' } });
  fireEvent.change(screen.getByLabelText('Type'), { target: { value: 'movies' } });
  fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Movies' } });
  fireEvent.change(screen.getByLabelText('Folder on the server'), { target: { value: '/media/movies' } });
  fireEvent.click(screen.getByRole('button', { name: 'Add library' }));
  await waitFor(() => {
    expect(screen.getByText('/media/movies')).not.toBeNull();
  });
  expect(mocked.addSource).toHaveBeenCalledWith('Movies', '/media/movies', 'movies');
});

test('a refused add keeps the reason on the pane', async () => {
  mocked.addSource.mockResolvedValue({ ok: false, error: 'the path is not a directory this server can see' });
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Add library' })).not.toBeNull();
  });
  fireEvent.change(screen.getByLabelText('Type'), { target: { value: 'shows' } });
  fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Movies' } });
  fireEvent.change(screen.getByLabelText('Folder on the server'), { target: { value: '/nowhere' } });
  fireEvent.click(screen.getByRole('button', { name: 'Add library' }));
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual('the path is not a directory this server can see');
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

test('one track is a track, and the type list offers music, movies and TV shows', async () => {
  mocked.loadSources.mockResolvedValue({
    ok: true,
    value: { sources: [{ id: 'c3c3c3c3c3c3c3c3', name: 'Songs', path: '/media/music', kind: 'music', items: 1 }] },
  });
  render(<SourcesPane />);
  await waitFor(() => {
    expect(screen.getByText('1 track')).not.toBeNull();
  });
  const type = screen.getByLabelText('Type');
  expect([...type.querySelectorAll('option')].map((option) => [option.value, option.textContent])).toStrictEqual([
    ['music', 'Music'],
    ['movies', 'Movies'],
    ['shows', 'TV shows'],
  ]);
});

test('each change the server accepted is reported to the host, and a refused one is not', async () => {
  const changes: string[] = [];
  mocked.loadSources.mockResolvedValue({ ok: true, value: oneSource });
  mocked.addSource.mockResolvedValueOnce({ ok: false, error: 'that path is already a library' });
  mocked.addSource.mockResolvedValue({ ok: true, value: true });
  mocked.rescanSource.mockResolvedValueOnce({ ok: false, error: 'no such source' });
  mocked.rescanSource.mockResolvedValue({ ok: true, value: true });
  mocked.removeSource.mockResolvedValueOnce({ ok: false, error: 'no such source' });
  mocked.removeSource.mockResolvedValue({ ok: true, value: true });
  render(
    <SourcesPane
      onChange={() => {
        changes.push('changed');
      }}
    />,
  );
  await waitFor(() => {
    expect(screen.getByText('/media/movies')).not.toBeNull();
  });
  const add = async () => {
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Songs' } });
    fireEvent.change(screen.getByLabelText('Folder on the server'), { target: { value: '/media/music' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add library' }));
  };
  await add();
  await waitFor(() => {
    expect(screen.getByRole('alert').textContent).toStrictEqual('that path is already a library');
  });
  fireEvent.click(screen.getByRole('button', { name: 'Rescan' }));
  await waitFor(() => {
    expect(mocked.rescanSource).toHaveBeenCalledTimes(1);
  });
  fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
  await waitFor(() => {
    expect(mocked.removeSource).toHaveBeenCalledTimes(1);
  });
  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Remove' }).hasAttribute('disabled')).toStrictEqual(false);
  });
  expect(changes).toStrictEqual([]);

  await add();
  await waitFor(() => {
    expect(changes).toStrictEqual(['changed']);
  });
  fireEvent.click(screen.getByRole('button', { name: 'Rescan' }));
  await waitFor(() => {
    expect(changes).toStrictEqual(['changed', 'changed']);
  });
  fireEvent.click(screen.getByRole('button', { name: 'Remove' }));
  await waitFor(() => {
    expect(changes).toStrictEqual(['changed', 'changed', 'changed']);
  });
});
