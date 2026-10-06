import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { Shell } from './Shell.tsx';

afterEach(cleanup);

test('the shell is found by its role and name and shows the count in the token colour', () => {
  render(<Shell label="Gunmetal" count={3} />);
  const status = screen.getByRole('status', { name: 'Gunmetal' });
  expect(status.textContent).toStrictEqual('3');
  expect(screen.getByText('Gunmetal').closest('#token-text')?.tagName).toStrictEqual('DIV');
});

test('a zero count is shown', () => {
  render(<Shell label="Gunmetal" count={0} />);
  expect(screen.getByRole('status', { name: 'Gunmetal' }).textContent).toStrictEqual('0');
});
