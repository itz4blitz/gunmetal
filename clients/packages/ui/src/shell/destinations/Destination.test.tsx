import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import type { ShellLibrary } from '../library-types.ts';
import { Shell } from '../Shell.tsx';

afterEach(cleanup);

test('home library search and settings destinations render fixture chrome', () => {
  const library = demoLibrary();
  const home = render(<Shell path="/" widthPx={1600} library={library} showDemoLabel />);
  expect(screen.getByRole('heading', { name: 'Continue listening' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Recently played' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Recently added' })).toBeTruthy();
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toStrictEqual(
    '01',
  );
  expect(document.querySelector('#home-spotlight')).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Open' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'See all' })).toBeTruthy();
  expect(screen.getByText('Nothing to continue yet')).toBeTruthy();
  expect(screen.getByText('Nothing played yet')).toBeTruthy();
  expect(screen.getByText('No loved tracks yet')).toBeTruthy();
  expect(document.querySelectorAll('[data-empty-card="1"]').length).toBeGreaterThanOrEqual(3);
  const continueCard = document.querySelector('#home-row-continue [data-empty-card="1"]');
  expect(continueCard?.querySelector('[data-empty-title="1"]')?.textContent).toStrictEqual(
    'Continue listening',
  );
  expect(continueCard?.querySelector('[data-empty-state="continue"]')?.textContent).toStrictEqual(
    'Nothing to continue yet',
  );
  expect(continueCard?.querySelector('[data-empty-mark="1"]')).toBeTruthy();
  expect(
    document.querySelector('#home-row-played [data-empty-title="1"]')?.textContent,
  ).toStrictEqual('Recently played');
  expect(
    document.querySelector('#home-row-loved [data-empty-title="1"]')?.textContent,
  ).toStrictEqual('Loved');
  expect(screen.getByRole('button', { name: 'Harbour Lights' })).toBeTruthy();
  home.rerender(
    <Shell
      path="/"
      widthPx={1600}
      library={library}
      showDemoLabel
      historyState={{ scrollY: 0, itemId: 'demo-album-01' }}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual(
    'destination-headline',
  );
  expect(document.querySelector('#destination-album')?.getAttribute('data-art-tone')).toStrictEqual(
    '01',
  );
  expect(document.querySelector('[data-album-header-large="1"]')).toBeTruthy();
  expect(document.querySelector('[data-album-header-bleed="1"]')).toBeTruthy();
  expect(document.querySelector('#album-play[data-brass-hex="1"]')).toBeTruthy();
  expect(screen.getByText('4 tracks').id).toStrictEqual('album-track-count');
  expect(screen.getByRole('heading', { name: 'Tracks' })).toBeTruthy();
  home.unmount();

  const libraryView = render(<Shell path="/library" widthPx={1200} library={library} />);
  expect(document.querySelector('#library-section-count')?.textContent).toStrictEqual(
    `${library.albums.length} albums`,
  );
  expect(document.querySelector('#destination')?.getAttribute('data-page-enter')).toStrictEqual('1');
  expect(document.querySelector('#library-tab-albums')?.getAttribute('data-selected')).toStrictEqual(
    '1',
  );
  fireEvent.click(screen.getByRole('tab', { name: 'Artists' }));
  expect(document.querySelector('#library-tab-artists')?.getAttribute('data-selected')).toStrictEqual(
    '1',
  );
  expect(document.querySelector('[data-artist-avatar="1"]')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Keratin' }), { key: ' ' });
  expect(screen.getByRole('heading', { name: 'Signal Loss' }).id).toStrictEqual(
    'destination-headline',
  );
  expect(screen.getByText('Cannot play')).toBeTruthy();
  expect(screen.getByText('Damaged')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  fireEvent.click(screen.getByRole('tab', { name: 'Tracks' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Codec Mirage' }), { key: 'Enter' });
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Handshake');
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Albums' }), { key: 'Enter' });
  expect(document.querySelector('#library-album-grid')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Artists' }), { key: 'Tab' });
  libraryView.unmount();

  const search = render(<Shell path="/search" widthPx={800} library={library} />);
  expect(screen.getByRole('heading', { name: 'Recent searches' })).toBeTruthy();
  expect(screen.getByText('No recent searches')).toBeTruthy();
  expect(document.querySelector('#search-recent [data-empty-title="1"]')?.textContent).toStrictEqual(
    'Recent searches',
  );
  expect(document.querySelector('#search-recent [data-empty-state="search-recent"]')?.textContent).toStrictEqual(
    'No recent searches',
  );
  expect(document.querySelector('#search-recent [data-empty-mark="1"]')).toBeTruthy();
  expect(document.querySelector('#search-affordance')).toBeTruthy();
  expect(document.querySelector('#search-field-wrap')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), {
    target: { value: 'Mira' },
  });
  expect(screen.getByText('Demo-local filter — not CorePort search')).toBeTruthy();
  expect(document.querySelector('#search-group-albums')?.textContent).toStrictEqual('Albums');
  expect(document.querySelector('#search-group-tracks')?.textContent).toStrictEqual('Tracks');
  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('[data-search-albums="1"]')).toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Albums' }), { key: 'Enter' });
  expect(document.querySelector('[data-search-albums="1"]')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Tracks' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Albums' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Albums' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: ' ' });
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).toBeTruthy();
  search.unmount();

  const searchTracks = render(<Shell path="/search" widthPx={800} library={library} />);
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), {
    target: { value: 'Pier' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Albums' }));
  expect(document.querySelector('[data-search-albums="1"]')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), {
    target: { value: 'zzzz' },
  });
  expect(screen.getByText('No matches for this query in Music')).toBeTruthy();
  expect(document.querySelector('#search-no-hits[data-empty-card="1"]')).toBeTruthy();
  expect(document.querySelector('#search-no-hits [data-empty-mark="1"]')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Tracks' }));
  fireEvent.click(screen.getByRole('button', { name: 'Tracks' }));
  searchTracks.unmount();

  const settings = render(<Shell path="/settings" widthPx={1600} library={library} />);
  expect(screen.getByRole('heading', { name: 'Appearance' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Demo data' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Playback' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'About' })).toBeTruthy();
  expect(screen.getByText(/Stage-A fixture catalogue/)).toBeTruthy();
  expect(screen.getByText(/arrive with CorePort/)).toBeTruthy();
  expect(screen.getByText(/fixture demo data only/)).toBeTruthy();
  expect(document.querySelector('#settings-appearance #theme-switcher')).toBeTruthy();
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toBeGreaterThanOrEqual(4);
  settings.unmount();
});

test('opening an album uses history itemId and play fills the bar', () => {
  const library = demoLibrary();
  render(<Shell path="/library" widthPx={1600} library={library} />);
  fireEvent.click(screen.getByRole('button', { name: 'Stages' }));
  expect(screen.getByRole('heading', { name: 'Stages' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('heading', { name: 'Act One' })).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: ' ' });
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Curtain');
  expect(document.querySelector('#player-full')?.getAttribute('data-open')).toStrictEqual('1');
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  expect(document.querySelector('#player-full')).toBeNull();
  fireEvent.click(document.querySelector('#shell-play')!);
  expect(document.querySelector('#shell-play')?.getAttribute('aria-label')).toStrictEqual('Play');
  fireEvent.keyDown(document.querySelector('#shell-play')!, { key: 'Enter' });
  fireEvent.keyDown(document.querySelector('#player-next')!, { key: ' ' });
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Understudy');
  fireEvent.keyDown(document.querySelector('#player-prev')!, { key: 'Enter' });
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Curtain');
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Enter' });
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
});

test('player art and meta open the full player sheet; Escape and Close dismiss it', () => {
  const library = demoLibrary();
  render(<Shell path="/library" widthPx={1600} library={library} />);
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(document.querySelector('#track-row-demo-track-01-01')?.getAttribute('data-current')).toStrictEqual(
    '1',
  );
  expect(document.querySelector('#track-row-demo-track-01-01 [data-now-playing="1"]')).toBeTruthy();
  expect(document.querySelector('#player-full')?.getAttribute('data-open')).toStrictEqual('1');
  expect(document.querySelector('#player-full-title')?.textContent).toStrictEqual('Pier at Dusk');
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(document.querySelector('#player-full')).toBeNull();
  fireEvent.click(screen.getAllByRole('button', { name: 'Open full player' })[0]!);
  expect(document.querySelector('#player-full')?.getAttribute('data-open')).toStrictEqual('1');
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  expect(document.querySelector('#player-full')).toBeNull();
  fireEvent.keyDown(screen.getAllByRole('button', { name: 'Open full player' })[1]!, {
    key: 'Enter',
  });
  expect(document.querySelector('#player-full')?.getAttribute('data-open')).toStrictEqual('1');
  fireEvent.click(document.querySelector('#player-full-play')!);
  expect(document.querySelector('#shell-play')?.getAttribute('data-playing')).toStrictEqual('0');
  expect(document.querySelector('#player-full-play')?.getAttribute('data-playing')).toStrictEqual(
    '0',
  );
  fireEvent.keyDown(document.querySelector('#player-full-play')!, { key: 'Enter' });
  fireEvent.keyDown(document.querySelector('#player-full-next')!, { key: ' ' });
  expect(document.querySelector('#player-full-title')?.textContent).toStrictEqual('Salt Window');
  fireEvent.keyDown(document.querySelector('#player-full-prev')!, { key: 'Enter' });
  expect(document.querySelector('#player-full-title')?.textContent).toStrictEqual('Pier at Dusk');
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  expect(document.querySelector('#player-full')).toBeNull();
  fireEvent.click(document.querySelector('#player-expand')!);
  expect(document.querySelector('#player-full')?.getAttribute('data-open')).toStrictEqual('1');
});

test('missing album itemId and empty shell without library keep closed routes', () => {
  const library = demoLibrary();
  const missing = render(
    <Shell
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

  const emptyHome = render(<Shell path="/" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Home' }).id).toStrictEqual('destination-headline');
  emptyHome.unmount();
  const emptySearch = render(<Shell path="/search" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Search' }).id).toStrictEqual('destination-headline');
  emptySearch.unmount();
  const emptyLibrary = render(<Shell path="/library" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
  emptyLibrary.unmount();
  const emptySettings = render(<Shell path="/settings" widthPx={1600} />);
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  emptySettings.unmount();
});

test('compact queue sheet opens and closes from the player queue control', () => {
  const library = demoLibrary();
  const navigated: string[] = [];
  render(
    <Shell
      path="/library"
      widthPx={360}
      library={library}
      onNavigate={(next) => {
        navigated.push(next);
      }}
    />,
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Night Shift' }), { key: ' ' });
  expect(navigated).toStrictEqual(['/library']);
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: 'Enter' });
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('0');
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: ' ' });
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('0');
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: 'Tab' });
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Tab' });
  fireEvent.keyDown(document.querySelector('#shell-play')!, { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(navigated).toStrictEqual(['/library', '/library']);
});

test('artist rows with no albums and untitled discs are reachable', () => {
  const base = demoLibrary();
  const first = base.albums[0]!;
  const library: ShellLibrary = {
    kind: 'demo-fixtures',
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
  render(<Shell path="/library" widthPx={1200} library={library} />);
  fireEvent.click(screen.getByRole('tab', { name: 'Artists' }));
  expect(document.querySelector('[data-artist-avatar="1"]')?.textContent).toStrictEqual('L');
  expect(
    [...document.querySelectorAll('[data-artist-avatar="1"]')].some((node) => node.textContent === '?'),
  ).toStrictEqual(true);
  fireEvent.click(screen.getByRole('button', { name: 'Lonely Artist' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lonely Artist' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lonely Artist' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: first.artistName }), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: first.artistName }));
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
  render(<Shell widthPx={1600} library={library} />);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Night Shift' }), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'Harbour Lights' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual(
    'destination-headline',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
});

test('hostile fixture album uses catalogue labels instead of corpus text in chrome', () => {
  const library = demoLibrary();
  render(<Shell path="/library" widthPx={1600} library={library} />);
  fireEvent.click(screen.getByRole('button', { name: 'Hostile metadata (fixture)' }));
  expect(screen.getByRole('heading', { name: 'Hostile metadata (fixture)' }).id).toStrictEqual(
    'destination-headline',
  );
  expect(screen.getByText('Security corpus')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
});

test('home recently-added empty card appears when every fixture album is hostile', () => {
  const base = demoLibrary();
  const hostile = base.albums.find((album) => album.hostile);
  expect(hostile).toBeTruthy();
  const library: ShellLibrary = {
    kind: 'demo-fixtures',
    albums: [hostile!],
    artists: base.artists,
  };
  render(<Shell path="/" widthPx={1600} library={library} />);
  expect(screen.getByText('No albums added yet')).toBeTruthy();
  expect(document.querySelector('#home-row-recent [data-empty-card="1"]')).toBeTruthy();
  expect(
    document.querySelector('#home-row-recent [data-empty-title="1"]')?.textContent,
  ).toStrictEqual('Recently added');
  expect(document.querySelector('#destination-home')?.getAttribute('data-art-tone')).toBeNull();
  expect(document.querySelector('#home-spotlight')).toBeNull();
});

test('home see all navigates to the library destination', () => {
  const library = demoLibrary();
  render(<Shell path="/" widthPx={1600} library={library} />);
  fireEvent.click(screen.getByRole('button', { name: 'See all' }));
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual(
    'destination-headline',
  );
});

test('home spotlight play fills the bar from the featured fixture album', () => {
  const library = demoLibrary();
  render(<Shell path="/" widthPx={1600} library={library} />);
  expect(
    document.querySelector('#home-spotlight [data-spotlight-eyebrow="1"]')?.textContent,
  ).toStrictEqual('Featured');
  expect(document.querySelector('#destination-headline')).toBeNull();
  fireEvent.click(document.querySelector('#home-spotlight-play')!);
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
  expect(document.querySelector('#player-full')?.getAttribute('data-open')).toStrictEqual('1');
  fireEvent.click(screen.getByRole('button', { name: 'Close' }));
  fireEvent.click(screen.getByRole('button', { name: 'Open' }));
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual(
    'destination-headline',
  );
});
