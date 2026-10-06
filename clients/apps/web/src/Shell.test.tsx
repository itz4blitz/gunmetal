import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { Shell } from './Shell.tsx';

afterEach(cleanup);

const props = {
  wordmark: 'Gunmetal',
  title: 'Nothing is playing',
  hint: 'The web client is running. Library and playback are not wired yet.',
  clock: '0:03',
};

test('the shell shows the wordmark, empty player copy, brass control and uptime', () => {
  render(<Shell {...props} />);
  expect(screen.getByText('Gunmetal').closest('#token-text')?.tagName).toStrictEqual('DIV');
  expect(screen.getByText('Nothing is playing').id).toStrictEqual('shell-title');
  expect(
    screen.getByText('The web client is running. Library and playback are not wired yet.').id,
  ).toStrictEqual('shell-hint');
  expect(screen.getByRole('status', { name: 'Uptime' }).textContent).toStrictEqual('0:03');
  expect(document.getElementById('shell-play')?.tagName).toStrictEqual('DIV');
});

test('a zero clock is shown', () => {
  render(<Shell {...props} clock="0:00" />);
  expect(screen.getByRole('status', { name: 'Uptime' }).textContent).toStrictEqual('0:00');
});
