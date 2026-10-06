import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { App } from './App.tsx';

afterEach(cleanup);

test('the demo app mounts the shell with fixture home rows and demo data', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  expect(screen.getByText('Gunmetal').id).toStrictEqual('shell-wordmark');
  expect(screen.getByText('Demo data').id).toStrictEqual('demo-label');
  expect(document.querySelector('#destination-headline')).toBeNull();
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toStrictEqual(
    '01',
  );
  expect(document.querySelector('#home-spotlight')).toBeTruthy();
  expect(
    document.querySelector('#home-spotlight [data-spotlight-eyebrow="1"]')?.textContent,
  ).toStrictEqual('Featured');
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Recently added' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Recently played' })).toBeTruthy();
  expect(screen.getByText('Nothing played yet')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Harbour Lights' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'See all' })).toBeTruthy();
});

test('playing a fixture album fills the player bar from demo-local state', () => {
  window.history.pushState(null, '', '/');
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1600 });
  render(<App />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual(
    'destination-headline',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
  expect(document.querySelector('#player-artist')?.textContent).toStrictEqual('Mira Sol');
  expect(document.querySelector('#shell-play')?.getAttribute('aria-label')).toStrictEqual('Pause');
  expect(document.querySelector('#shell-play')?.getAttribute('data-playing')).toStrictEqual('1');
  expect(document.querySelector('#queue-line-demo-track-01-01')).toBeTruthy();
  expect(document.querySelector('#player-full')).toBeNull();
  expect(document.querySelector('#player-album')?.textContent).toStrictEqual('Harbour Lights');
  expect(document.querySelector('#nav-sidebar')).toBeTruthy();
  expect(document.querySelector('#player-bar')).toBeTruthy();
});
