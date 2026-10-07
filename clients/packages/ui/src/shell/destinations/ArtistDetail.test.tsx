import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { act } from 'react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellLibrary } from '../library-types.ts';
import { ArtistDetail } from './ArtistDetail.tsx';

/** A lookup that must land: the test names what it could not find. */
function required<T extends Element>(node: T | null | undefined, what: string): T {
  if (node === null || node === undefined) {
    throw new Error(`${what} missing`);
  }
  return node;
}

afterEach(cleanup);

/** A scriptable stand-in for the browser's IntersectionObserver. */
class StubObserver {
  static instances: StubObserver[] = [];
  readonly observed: Element[] = [];
  readonly callback: (entries: Array<{ isIntersecting: boolean }>) => void;
  constructor(callback: (entries: Array<{ isIntersecting: boolean }>) => void) {
    this.callback = callback;
    StubObserver.instances.push(this);
  }
  observe(target: Element): void {
    this.observed.push(target);
  }
  unobserve(): void {}
  disconnect(): void {}
  seeAll(isIntersecting: boolean): void {
    this.callback([{ isIntersecting }]);
  }
}

const asIO = StubObserver as unknown as typeof IntersectionObserver;

function fakeScroller(top: number, height: number): { element: HTMLElement; scrollTo: (value: number) => void } {
  const element = document.createElement('div');
  document.body.appendChild(element);
  let value = top;
  Object.defineProperty(element, 'scrollTop', { get: () => value, configurable: true });
  Object.defineProperty(element, 'clientHeight', { value: height, configurable: true });
  return {
    element,
    scrollTo: (next) => {
      value = next;
    },
  };
}

test('the artist album grid defers cover URLs to an injected IntersectionObserver', () => {
  const library = demoLibrary();
  const artist = library.artists.find((row) => row.key === 'mira-sol');
  if (artist === undefined) {
    throw new Error('fixture artist mira-sol missing');
  }
  StubObserver.instances = [];
  const view = render(
    <ArtistDetail
      artist={artist}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
      nearViewObserver={asIO}
    />,
  );
  expect(StubObserver.instances.length).toStrictEqual(artist.albumIds.length);
  expect(
    [...document.querySelectorAll('[data-cover-art="1"]')].filter((node) =>
      (node.getAttribute('style') ?? '').includes('background-image'),
    ),
  ).toStrictEqual([]);
  act(() => {
    for (const instance of StubObserver.instances) {
      instance.seeAll(true);
    }
  });
  expect(
    [...document.querySelectorAll('[data-cover-art="1"]')].filter((node) =>
      (node.getAttribute('style') ?? '').includes('background-image'),
    ).length,
  ).toStrictEqual(artist.albumIds.length);
  view.unmount();
});

test('the all-songs list windows its rows against the content pane', () => {
  const library = demoLibrary();
  const artist = library.artists.find((row) => row.key === 'mira-sol');
  if (artist === undefined) {
    throw new Error('fixture artist mira-sol missing');
  }
  const all = library.albums
    .filter((album) => artist.albumIds.includes(album.id) && !album.hostile)
    .flatMap((album) => album.tracks.map((track) => track.id));
  if (all.length < 7) {
    throw new Error('fixture mira-sol tracks missing');
  }
  const scroller = fakeScroller(144, 48);
  render(
    <ArtistDetail
      artist={artist}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
      getScroller={() => scroller.element}
    />,
  );
  const shown = [...document.querySelectorAll('[data-artist-song-list] [data-track-row]')].map((row) => row.id);
  // 144px down at 52px rows = row 2 first; the overscan reaches past both
  // ends of a 7-song list, so every row still renders — clamped, honest.
  expect(shown).toStrictEqual(all.map((id) => `track-row-${id}`));
  // Scrolling deep enough that the overscan clamps at the list's top moves
  // the window to the last rows.
  scroller.scrollTo(572);
  act(() => {
    scroller.element.dispatchEvent(new Event('scroll'));
  });
  const after = [...document.querySelectorAll('[data-artist-song-list] [data-track-row]')].map((row) => row.id);
  expect(after).toStrictEqual(all.slice(3).map((id) => `track-row-${id}`));
});

test('artist detail shows hero image name album grid all songs and plays the first album', () => {
  const library = demoLibrary();
  const artist = library.artists.find((row) => row.key === 'mira-sol');
  const onPlayAlbum = vi.fn();
  const onOpenAlbum = vi.fn();
  const onPlayTrack = vi.fn();
  const { container } = render(
    <ArtistDetail
      artist={artist}
      library={library}
      messages={destinationMessages()}
      currentTrackId="demo-track-02-02"
      onBack={vi.fn()}
      onOpenAlbum={onOpenAlbum}
      onPlayAlbum={onPlayAlbum}
      onPlayTrack={onPlayTrack}
    />,
  );
  const root = container.querySelector('#destination-artist');
  expect(root?.getAttribute('data-artist-key')).toStrictEqual('mira-sol');
  expect(root?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(container.querySelector('[data-artist-hero="1"]')).not.toBeNull();
  // The generated artist image fills the nut-shaped hero and blooms behind
  // the header as aria-hidden ambient colour.
  expect(container.querySelector('[data-artist-avatar-nut="1"]')).not.toBeNull();
  const bloom = container.querySelector('[data-artist-bloom="1"]') as HTMLElement;
  expect(bloom.getAttribute('aria-hidden')).toStrictEqual('true');
  expect(bloom.style.backgroundImage).toContain('/media/artists/mira-sol.svg');
  expect(bloom.textContent).toStrictEqual('');
  expect(container.querySelector('[data-artist-scrim="1"]')?.getAttribute('aria-hidden')).toStrictEqual('true');
  const hero = container.querySelector('[data-artist-avatar="hero"]') as HTMLElement;
  expect(hero.getAttribute('data-artist-image')).toStrictEqual('1');
  expect(hero.style.backgroundImage).toContain('/media/artists/mira-sol.svg');
  expect(hero.textContent).toStrictEqual('M');
  expect(screen.getByRole('heading', { name: 'Mira Sol' }).id).toStrictEqual('destination-headline');
  expect(container.querySelector('#artist-disambiguation')).toBeNull();
  expect(container.querySelector('[data-artist-album-count="1"]')?.textContent).toStrictEqual('2 albums');
  // The hero reads eyebrow · name · "albums · songs", and the grid has its own heading.
  expect(container.querySelector('[data-artist-header-text="1"] [data-detail-eyebrow="1"]')?.textContent).toStrictEqual(
    'Artist',
  );
  expect([...container.querySelectorAll('[data-artist-meta="1"] > *')].map((node) => node.textContent)).toStrictEqual([
    '2 albums',
    '·',
    '7 songs',
  ]);
  expect(screen.getByRole('heading', { name: 'Albums' }).getAttribute('data-section-heading')).toStrictEqual('1');
  expect(container.querySelector('#artist-back svg')?.getAttribute('data-icon')).toStrictEqual('back');
  expect(container.querySelector('#artist-play[data-brass-hex="1"]')).not.toBeNull();
  expect(container.querySelector('#artist-album-grid')).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Harbour Lights' })).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Night Shift' })).not.toBeNull();
  fireEvent.click(required(document.querySelector('#artist-play'), '#artist-play'));
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(required(document.querySelector('#artist-play'), '#artist-play'), { key: 'Enter' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(required(document.querySelector('#artist-play'), '#artist-play'), { key: ' ' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(required(document.querySelector('#artist-play'), '#artist-play'), { key: 'Tab' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(3);
  fireEvent.click(screen.getByRole('button', { name: 'Night Shift' }));
  expect(onOpenAlbum).toHaveBeenCalledWith('demo-album-02');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });

  // All songs reads across the artist's own albums (4 + 3 tracks) and plays a row.
  const songs = container.querySelector('#artist-all-songs');
  expect(songs).not.toBeNull();
  expect(screen.getByRole('heading', { name: 'All songs' })).not.toBeNull();
  // The table head is presentational chrome over the row grid: number,
  // title, album, time — hidden from assistive tech, aligned with the columns.
  const tableHead = container.querySelector('[data-artist-table-head="1"]');
  expect(tableHead?.getAttribute('aria-hidden')).toStrictEqual('true');
  expect([...(tableHead?.querySelectorAll('*') ?? [])].map((node) => node.textContent)).toStrictEqual([
    '#',
    'Title',
    'Album',
    'Time',
  ]);
  expect(container.querySelectorAll('[data-artist-song-list="1"] [data-track-row]')).toHaveLength(7);
  expect(
    [...container.querySelectorAll('[data-artist-song-list="1"] [data-track-album]')].map((node) => node.textContent),
  ).toStrictEqual([
    'Harbour Lights',
    'Harbour Lights',
    'Harbour Lights',
    'Harbour Lights',
    'Night Shift',
    'Night Shift',
    'Night Shift',
  ]);
  const currentRow = container.querySelector('#track-row-demo-track-02-02');
  expect(currentRow?.getAttribute('data-current')).toStrictEqual('1');
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
});

test('same-name artists carry the catalogue-key disambiguation line; unique names do not', () => {
  const library = demoLibrary();
  const alex = library.artists.find((row) => row.key === 'alex-reed-north');
  const { unmount } = render(
    <ArtistDetail
      artist={alex}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Alex Reed' })).not.toBeNull();
  expect(document.querySelector('#artist-disambiguation')?.textContent).toStrictEqual('alex-reed-north');
  unmount();

  const mira = library.artists.find((row) => row.key === 'mira-sol');
  render(
    <ArtistDetail
      artist={mira}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
    />,
  );
  expect(document.querySelector('#artist-disambiguation')).toBeNull();
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
  // No artist image means no ambient bloom — the nut and initial stand alone.
  expect(empty.container.querySelector('[data-artist-bloom="1"]')).toBeNull();
  expect(empty.container.querySelector('[data-artist-scrim="1"]')).toBeNull();
  const hero = empty.container.querySelector('[data-artist-avatar="hero"]') as HTMLElement;
  expect(hero.getAttribute('data-artist-image')).toStrictEqual('0');
  expect(hero.textContent).toStrictEqual('?');
  expect(document.querySelector('#destination-headline')?.textContent).toStrictEqual('   ');
  fireEvent.click(required(document.querySelector('#artist-play'), '#artist-play'));
  fireEvent.keyDown(required(document.querySelector('#artist-play'), '#artist-play'), { key: 'Enter' });
  fireEvent.keyDown(required(document.querySelector('#artist-play'), '#artist-play'), { key: ' ' });
  expect(onPlayAlbum).not.toHaveBeenCalled();
  expect(document.querySelector('#artist-all-songs')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(onBack).toHaveBeenCalledTimes(2);
});

test('hostile artist chrome uses the catalogue label and its albums never list songs', () => {
  const library = demoLibrary();
  const artist = library.artists.find((row) => row.key === 'hostile-artist');
  const onPlayTrack = vi.fn();
  const { container } = render(
    <ArtistDetail
      artist={artist}
      library={library}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={onPlayTrack}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Security corpus' }).id).toStrictEqual('destination-headline');
  const hero = container.querySelector('[data-artist-avatar="hero"]') as HTMLElement;
  expect(hero.getAttribute('data-artist-image')).toStrictEqual('0');
  expect(hero.textContent).toStrictEqual('S');
  expect(container.querySelector('#artist-all-songs')).toBeNull();
  expect(container.textContent).not.toContain('onerror');
  expect(container.textContent).not.toContain('__gm_xss');
});
