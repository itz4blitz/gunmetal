import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { Counter } from './Counter.tsx';

afterEach(cleanup);

test('the counter is found by its role and name and shows the count', () => {
  render(<Counter label="Unplayed tracks" count={3} />);
  const counter = screen.getByRole('status', { name: 'Unplayed tracks' });
  expect(counter.textContent).toStrictEqual('3');
  expect(counter.getAttribute('data-empty')).toStrictEqual('false');
});

test('a count of zero is shown and marked empty', () => {
  render(<Counter label="Unplayed tracks" count={0} />);
  const counter = screen.getByRole('status', { name: 'Unplayed tracks' });
  expect(counter.textContent).toStrictEqual('0');
  expect(counter.getAttribute('data-empty')).toStrictEqual('true');
});
