import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { Home } from './Home.tsx';

afterEach(cleanup);

test('spotlight uses the first recently-added fixture with art tone and actions', () => {
  const library = demoLibrary();
  const onOpenAlbum = vi.fn();
  const onPlayAlbum = vi.fn();
  const onSeeAll = vi.fn();
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={onOpenAlbum}
      onPlayAlbum={onPlayAlbum}
      onSeeAll={onSeeAll}
    />,
  );

  const home = document.querySelector('#destination-home');
  expect(home?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(document.querySelector('#home-spotlight')).toBeTruthy();
  expect(document.querySelector('#cover-spotlight-demo-album-01')).toBeTruthy();
  expect(
    document.querySelector('#home-spotlight [data-spotlight-eyebrow="1"]')?.textContent,
  ).toStrictEqual('Featured');
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).toBeTruthy();
  expect(
    document.querySelector('#home-spotlight [data-spotlight-artist="1"]')?.textContent,
  ).toStrictEqual('Mira Sol');
  expect(document.querySelector('#destination-headline')).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Home' })).toBeNull();
  expect(
    [...document.querySelectorAll('#destination-home > [id^="home-"]')].map((node) => node.id),
  ).toStrictEqual([
    'home-spotlight',
    'home-row-continue',
    'home-row-played',
    'home-row-loved',
    'home-row-recent',
  ]);

  fireEvent.click(document.querySelector('#home-spotlight-play')!);
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(document.querySelector('#home-spotlight-play')!, { key: 'Enter' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(document.querySelector('#home-spotlight-play')!, { key: ' ' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(document.querySelector('#home-spotlight-play')!, { key: 'Tab' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);

  fireEvent.click(screen.getByRole('button', { name: 'Open' }));
  expect(onOpenAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Open' }), { key: 'Enter' });
  expect(onOpenAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Open' }), { key: ' ' });
  expect(onOpenAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Open' }), { key: 'Tab' });
  expect(onOpenAlbum).toHaveBeenCalledTimes(3);

  fireEvent.click(screen.getByRole('button', { name: 'See all' }));
  expect(onSeeAll).toHaveBeenCalledTimes(1);
  fireEvent.keyDown(screen.getByRole('button', { name: 'See all' }), { key: 'Enter' });
  expect(onSeeAll).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'See all' }), { key: ' ' });
  expect(onSeeAll).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'See all' }), { key: 'Tab' });
  expect(onSeeAll).toHaveBeenCalledTimes(3);
});

test('home without browsable albums omits spotlight and art tone', () => {
  const base = demoLibrary();
  const hostile = base.albums.find((album) => album.hostile)!;
  const library: ShellLibrary = {
    kind: 'demo-fixtures',
    albums: [hostile],
    artists: base.artists,
  };
  render(
    <Home
      messages={destinationMessages()}
      library={library}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onSeeAll={vi.fn()}
    />,
  );
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toBeNull();
  expect(document.querySelector('#home-spotlight')).toBeNull();
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  expect(screen.queryByRole('button', { name: 'See all' })).toBeNull();
  expect(screen.getByText('No albums added yet')).toBeTruthy();
});
