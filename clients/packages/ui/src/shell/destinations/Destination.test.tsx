import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import type { ShellLibrary } from '../library-types.ts';
import { Shell } from '../Shell.tsx';

afterEach(cleanup);

test('home library search and settings destinations render fixture chrome', () => {
  const library = demoLibrary();
  const home = render(<Shell path="/" widthPx={1600} library={library} showDemoLabel />);
  expect(screen.getByRole('heading', { name: 'Recently added' })).toBeTruthy();
  expect(screen.getByText('Nothing to continue yet')).toBeTruthy();
  expect(screen.getByText('No loved tracks yet')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: 'Enter' });
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual(
    'destination-headline',
  );
  home.unmount();

  const libraryView = render(<Shell path="/library" widthPx={1200} library={library} />);
  fireEvent.click(screen.getByRole('tab', { name: 'Artists' }));
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
  expect(screen.getByText('No recent searches')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), {
    target: { value: 'Harbour' },
  });
  expect(screen.getByText('Demo-local filter — not CorePort search')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Harbour Lights' }), { key: ' ' });
  expect(screen.getByRole('heading', { name: 'Harbour Lights' })).toBeTruthy();
  search.unmount();

  const searchTracks = render(<Shell path="/search" widthPx={800} library={library} />);
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), {
    target: { value: 'Pier' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Pier at Dusk');
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), {
    target: { value: 'zzzz' },
  });
  expect(screen.getByText('No matches for this query in Music')).toBeTruthy();
  searchTracks.unmount();

  const settings = render(<Shell path="/settings" widthPx={1600} library={library} />);
  expect(screen.getByRole('heading', { name: 'Appearance' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Playback' })).toBeTruthy();
  expect(screen.getByText(/fixture demo data only/)).toBeTruthy();
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
  fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
  expect(screen.getByRole('button', { name: 'Play' }).id).toStrictEqual('shell-play');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Next' }), { key: ' ' });
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Understudy');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Previous' }), { key: 'Enter' });
  expect(document.querySelector('#player-title')?.textContent).toStrictEqual('Curtain');
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Enter' });
  expect(screen.getByRole('heading', { name: 'Library' }).id).toStrictEqual('destination-headline');
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
  expect(document.querySelector('#queue-sheet')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: 'Enter' });
  expect(document.querySelector('#queue-sheet')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  expect(document.querySelector('#queue-sheet')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: ' ' });
  expect(document.querySelector('#queue-sheet')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Queue' }));
  expect(document.querySelector('#queue-sheet')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: 'Tab' });
  expect(document.querySelector('#queue-sheet')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Pause' }), { key: 'Tab' });
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
      { key: 'custom', name: first.artistName, albumIds: ['demo-album-custom'] },
    ],
  };
  render(<Shell path="/library" widthPx={1200} library={library} />);
  fireEvent.click(screen.getByRole('tab', { name: 'Artists' }));
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
