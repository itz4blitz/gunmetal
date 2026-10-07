import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { stubPlayback } from '../test-playback.ts';
import { demoLocalFilter } from '../../../../fake-server/src/filter.ts';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import type { ShellLibrary } from '../library-types.ts';
import { Shell } from '../Shell.tsx';

afterEach(cleanup);

test('missing album itemId and empty shell without library keep closed routes', () => {
  const library = demoLibrary();
  const missing = render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
      historyState={{ itemId: 'demo-album-99' }}
    />,
  );
  expect(screen.getByRole('heading', { name: 'That album is not in the demo library' })).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Previous' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Next' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: 'Enter' });
  fireEvent.click(screen.getByRole('button', { name: 'Play' }));
  missing.unmount();

  const emptyHome = render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} path="/" widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  emptyHome.unmount();
  const emptySearch = render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} path="/search" widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  emptySearch.unmount();
  const emptyLibrary = render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} path="/library" widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  emptyLibrary.unmount();
  const emptySettings = render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} path="/settings" widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  emptySettings.unmount();
});

test('artist rows with no albums and untitled discs are reachable', () => {
  const base = demoLibrary();
  const first = base.albums[0]!;
  const library: ShellLibrary = {
    albums: [
      {
        ...first,
        id: 'demo-album-custom',
        discs: [
          { index: 1, title: '' },
          { index: 2, title: 'Named Disc' },
        ],
        tracks: [
          { ...first.tracks[0]!, id: 'demo-track-custom-1', albumId: 'demo-album-custom', discIndex: 1 },
          { ...first.tracks[1]!, id: 'demo-track-custom-2', albumId: 'demo-album-custom', discIndex: 2 },
        ],
      },
    ],
    artists: [
      { key: 'lonely', name: 'Lonely Artist', albumIds: [] },
      { key: 'blank', name: '   ', albumIds: [] },
      { key: 'custom', name: first.artistName, albumIds: ['demo-album-custom'] },
    ],
  };
  render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1200}
      library={library}
    />,
  );
  fireEvent.click(screen.getByRole('tab', { name: 'Artists' }));
  expect(document.querySelector('[data-artist-avatar="1"]')?.textContent).toStrictEqual('L');
  expect(
    [...document.querySelectorAll('[data-artist-avatar="1"]')].some((node) => node.textContent === '?'),
  ).toStrictEqual(true);
  fireEvent.click(screen.getByRole('button', { name: 'Lonely Artist' }));
  expect(screen.getByRole('heading', { name: 'Lonely Artist' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  fireEvent.click(screen.getByRole('tab', { name: 'Artists' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lonely Artist' }), { key: 'Enter' });
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  fireEvent.click(screen.getByRole('tab', { name: 'Artists' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lonely Artist' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: first.artistName }), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: first.artistName }));
  expect(screen.getByRole('heading', { name: first.artistName }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: first.title }));
  expect(screen.getByRole('heading', { name: 'Discs 1' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Named Disc' })).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  fireEvent.keyDown(screen.getByRole('button', { name: first.tracks[0]!.title }), { key: 'Tab' });
});

test('uncontrolled shell opens and leaves album detail through history', () => {
  const library = demoLibrary();
  window.history.pushState(null, '', '/library');
  render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} widthPx={1600} library={library} />,
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Night Shift' }), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
});

test('hostile fixture album uses catalogue labels instead of corpus text in chrome', () => {
  const library = demoLibrary();
  render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Hostile metadata (fixture)' }));
  expect(screen.getByRole('heading', { name: 'Hostile metadata (fixture)' }).id).toStrictEqual('destination-headline');
  expect(screen.getByText('Security corpus')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
});

test('home recently-added empty card appears when every fixture album is hostile', () => {
  const base = demoLibrary();
  const hostile = base.albums.find((album) => album.hostile);
  expect(hostile).toBeTruthy();
  const library: ShellLibrary = {
    albums: [hostile!],
    artists: base.artists,
  };
  render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/"
      widthPx={1600}
      library={library}
    />,
  );
  expect(screen.getByText('No albums added yet')).toBeTruthy();
  expect(document.querySelector('#home-row-recent [data-empty-card="1"]')).toBeTruthy();
  expect(document.querySelector('#home-row-recent [data-empty-title="1"]')?.textContent).toStrictEqual(
    'Recently added',
  );
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toBeNull();
  expect(document.querySelector('#home-spotlight')).toBeNull();
});

test('home see all navigates to the library destination', () => {
  const library = demoLibrary();
  render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/"
      widthPx={1600}
      library={library}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'See all' }));
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
});

test('history itemId artist key and go to artist open the artist destination', () => {
  const library = demoLibrary();
  const fromHistoryStub = stubPlayback();
  const fromHistory = render(
    <Shell
      playback={fromHistoryStub.controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
      historyState={{ scrollY: 0, itemId: 'mira-sol' }}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Mira Sol' }).id).toStrictEqual('destination-headline');
  expect(document.querySelector('#destination-artist')?.getAttribute('data-artist-key')).toStrictEqual('mira-sol');
  expect(document.querySelector('#destination-artist')?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(document.querySelector('[data-artist-hero="1"]')).toBeTruthy();
  expect(document.querySelector('[data-artist-avatar-nut="1"]')).toBeTruthy();
  expect(document.querySelector('#artist-album-grid')).toBeTruthy();
  fireEvent.click(document.querySelector('#artist-play')!);
  expect(fromHistoryStub.calls).toStrictEqual(['playAlbum']);
  fromHistory.unmount();

  const fromMenu = render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
    />,
  );
  fireEvent.contextMenu(document.querySelector('#album-tile-demo-album-07')!);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(screen.getByRole('heading', { name: 'Keratin' }).id).toStrictEqual('destination-headline');
  expect(document.querySelector('#destination-artist')?.getAttribute('data-artist-key')).toStrictEqual('keratin');
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  fireEvent.click(screen.getByRole('button', { name: 'Go to artist' }));
  expect(screen.getByRole('heading', { name: 'Mira Sol' }).id).toStrictEqual('destination-headline');
  fromMenu.unmount();
});

test('unresolvable album or track play leaves the full player closed', () => {
  const base = demoLibrary();
  const emptyAlbum = {
    ...base.albums[0]!,
    id: 'demo-album-empty',
    title: 'Silent Shelf',
    tracks: [],
  };
  const mismatched = {
    ...base.albums[1]!,
    id: 'demo-album-host',
    title: 'Host Album',
    tracks: [
      {
        ...base.albums[1]!.tracks[0]!,
        id: 'demo-track-orphan',
        albumId: 'demo-album-missing',
        title: 'Orphan Click',
      },
    ],
  };
  const library: ShellLibrary = {
    albums: [emptyAlbum, mismatched],
    artists: base.artists,
  };
  render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Silent Shelf' }));
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(document.querySelector('#player-full')).toBeNull();
  expect(document.querySelector('#player-empty')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Enter' });
  fireEvent.click(screen.getByRole('button', { name: 'Host Album' }));
  fireEvent.click(screen.getByRole('button', { name: 'Orphan Click' }));
  expect(document.querySelector('#player-full')).toBeNull();
  expect(document.querySelector('#player-empty')).toBeTruthy();
});
