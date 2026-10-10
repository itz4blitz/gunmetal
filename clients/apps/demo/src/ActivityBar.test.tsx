import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { ActivityBar } from './ActivityBar.tsx';

afterEach(cleanup);

test('a running job shows its label, count, and fill', async () => {
  const fetchImpl = (async () => ({
    ok: true,
    json: async () => ({ jobs: [{ id: 'artwork', label: 'Fetching album art', done: 1, total: 4 }] }),
  })) as unknown as typeof fetch;
  render(<ActivityBar fetchImpl={fetchImpl} />);
  expect(await screen.findByText('Fetching album art')).not.toBeNull();
  expect(screen.getByText('1 of 4')).not.toBeNull();
  expect(document.querySelector('[data-activity-fill="1"]')?.getAttribute('style')).toStrictEqual('width: 25%;');
});

test('a refused activity document stays hidden', async () => {
  const fetchImpl = (async () => {
    throw new Error('down');
  }) as unknown as typeof fetch;
  render(<ActivityBar fetchImpl={fetchImpl} />);
  await Promise.resolve();
  expect(document.querySelector('#activity-bar')).toBeNull();
});

test('a bad status or a bad document keeps the bar hidden', async () => {
  const notOk = (async () => ({ ok: false })) as unknown as typeof fetch;
  render(<ActivityBar fetchImpl={notOk} />);
  await Promise.resolve();
  expect(document.querySelector('#activity-bar')).toBeNull();
  cleanup();

  const wrongShape = (async () => ({ ok: true, json: async () => ({ jobs: 'nope' }) })) as unknown as typeof fetch;
  render(<ActivityBar fetchImpl={wrongShape} />);
  await Promise.resolve();
  await Promise.resolve();
  expect(document.querySelector('#activity-bar')).toBeNull();
});

test('advance is reported when the count changes, not when it repeats', async () => {
  vi.useFakeTimers();
  try {
    const advances: number[] = [];
    const fetchImpl = (async () => ({
      ok: true,
      json: async () => ({ jobs: [{ id: 'artwork', label: 'Fetching album art', done: 1, total: 4 }] }),
    })) as unknown as typeof fetch;
    render(
      <ActivityBar
        fetchImpl={fetchImpl}
        onAdvance={(done) => {
          advances.push(done);
        }}
      />,
    );
    await vi.advanceTimersByTimeAsync(0);
    expect(advances).toStrictEqual([1]);
    // The next poll reports the same count: no advance again.
    await vi.advanceTimersByTimeAsync(2000);
    expect(advances).toStrictEqual([1]);
  } finally {
    vi.useRealTimers();
  }
});

test('a poll that lands after unmount is ignored', async () => {
  let resolveFetch: ((value: unknown) => void) | undefined;
  const fetchImpl = (async () =>
    new Promise((resolve) => {
      resolveFetch = resolve;
    })) as unknown as typeof fetch;
  const view = render(<ActivityBar fetchImpl={fetchImpl} />);
  view.unmount();
  resolveFetch?.({ ok: true, json: async () => ({ jobs: [] }) });
  await Promise.resolve();
  await Promise.resolve();
  expect(document.querySelector('#activity-bar')).toBeNull();
});
