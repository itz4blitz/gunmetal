import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { App } from './App.tsx';

afterEach(cleanup);

test('the demo app mounts the shell with the demo data label', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  expect(screen.getByText('Gunmetal').id).toStrictEqual('shell-wordmark');
  expect(screen.getByText('Demo data').id).toStrictEqual('demo-label');
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
});
