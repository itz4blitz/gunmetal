import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { stubPlayback, queuedSnapshot } from '../test-playback.ts';
import { demoLocalFilter } from '../../../../fake-server/src/filter.ts';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { pluginSlots } from '../../../../fake-server/src/plugin-slots.ts';
import type { ShellLibrary } from '../library-types.ts';
import { catalogue } from '../../messages/catalogue.ts';
import { matchAddress } from '../../router/match.ts';
import { Shell } from '../Shell.tsx';
import { Destination } from './Destination.tsx';
import { Store } from './Store.tsx';

/** A lookup that must land: the test names what it could not find. */
function required<T extends Element>(node: T | null | undefined, what: string): T {
  if (node === null || node === undefined) {
    throw new Error(`${what} missing`);
  }
  return node;
}

afterEach(() => {
  cleanup();
  window.history.pushState(null, '', '/');
});

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
  expect(screen.getByRole('heading', { name: 'That album is not in the demo library' })).not.toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Previous' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Next' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Queue' }), { key: 'Enter' });
  fireEvent.click(screen.getByRole('button', { name: 'Play' }));
  missing.unmount();

  window.history.pushState(null, '', '/');
  const emptyHome = render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} path="/" widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  emptyHome.unmount();
  window.history.pushState(null, '', '/search');
  const emptySearch = render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} path="/search" widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  emptySearch.unmount();
  window.history.pushState(null, '', '/library');
  const emptyLibrary = render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} path="/library" widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  emptyLibrary.unmount();
  window.history.pushState(null, '', '/settings');
  const emptySettings = render(
    <Shell playback={stubPlayback().controller} searchLibrary={demoLocalFilter} path="/settings" widthPx={1600} />,
  );
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  emptySettings.unmount();
});

test('artist rows with no albums and untitled discs are reachable', () => {
  const base = demoLibrary();
  const first = base.albums[0];
  if (first === undefined) {
    throw new Error('fixture album missing');
  }
  const firstTrackA = first.tracks[0];
  if (firstTrackA === undefined) {
    throw new Error('fixture track missing');
  }
  const firstTrackB = first.tracks[1];
  if (firstTrackB === undefined) {
    throw new Error('fixture track missing');
  }
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
          { ...firstTrackA, id: 'demo-track-custom-1', albumId: 'demo-album-custom', discIndex: 1 },
          { ...firstTrackB, id: 'demo-track-custom-2', albumId: 'demo-album-custom', discIndex: 2 },
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
  expect(screen.getByRole('heading', { name: 'Discs 1' })).not.toBeNull();
  expect(screen.getByRole('heading', { name: 'Named Disc' })).not.toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  fireEvent.keyDown(screen.getByRole('button', { name: firstTrackA.title }), { key: 'Tab' });
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
  expect(screen.getByText('Security corpus')).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
});

test('home recently-added empty card appears when every fixture album is hostile', () => {
  const base = demoLibrary();
  const hostile = base.albums.find((album) => album.hostile);
  if (hostile === undefined) {
    throw new Error('fixture hostile album missing');
  }
  expect(hostile.id).toStrictEqual('demo-album-08');
  const library: ShellLibrary = {
    albums: [hostile],
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
  expect(screen.getByText('No albums added yet')).not.toBeNull();
  expect(document.querySelector('#home-row-recent [data-empty-card="1"]')).not.toBeNull();
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

test('the album in the playback state is the one tile with the where-you-are state', () => {
  const library = demoLibrary();
  render(
    <Shell
      playback={stubPlayback(queuedSnapshot()).controller}
      searchLibrary={demoLocalFilter}
      path="/"
      widthPx={1600}
      library={library}
    />,
  );
  // queuedSnapshot() plays demo-album-01; exactly that tile is marked playing.
  expect(document.querySelector('[data-album-tile="demo-album-01"]')?.getAttribute('data-tile-playing')).toStrictEqual(
    '1',
  );
  expect(document.querySelector('[data-album-tile="demo-album-02"]')?.getAttribute('data-tile-playing')).toStrictEqual(
    '0',
  );
  expect(document.querySelectorAll('[data-tile-playing="1"]')).toHaveLength(1);
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
  expect(document.querySelector('[data-artist-hero="1"]')).not.toBeNull();
  expect(document.querySelector('[data-artist-avatar-nut="1"]')).not.toBeNull();
  expect(document.querySelector('#artist-album-grid')).not.toBeNull();
  fireEvent.click(required(document.querySelector('#artist-play'), '#artist-play'));
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
  fireEvent.contextMenu(required(document.querySelector('#album-tile-demo-album-07'), '#album-tile-demo-album-07'));
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
  const baseFirst = base.albums[0];
  if (baseFirst === undefined) {
    throw new Error('fixture album missing');
  }
  const baseSecond = base.albums[1];
  if (baseSecond === undefined) {
    throw new Error('fixture album missing');
  }
  const baseSecondTrack = baseSecond.tracks[0];
  if (baseSecondTrack === undefined) {
    throw new Error('fixture track missing');
  }
  const emptyAlbum = {
    ...baseFirst,
    id: 'demo-album-empty',
    title: 'Silent Shelf',
    tracks: [],
  };
  const mismatched = {
    ...baseSecond,
    id: 'demo-album-host',
    title: 'Host Album',
    tracks: [
      {
        ...baseSecondTrack,
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
  // The page's own play, in the action rail (the artist's other release is
  // listed below with a play of its own).
  fireEvent.click(document.querySelector('#album-play') as HTMLElement);
  expect(document.querySelector('#player-full')).toBeNull();
  expect(document.querySelector('#player-empty')).not.toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Enter' });
  fireEvent.click(screen.getByRole('button', { name: 'Host Album' }));
  fireEvent.click(screen.getByRole('button', { name: 'Orphan Click' }));
  expect(document.querySelector('#player-full')).toBeNull();
  expect(document.querySelector('#player-empty')).not.toBeNull();
});

test('an album page lists other releases by its artist, and nothing for a one-release artist', () => {
  const library = demoLibrary();
  const { calls, controller } = stubPlayback();
  const view = render(
    <Shell
      playback={controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
      historyState={{ itemId: 'demo-album-01' }}
    />,
  );
  // Mira Sol has Harbour Lights (this page) and Night Shift: only the other one is listed.
  expect(screen.getByRole('heading', { name: 'More by Mira Sol' }).getAttribute('data-section-heading')).toStrictEqual(
    '1',
  );
  expect([...document.querySelectorAll('#album-more-by [data-album-tile]')].map((tile) => tile.id)).toStrictEqual([
    'album-tile-demo-album-02',
  ]);
  // The tile's menu reaches the album-level queue actions the shell wired.
  fireEvent.click(document.querySelector('#album-more-by [data-item-more="1"]') as HTMLElement);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  fireEvent.click(document.querySelector('#album-more-by [data-item-more="1"]') as HTMLElement);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(calls).toStrictEqual(['playNextAlbum', 'addAlbumToQueue']);
  // Opening it moves to that album, whose own "more" is the first one.
  fireEvent.click(screen.getByRole('button', { name: 'Night Shift' }));
  expect(screen.getByRole('heading', { name: 'Night Shift' }).id).toStrictEqual('destination-headline');
  expect([...document.querySelectorAll('#album-more-by [data-album-tile]')].map((tile) => tile.id)).toStrictEqual([
    'album-tile-demo-album-01',
  ]);
  view.unmount();
  window.history.pushState(null, '', '/library');

  // The Compound has one release in the fixture: no section at all.
  render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
      historyState={{ itemId: 'demo-album-05' }}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Stages' }).id).toStrictEqual('destination-headline');
  expect(document.querySelector('#album-more-by')).toStrictEqual(null);
});

test('a hostile album recommends nothing and is never recommended', () => {
  const base = demoLibrary();
  const hostile = base.albums.find((album) => album.hostile);
  const clean = base.albums.find((album) => !album.hostile);
  expect(hostile?.id).toStrictEqual('demo-album-08');
  expect(clean?.id).toStrictEqual('demo-album-01');
  // Put a clean album under the hostile album's artist key: a shared key must
  // not surface either one beside the other.
  const library: ShellLibrary = {
    ...base,
    albums: base.albums.map((album) =>
      album.id === 'demo-album-01' ? { ...album, artistKey: 'hostile-artist' } : album,
    ),
  };
  expect(library.albums.find((album) => album.id === 'demo-album-08')?.artistKey).toStrictEqual('hostile-artist');
  const onHostile = render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
      historyState={{ itemId: 'demo-album-08' }}
    />,
  );
  expect(document.querySelector('#destination-album')?.getAttribute('data-hostile')).toStrictEqual('1');
  expect(document.querySelector('#album-more-by')).toStrictEqual(null);
  onHostile.unmount();

  render(
    <Shell
      playback={stubPlayback().controller}
      searchLibrary={demoLocalFilter}
      path="/library"
      widthPx={1600}
      library={library}
      historyState={{ itemId: 'demo-album-01' }}
    />,
  );
  expect(document.querySelector('#destination-album')?.getAttribute('data-hostile')).toStrictEqual('0');
  expect(document.querySelector('#album-more-by')).toStrictEqual(null);
});

test('unknown addresses and the search route render their own destinations', () => {
  const base = {
    searchLibrary: demoLocalFilter,
    lyricsFor: () => [] as string[],
    pluginSlots: [],
    messages: catalogue(),
    library: demoLibrary(),
    itemId: undefined,
    theme: 'dark' as const,
    onThemeChange: vi.fn(),
    onOpenAlbum: vi.fn(),
    onOpenArtist: vi.fn(),
    onBackFromAlbum: vi.fn(),
    onPlayAlbum: vi.fn(),
    onPlayTrack: vi.fn(),
    onSeeAll: vi.fn(),
    width: 'wide' as const,
  };
  // An unknown path is the not-found headline, keyed for the page animation.
  const lost = render(
    <Destination {...base} match={matchAddress({ pathname: '/nowhere', search: '', hash: '', state: null })} />,
  );
  expect(screen.getByRole('heading', { name: 'Not found' }).id).toStrictEqual('destination-headline');
  lost.unmount();
  // The search route hands the library to the Search surface.
  const search = render(
    <Destination {...base} match={matchAddress({ pathname: '/search', search: '', hash: '', state: null })} />,
  );
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  expect(document.querySelector('#destination-search')).not.toBeNull();
  search.unmount();
});

test('the library error and retry travel from the destination to the page', () => {
  const library = demoLibrary();
  const onLibraryRetry = vi.fn();
  render(
    <Destination
      searchLibrary={demoLocalFilter}
      lyricsFor={() => []}
      pluginSlots={[]}
      match={matchAddress({ pathname: '/library', search: '', hash: '', state: null })}
      messages={catalogue()}
      library={library}
      itemId={undefined}
      theme="dark"
      onThemeChange={vi.fn()}
      onOpenAlbum={vi.fn()}
      onOpenArtist={vi.fn()}
      onBackFromAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
      onSeeAll={vi.fn()}
      width="wide"
      libraryError="The scan did not finish"
      onLibraryRetry={onLibraryRetry}
    />,
  );
  expect(document.querySelector('#library-error')).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
  expect(onLibraryRetry).toHaveBeenCalledTimes(1);
});

test('a media address with no item shows the kind, and a loaded library says it is missing', () => {
  const base = {
    searchLibrary: demoLocalFilter,
    lyricsFor: () => [] as string[],
    pluginSlots: [],
    messages: catalogue(),
    theme: 'dark' as const,
    onThemeChange: vi.fn(),
    onOpenAlbum: vi.fn(),
    onOpenArtist: vi.fn(),
    onBackFromAlbum: vi.fn(),
    onPlayAlbum: vi.fn(),
    onPlayTrack: vi.fn(),
    onSeeAll: vi.fn(),
    width: 'wide' as const,
  };
  const waiting = render(
    <Destination
      {...base}
      library={undefined}
      itemId={undefined}
      match={matchAddress({ pathname: '/music/albums/harbour-lights', search: '', hash: '', state: null })}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  expect(document.querySelector('[data-media-empty]')).toBeNull();
  waiting.unmount();

  const cases = [
    ['/music/albums/not-here', 'album', 'That album is not in this library.'],
    ['/music/artists/not-here', 'artist', 'That artist is not in this library.'],
    ['/music/tracks/not-here', 'track', 'That song is not in this library.'],
    ['/watch/movies/inception', 'movie', 'Movies are not connected to this library yet. This address is ready for them.'],
    ['/watch/shows/the-wire', 'show', 'TV shows are not connected to this library yet. This address is ready for them.'],
  ] as const;
  for (const [pathname, kind, copy] of cases) {
    const view = render(
      <Destination
        {...base}
        library={demoLibrary()}
        itemId={undefined}
        match={matchAddress({ pathname, search: '', hash: '', state: null })}
      />,
    );
    expect(document.querySelector('[data-media-kind]')?.getAttribute('data-media-kind')).toStrictEqual(kind);
    expect(document.querySelector('[data-media-empty]')?.textContent).toStrictEqual(copy);
    view.unmount();
  }
});

// Verifies: SEC-EXT-018
test('the store route shows the catalogue and opens an extension page, not settings', () => {
  const onOpenPath = vi.fn();
  localStorage.removeItem('gunmetal.extension.choices');
  render(
    <Destination
      searchLibrary={demoLocalFilter}
      lyricsFor={() => []}
      pluginSlots={[]}
      match={matchAddress({ pathname: '/store', search: '', hash: '', state: null })}
      messages={catalogue()}
      library={demoLibrary()}
      itemId={undefined}
      theme="dark"
      onThemeChange={vi.fn()}
      onOpenAlbum={vi.fn()}
      onOpenArtist={vi.fn()}
      onBackFromAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
      onSeeAll={vi.fn()}
      onOpenPath={onOpenPath}
      width="wide"
    />,
  );
  expect(matchAddress({ pathname: '/store', search: '', hash: '', state: null })).toStrictEqual({
    kind: 'ok',
    route: { path: '/store', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    history: { scrollY: 0, itemId: undefined },
  });
  expect(screen.getByRole('heading', { name: 'Store' }).id).toStrictEqual('destination-headline');
  expect(document.querySelector('#destination-store')).not.toBeNull();
  expect(document.querySelector('#destination-settings')).toBeNull();
  expect(document.querySelector('[data-store-home]')?.textContent).toStrictEqual('itz4blitz/gunmetal-extensions');
  expect(document.querySelector('[data-store-lede]')?.textContent).toStrictEqual(
    'A pull request merged into main lists a record here. That does not install it. On this server means this server already runs the job.',
  );
  expect(
    [...document.querySelectorAll('[data-store-card]')].map((node) => [
      node.getAttribute('data-store-card'),
      node.querySelector('[data-store-status]')?.textContent,
    ]),
  ).toStrictEqual([
    ['cover-art', 'On this server'],
    ['lyrics', 'In the store'],
    ['catalogue-search', 'In the store'],
    ['scrobble', 'In the store'],
    ['themes', 'In the store'],
    ['home-rows', 'In the store'],
    ['url-style', 'On this server'],
  ]);
  const card = screen.getByRole('button', { name: 'Metadata and artwork' });
  expect(card.getAttribute('data-store-card')).toStrictEqual('cover-art');
  expect(card.textContent).toContain(
    'Fills missing album art and artist photos from MusicBrainz and Cover Art Archive.',
  );
  expect(card.textContent).toContain('On this server');
  expect(screen.getByRole('button', { name: 'Uninstall Metadata and artwork' })).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Install Lyrics lookup' })).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Install Lyrics lookup' }));
  expect(onOpenPath).not.toHaveBeenCalled();
  fireEvent.click(card);
  expect(onOpenPath).toHaveBeenCalledTimes(1);
  expect(onOpenPath).toHaveBeenCalledWith('/store/cover-art');
  fireEvent.keyDown(card, { key: 'Enter' });
  expect(onOpenPath).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(card, { key: ' ' });
  expect(onOpenPath).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(card, { key: 'Tab' });
  expect(onOpenPath).toHaveBeenCalledTimes(3);
});

test('a record that arrived from a merged pull request is listed and not opened as a page', () => {
  const onOpenPath = vi.fn();
  render(
    <Store
      messages={catalogue().destinations}
      listings={[
        {
          id: 'desk-lamp',
          title: 'Desk lamp',
          version: '1.0.0',
          plane: 'client',
          slot: 'theme-pack',
          status: 'not-in-build',
          summary: 'Would add a lamp colour as data.',
          detail: ['A merge lists it.'],
          grants: ['theme:apply'],
        },
      ]}
      onOpenPath={onOpenPath}
    />,
  );
  expect(document.querySelector('[data-store-card="desk-lamp"]')?.textContent).toContain('In the store');
  fireEvent.click(screen.getByRole('button', { name: 'Desk lamp' }));
  expect(onOpenPath).not.toHaveBeenCalled();
});
