import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { App } from './App.tsx';

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

test('the app starts at the given uptime and advances it at run time', async () => {
  vi.useFakeTimers();
  render(<App start={2} />);
  expect(screen.getByRole('status', { name: 'Uptime' }).textContent).toStrictEqual('0:02');
  await act(async () => {
    await vi.advanceTimersByTimeAsync(1000);
  });
  expect(screen.getByRole('status', { name: 'Uptime' }).textContent).toStrictEqual('0:03');
  await act(async () => {
    await vi.advanceTimersByTimeAsync(999);
  });
  expect(screen.getByRole('status', { name: 'Uptime' }).textContent).toStrictEqual('0:03');
});

test('the app starts at zero when no start is given', () => {
  vi.useFakeTimers();
  render(<App />);
  expect(screen.getByRole('status', { name: 'Uptime' }).textContent).toStrictEqual('0:00');
  expect(screen.getByText('Nothing is playing').id).toStrictEqual('shell-title');
});
