import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { ActivityBar } from './ActivityBar.tsx';

afterEach(cleanup);

test('a running job shows its label, count, and fill', async () => {
  const fetchImpl = (async () =>
    ({
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
