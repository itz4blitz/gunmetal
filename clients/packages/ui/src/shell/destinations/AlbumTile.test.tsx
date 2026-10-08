import { act } from 'react';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum } from '../library-types.ts';
import { AlbumTile } from './AlbumTile.tsx';

/** A lookup that must land: the test names what it could not find. */
function required<T extends Element>(node: T | null | undefined, what: string): T {
  if (node === null || node === undefined) {
    throw new Error(`${what} missing`);
  }
  return node;
}

afterEach(cleanup);

const messages = {
  hostileAlbumLabel: 'Hostile metadata (fixture)',
  hostileArtistLabel: 'Hostile artist (fixture)',
  playAlbum: 'Play album',
  play: 'Play',
  playNext: 'Play next',
  addToQueue: 'Add to queue',
  goToAlbum: 'Go to album',
  goToArtist: 'Go to artist',
  moreActions: 'More',
  contextMenu: 'Actions',
  trackCountOne: 'track',
  trackCountLabel: 'tracks',
} as DestinationMessages;

const album: ShellAlbum = {
  id: 'demo-album-1',
  title: 'Harbour Lights',
  artistName: 'Keratin',
  artistKey: 'keratin',
  year: 2024,
  coverTone: '02',
  coverUrl: '/media/covers/fixture.svg',
  hostile: false,
  discs: [{ index: 1, title: '' }],
  tracks: [
    {
      id: 't1',
      albumId: 'demo-album-1',
      discIndex: 1,
      number: 1,
      title: 'Pier at Dusk',
      artistName: 'Keratin',
      durationMs: 180_000,
      flag: 'ok',
      lyricsKind: 'none',
      mediaUrl: '/media/audio/fixtures.wav',
    },
  ],
};

test('album tile shows title artist hierarchy and opens on activate', () => {
  const onOpen = vi.fn();
  render(<AlbumTile album={album} messages={messages} onOpen={onOpen} />);
  expect(screen.getByText('Harbour Lights')).not.toBeNull();
  expect(screen.getByText('Keratin')).not.toBeNull();
  expect(document.querySelector('[data-album-title="1"]')?.textContent).toStrictEqual('Harbour Lights');
  expect(document.querySelector('[data-album-artist="1"]')?.textContent).toStrictEqual('Keratin');
  fireEvent.click(required(document.querySelector('[data-album-art="1"]'), '[data-album-art="1"]'));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(onOpen).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: 'Enter' });
  expect(onOpen).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: ' ' });
  expect(onOpen).toHaveBeenCalledTimes(4);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: 'Tab' });
  expect(onOpen).toHaveBeenCalledTimes(4);
});

test('play control is a sibling of open — never a nested button', () => {
  const { container } = render(<AlbumTile album={album} messages={messages} onOpen={vi.fn()} onPlay={vi.fn()} />);
  expect(container.querySelectorAll('button button')).toHaveLength(0);
  const open = screen.getByRole('button', { name: 'Harbour Lights' });
  const play = screen.getByRole('button', { name: 'Play album' });
  expect(open.contains(play)).toStrictEqual(false);
  expect(play.contains(open)).toStrictEqual(false);
  expect(play.closest('[data-album-art="1"]')).not.toBeNull();
  expect(open.getAttribute('data-album-open')).toStrictEqual('1');
});

test('2026 art actions: a scrim carries the controls, kebab rides the art', () => {
  const onOpen = vi.fn();
  render(<AlbumTile album={album} messages={messages} onOpen={onOpen} onOpenArtist={vi.fn()} />);
  const art = document.querySelector('[data-album-art="1"]');
  // The scrim is the fade the controls sit on — decorative, never a control.
  const scrim = art?.querySelector(':scope > [data-art-scrim="1"]');
  expect(scrim).not.toBeNull();
  expect(scrim?.getAttribute('role')).toStrictEqual(null);
  // The kebab is anchored to the art (it sits on the scrim), not the tile text.
  expect(screen.getByRole('button', { name: 'More' }).closest('[data-album-art="1"]')).toStrictEqual(art);
  // The kebab is the "more" icon alone; its name is the aria-label.
  expect(screen.getByRole('button', { name: 'More' }).querySelector('svg')?.getAttribute('data-icon')).toStrictEqual(
    'more',
  );
  expect(screen.getByRole('button', { name: 'More' }).textContent).toStrictEqual('');
  // It still opens the catalogue menu without opening the album underneath.
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  expect(screen.getByRole('menuitem', { name: 'Go to artist' })).not.toBeNull();
  expect(onOpen).not.toHaveBeenCalled();
  // Play stays a sibling of open, above the scrim.
  const play = screen.getByRole('button', { name: 'Play album' });
  expect(art?.contains(play)).toStrictEqual(true);
  expect(scrim?.contains(play)).toStrictEqual(false);
});

test('play affordance calls onPlay when provided otherwise onOpen', () => {
  const onOpen = vi.fn();
  const onPlay = vi.fn();
  const withPlay = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} onPlay={onPlay} />);
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(onPlay).toHaveBeenCalledWith('demo-album-1');
  expect(onOpen).not.toHaveBeenCalled();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Enter' });
  expect(onPlay).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: ' ' });
  expect(onPlay).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Tab' });
  expect(onPlay).toHaveBeenCalledTimes(3);
  withPlay.unmount();

  const openOnly = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} />);
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  openOnly.unmount();
});

test('playing prop marks the tile with the brass where-you-are state', () => {
  const onOpen = vi.fn();
  const playing = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} playing />);
  const tile = document.querySelector('#album-tile-demo-album-1');
  expect(tile?.getAttribute('data-tile-playing')).toStrictEqual('1');
  // The state is presentation only: open still works from title and art.
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  playing.unmount();

  const resting = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} />);
  expect(document.querySelector('#album-tile-demo-album-1')?.getAttribute('data-tile-playing')).toStrictEqual('0');
  resting.unmount();
});

test('hostile album uses catalogue labels on the tile', () => {
  const onOpen = vi.fn();
  render(
    <AlbumTile
      album={{ ...album, hostile: true, title: '<script>', artistName: 'x' }}
      messages={messages}
      onOpen={onOpen}
    />,
  );
  expect(screen.getByRole('button', { name: 'Hostile metadata (fixture)' })).not.toBeNull();
  expect(screen.getByText('Hostile artist (fixture)')).not.toBeNull();
  // Grid covers carry no letter — the hostile label lives in the tile text.
  expect(document.querySelector('[data-cover-label]')).toBeNull();
});

test('stagger slots and go to artist open from context and more', () => {
  const onOpenArtist = vi.fn();
  const first = render(<AlbumTile album={album} messages={messages} onOpen={vi.fn()} staggerIndex={3} />);
  expect(document.querySelector('[data-tile-stagger="3"]')).not.toBeNull();
  expect(first.container.querySelector('[data-item-more="1"]')).toBeNull();
  fireEvent.contextMenu(required(document.querySelector('#album-tile-demo-album-1'), '#album-tile-demo-album-1'));
  expect(screen.queryByRole('menuitem', { name: 'Go to artist' })).toBeNull();
  first.unmount();

  const capped = render(
    <AlbumTile album={album} messages={messages} onOpen={vi.fn()} onOpenArtist={onOpenArtist} staggerIndex={12} />,
  );
  expect(document.querySelector('[data-tile-stagger="6"]')).not.toBeNull();
  fireEvent.contextMenu(required(document.querySelector('#album-tile-demo-album-1'), '#album-tile-demo-album-1'));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(onOpenArtist).toHaveBeenCalledWith('keratin');
  fireEvent.keyDown(screen.getByRole('button', { name: 'More' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: 'Escape' });
  // The kebab is a toggle wired to its menu: Space opens, a key that is not
  // an activation key changes nothing, a second press closes.
  const kebab = screen.getByRole('button', { name: 'More' });
  expect(kebab.getAttribute('aria-haspopup')).toStrictEqual('menu');
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  fireEvent.keyDown(kebab, { key: ' ' });
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  expect(screen.getByRole('menu', { name: 'Actions' }).getAttribute('data-menu-id')).toStrictEqual(
    kebab.getAttribute('aria-controls'),
  );
  fireEvent.keyDown(kebab, { key: 'Tab' });
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.click(kebab);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  expect(screen.queryByRole('menu')).toStrictEqual(null);
  // Open again; a press on the kebab itself is left to the kebab, a press
  // anywhere else closes the menu.
  fireEvent.click(kebab);
  expect(screen.getAllByRole('menuitem').map((item) => item.getAttribute('aria-label'))).toStrictEqual([
    'Play',
    'Play next',
    'Add to queue',
    'Go to album',
    'Go to artist',
  ]);
  fireEvent.pointerDown(kebab);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.pointerDown(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.pointerDown(document.body);
  expect(kebab.getAttribute('aria-expanded')).toStrictEqual('false');
  expect(screen.queryByRole('menu')).toStrictEqual(null);
  capped.unmount();
});

test('catalogue menu plays queues and opens the album from the tile', () => {
  const onOpen = vi.fn();
  const onPlay = vi.fn();
  const onPlayNext = vi.fn();
  const onAddToQueue = vi.fn();
  const armed = render(
    <AlbumTile
      album={album}
      messages={messages}
      onOpen={onOpen}
      onPlay={onPlay}
      onPlayNext={onPlayNext}
      onAddToQueue={onAddToQueue}
      onOpenArtist={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onPlay).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNext).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddToQueue).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  armed.unmount();

  const quiet = render(<AlbumTile album={album} messages={messages} onOpen={onOpen} onOpenArtist={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onPlay).toHaveBeenCalledTimes(1);
  expect(onPlayNext).toHaveBeenCalledTimes(1);
  expect(onAddToQueue).toHaveBeenCalledTimes(1);
  quiet.unmount();
});

test('album context menu plays, queues without shuffle, and opens the album', () => {
  const onOpen = vi.fn();
  const onPlay = vi.fn();
  const onPlayNext = vi.fn();
  const onAddToQueue = vi.fn();
  render(
    <AlbumTile
      album={album}
      messages={messages}
      onOpen={onOpen}
      onPlay={onPlay}
      onPlayNext={onPlayNext}
      onAddToQueue={onAddToQueue}
      onOpenArtist={vi.fn()}
    />,
  );
  fireEvent.contextMenu(required(document.querySelector('#album-tile-demo-album-1'), '#album-tile-demo-album-1'));
  const labels = [...document.querySelectorAll('[data-menu-label="1"]')].map((node) => node.textContent);
  expect(labels).toStrictEqual(['Play', 'Play next', 'Add to queue', 'Go to album', 'Go to artist']);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  expect(onPlay).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNext).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddToQueue).toHaveBeenCalledWith('demo-album-1');
  fireEvent.click(screen.getByRole('button', { name: 'More' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onOpen).toHaveBeenCalledWith('demo-album-1');
});

test('nearArt defers the cover URL until the tile reports itself near', async () => {
  let report: (() => void) | undefined = undefined;
  let disconnected = false;
  const nearArt = (target: Element, onNear: () => void) => {
    expect(target.id).toStrictEqual('cover-grid-demo-album-1');
    report = onNear;
    return () => {
      disconnected = true;
    };
  };
  const { unmount } = render(<AlbumTile album={album} messages={messages} onOpen={vi.fn()} nearArt={nearArt} />);
  const cover = document.querySelector('#cover-grid-demo-album-1');
  expect(cover?.getAttribute('style') ?? '').not.toContain('background-image');
  expect(cover?.getAttribute('data-cover-art')).toStrictEqual('1');
  act(() => {
    report?.();
  });
  expect(document.querySelector('#cover-grid-demo-album-1')?.getAttribute('style')).toContain('background-image');
  unmount();
  expect(disconnected).toStrictEqual(true);
});

test('a folder cover URL is painted on the tile, and an empty cover is not', () => {
  const folder = '/media/library/covers/aaaaaaaaaaaaaaaa.jpg';
  const painted = render(<AlbumTile album={{ ...album, coverUrl: folder }} messages={messages} onOpen={vi.fn()} />);
  const cover = required(painted.container.querySelector('#cover-grid-demo-album-1'), 'folder cover') as HTMLElement;
  expect(cover.getAttribute('data-cover-art')).toStrictEqual('1');
  expect(cover.style.backgroundImage).toContain(folder);
  painted.unmount();

  const bare = render(<AlbumTile album={{ ...album, coverUrl: '' }} messages={messages} onOpen={vi.fn()} />);
  const plate = required(bare.container.querySelector('#cover-grid-demo-album-1'), 'empty cover') as HTMLElement;
  expect(plate.getAttribute('data-cover-art')).toStrictEqual('0');
  expect(plate.style.backgroundImage).toStrictEqual('');
});

test('the tile states year, track count and total duration as text', () => {
  // 180_000 + 65_000 = 245_000 ms = 4:05. The year is a catalogue fact, not a label.
  const two = {
    ...album,
    year: 2024,
    tracks: [
      album.tracks[0],
      {
        ...album.tracks[0],
        id: 't2',
        number: 2,
        title: 'Second',
        durationMs: 65_000,
      },
    ],
  };
  if (two.tracks[0] === undefined) {
    throw new Error('fixture track missing');
  }
  const view = render(<AlbumTile album={two} messages={messages} onOpen={vi.fn()} />);
  expect(view.container.querySelector('[data-album-facts="1"]')?.textContent).toStrictEqual('2024 · 2 tracks · 4:05');
  view.unmount();

  const one = render(<AlbumTile album={album} messages={messages} onOpen={vi.fn()} />);
  expect(one.container.querySelector('[data-album-facts="1"]')?.textContent).toStrictEqual('2024 · 1 track · 3:00');
});

test('a year of 0 is omitted — the tile never prints 0 for an unknown year', () => {
  const undated = render(<AlbumTile album={{ ...album, year: 0 }} messages={messages} onOpen={vi.fn()} />);
  expect(undated.container.querySelector('[data-album-facts="1"]')?.textContent).toStrictEqual('1 track · 3:00');
  expect(undated.container.querySelector('[data-album-year]')).toBeNull();
  expect(screen.queryByText('0')).toBeNull();
  undated.unmount();

  const negative = render(<AlbumTile album={{ ...album, year: -1 }} messages={messages} onOpen={vi.fn()} />);
  expect(negative.container.querySelector('[data-album-facts="1"]')?.textContent).toStrictEqual('1 track · 3:00');
});
