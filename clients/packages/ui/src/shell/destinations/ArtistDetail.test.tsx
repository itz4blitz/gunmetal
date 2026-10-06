import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { ArtistDetail } from './ArtistDetail.tsx';

afterEach(cleanup);

test('artist detail shows hex nut avatar name album grid and plays the first album', () => {
  const library = demoLibrary();
  const artist = library.artists.find((row) => row.key === 'mira-sol');
  const onPlayAlbum = vi.fn();
  const onOpenAlbum = vi.fn();
  const { container } = render(
    <ArtistDetail
      artist={artist}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={onOpenAlbum}
      onPlayAlbum={onPlayAlbum}
    />,
  );
  const root = container.querySelector('#destination-artist');
  expect(root?.getAttribute('data-artist-key')).toStrictEqual('mira-sol');
  expect(root?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(container.querySelector('[data-artist-hero="1"]')).toBeTruthy();
  expect(container.querySelector('[data-artist-avatar-nut="1"]')).toBeTruthy();
  expect(container.querySelector('[data-artist-avatar="hero"]')?.textContent).toStrictEqual('M');
  expect(screen.getByRole('heading', { name: 'Mira Sol' }).id).toStrictEqual('destination-headline');
  expect(container.querySelector('[data-artist-album-count="1"]')?.textContent).toStrictEqual('2 albums');
  expect(container.querySelector('#artist-play[data-brass-hex="1"]')).toBeTruthy();
  expect(container.querySelector('#artist-album-grid')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Harbour Lights' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Night Shift' })).toBeTruthy();
  fireEvent.click(document.querySelector('#artist-play')!);
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: 'Enter' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: ' ' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: 'Tab' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.click(screen.getByRole('button', { name: 'Night Shift' }));
  expect(onOpenAlbum).toHaveBeenCalledWith('demo-album-02');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
});

test('missing artist and an artist with no albums stay on chrome without a wash', () => {
  const library = demoLibrary();
  const onBack = vi.fn();
  const onPlayAlbum = vi.fn();
  const missing = render(
    <ArtistDetail
      artist={undefined}
      library={library}
      messages={destinationMessages()}
      onBack={onBack}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={onPlayAlbum}
    />,
  );
  expect(screen.getByRole('heading', { name: 'That artist is not in the demo library' }).id).toStrictEqual(
    'destination-headline',
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: ' ' });
  expect(onBack).toHaveBeenCalledTimes(1);
  missing.unmount();

  const emptyLibrary: ShellLibrary = {
    kind: 'demo-fixtures',
    albums: library.albums,
    artists: [{ key: 'lonely', name: '   ', albumIds: ['missing-album'] }],
  };
  const empty = render(
    <ArtistDetail
      artist={emptyLibrary.artists[0]}
      library={emptyLibrary}
      messages={destinationMessages()}
      onBack={onBack}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={onPlayAlbum}
    />,
  );
  expect(empty.container.querySelector('#destination-artist')?.getAttribute('data-art-tone')).toBeNull();
  expect(empty.container.querySelector('[data-artist-avatar="hero"]')?.textContent).toStrictEqual('?');
  expect(document.querySelector('#destination-headline')?.textContent).toStrictEqual('   ');
  fireEvent.click(document.querySelector('#artist-play')!);
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: 'Enter' });
  fireEvent.keyDown(document.querySelector('#artist-play')!, { key: ' ' });
  expect(onPlayAlbum).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(onBack).toHaveBeenCalledTimes(2);
});

test('hostile artist chrome uses the catalogue label', () => {
  const library = demoLibrary();
  const artist = library.artists.find((row) => row.key === 'hostile-artist');
  render(
    <ArtistDetail
      artist={artist}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Security corpus' }).id).toStrictEqual('destination-headline');
  expect(document.querySelector('[data-artist-avatar="hero"]')?.textContent).toStrictEqual('S');
});
