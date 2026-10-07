import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum } from '../library-types.ts';
import { AlbumDetail } from './AlbumDetail.tsx';

/** A lookup that must land: the test names what it could not find. */
function required<T extends Element>(node: T | null | undefined, what: string): T {
  if (node === null || node === undefined) {
    throw new Error(`${what} missing`);
  }
  return node;
}

afterEach(cleanup);

const album: ShellAlbum = {
  id: 'demo-album-01',
  title: 'Harbour Lights',
  artistName: 'Mira Sol',
  artistKey: 'mira-sol',
  year: 2021,
  coverTone: '01',
  coverUrl: '/media/covers/fixture.svg',
  hostile: false,
  discs: [{ index: 1, title: '' }],
  tracks: [
    {
      id: 'demo-track-01-01',
      albumId: 'demo-album-01',
      discIndex: 1,
      number: 1,
      title: 'Pier at Dusk',
      artistName: 'Mira Sol',
      durationMs: 214_000,
      flag: 'ok',
      lyricsKind: 'none',
      mediaUrl: '/media/audio/fixtures.wav',
    },
    {
      id: 'demo-track-01-02',
      albumId: 'demo-album-01',
      discIndex: 1,
      number: 2,
      title: 'Salt Window',
      artistName: 'Mira Sol',
      durationMs: 198_000,
      flag: 'ok',
      lyricsKind: 'none',
      mediaUrl: '/media/audio/fixtures.wav',
    },
  ],
};

const fixtureTrackA = album.tracks[0];
if (fixtureTrackA === undefined) {
  throw new Error('fixture track 01-01 missing');
}
const fixtureTrackB = album.tracks[1];
if (fixtureTrackB === undefined) {
  throw new Error('fixture track 01-02 missing');
}

function renderAlbum(overrides: Partial<Parameters<typeof AlbumDetail>[0]> = {}): ReturnType<typeof render> {
  return render(
    <AlbumDetail
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      album={album}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
      {...overrides}
    />,
  );
}

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

test('the single-disc track table windows its rows against the content pane', () => {
  const long: ShellAlbum = {
    ...album,
    tracks: Array.from({ length: 40 }, (_, index) => ({
      ...fixtureTrackA,
      id: `demo-track-01-${`${index + 1}`.padStart(2, '0')}`,
      number: index + 1,
      title: `Pier at Dusk ${index + 1}`,
    })),
  };
  const scroller = fakeScroller(936, 104);
  renderAlbum({ album: long, getScroller: () => scroller.element });
  // 936px down at 52px album rows = row 18 first; overscan bounds the slice.
  const shown = [...document.querySelectorAll('[data-track-row]')].map((row) => row.id);
  const firstVisible = Math.floor(936 / 52);
  expect(shown).toStrictEqual(
    long.tracks
      .slice(firstVisible - 8, Math.min(40, firstVisible + Math.ceil(104 / 52) + 8))
      .map((track) => `track-row-${track.id}`),
  );
});

test('a multi-disc album keeps its per-disc tables eager', () => {
  const scroller = fakeScroller(5000, 50);
  const twoDiscs: ShellAlbum = {
    ...album,
    discs: [
      { index: 1, title: '' },
      { index: 2, title: 'Named Disc' },
    ],
    tracks: [
      { ...fixtureTrackA, id: 'demo-track-01-01', discIndex: 1 },
      { ...fixtureTrackB, id: 'demo-track-01-02', discIndex: 2 },
    ],
  };
  renderAlbum({ album: twoDiscs, getScroller: () => scroller.element });
  expect(document.querySelectorAll('[data-track-row]').length).toStrictEqual(2);
});

test('an unmeasurable viewport renders the whole album table (jsdom)', () => {
  renderAlbum();
  expect(document.querySelectorAll('[data-track-row]').length).toStrictEqual(2);
});

test('a hostile single-disc album windows its hidden-label rows like any table', () => {
  const hostileSingle: ShellAlbum = {
    ...album,
    hostile: true,
    title: 'x"><img src=x onerror=alert(1)><script>window.__gm_xss=1</script>',
    artistName: 'x"><img src=x onerror=alert(1)><script>window.__gm_xss=1</script>',
    tracks: Array.from({ length: 20 }, (_, index) => ({
      ...fixtureTrackA,
      id: `demo-track-01-h${`${index + 1}`.padStart(2, '0')}`,
      number: index + 1,
    })),
  };
  const scroller = fakeScroller(520, 52);
  renderAlbum({ album: hostileSingle, getScroller: () => scroller.element });
  // The window applies to the hostile rows too, and every visible row is the
  // safe catalogue label — the corpus reaches no tree.
  const shown = [...document.querySelectorAll('[data-hostile-row="1"]')].map((row) => row.id);
  const firstVisible = Math.floor(520 / 52);
  expect(shown).toStrictEqual(
    hostileSingle.tracks
      .slice(firstVisible - 8, Math.min(20, firstVisible + Math.ceil(52 / 52) + 8))
      .map((track) => `track-row-${track.id}`),
  );
  expect(document.body.textContent).not.toContain('window.__gm_xss');
});

test('album detail paints a full-bleed cover-tone header with meta line and brass play', () => {
  const onPlayAlbum = vi.fn();
  const { container } = renderAlbum({ onPlayAlbum });
  const root = container.querySelector('#destination-album');
  expect(root?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(container.querySelector('[data-album-header-large="1"]')).not.toBeNull();
  expect(container.querySelector('[data-album-header-bleed="1"]')).not.toBeNull();
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  expect(container.querySelector('[data-album-artist]')?.textContent).toStrictEqual('Mira Sol');
  // Meta line: year · computed track count · computed total time (tabular segments).
  expect(container.querySelector('#album-year')?.textContent).toStrictEqual('2021');
  expect(container.querySelector('[data-album-meta="1"]')).not.toBeNull();
  expect(screen.getByText('2 tracks').id).toStrictEqual('album-track-count');
  expect(container.querySelector('#album-duration-total')?.textContent).toStrictEqual('6:52');
  expect(container.querySelector('#album-play[data-brass-hex="1"]')).not.toBeNull();
  // Eyebrow over the title, and an icon on the back control.
  expect(container.querySelector('[data-album-header-text="1"] [data-detail-eyebrow="1"]')?.textContent).toStrictEqual(
    'Album',
  );
  expect(container.querySelector('#album-back svg')?.getAttribute('data-icon')).toStrictEqual('back');
  expect(container.querySelector('#album-back')?.textContent).toStrictEqual('Back');
  fireEvent.click(required(container.querySelector('[data-album-artist]'), '[data-album-artist]'));
  fireEvent.keyDown(required(container.querySelector('[data-album-artist]'), '[data-album-artist]'), { key: 'Enter' });
  fireEvent.keyDown(required(container.querySelector('[data-album-artist]'), '[data-album-artist]'), { key: ' ' });
  fireEvent.keyDown(required(container.querySelector('[data-album-artist]'), '[data-album-artist]'), { key: 'Tab' });
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
});

test('the hero bloom mirrors the artwork behind an aria-hidden scrim layer', () => {
  const { container } = renderAlbum();
  const bloom = container.querySelector('[data-album-bloom="1"]') as HTMLElement;
  expect(bloom.getAttribute('aria-hidden')).toStrictEqual('true');
  expect(bloom.style.backgroundImage).toContain('/media/covers/fixture.svg');
  expect(container.querySelector('[data-album-scrim="1"]')?.getAttribute('aria-hidden')).toStrictEqual('true');
  // The bloom is decoration: it must never carry readable corpus text.
  expect(bloom.textContent).toStrictEqual('');

  const bare = renderAlbum({ album: { ...album, coverUrl: '' } });
  const bareBloom = bare.container.querySelector('[data-album-bloom="1"]') as HTMLElement;
  expect(bareBloom).not.toBeNull();
  expect(bareBloom.style.backgroundImage).toStrictEqual('');
});

test('shuffle sits in the action rail, disabled with its hint until the shell wires it', () => {
  const onShuffleAlbum = vi.fn();
  const unwired = renderAlbum();
  const rail = document.querySelector('[data-album-rail="1"]');
  expect(rail?.getAttribute('data-album-actions')).toStrictEqual('1');
  expect(document.querySelector('[data-shuffle-wrap="1"][data-wired="0"]')).not.toBeNull();
  expect(document.querySelector('#album-shuffle[data-hex-face="1"]')).not.toBeNull();
  expect(screen.getByText('Shuffle is not wired in this demo yet')).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Shuffle' }).getAttribute('aria-disabled')).toStrictEqual('true');
  fireEvent.click(screen.getByRole('button', { name: 'Shuffle' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Shuffle' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Shuffle' }), { key: ' ' });
  expect(onShuffleAlbum).not.toHaveBeenCalled();
  unwired.unmount();

  const wired = renderAlbum({ onShuffleAlbum });
  expect(document.querySelector('[data-shuffle-wrap="1"][data-wired="1"]')).not.toBeNull();
  expect(screen.getByRole('button', { name: 'Shuffle' }).getAttribute('aria-disabled')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Shuffle' }));
  expect(onShuffleAlbum).toHaveBeenCalledTimes(1);
  expect(onShuffleAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Shuffle' }), { key: 'Enter' });
  expect(onShuffleAlbum).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Shuffle' }), { key: ' ' });
  expect(onShuffleAlbum).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Shuffle' }), { key: 'Tab' });
  expect(onShuffleAlbum).toHaveBeenCalledTimes(3);
  expect(screen.getByText('Shuffle')).not.toBeNull();
  expect(wired.container.querySelector('[data-album-actions="1"]')).not.toBeNull();
});

test('the sticky rail holds play, shuffle, lyrics and the kebab in one reachable row', () => {
  const onPlayAlbum = vi.fn();
  const singable: ShellAlbum = {
    ...album,
    tracks: album.tracks.map((track, index) => (index === 1 ? { ...track, lyricsKind: 'plain' as const } : track)),
  };
  const { container } = renderAlbum({ album: singable, onPlayAlbum });
  const rail = container.querySelector('[data-album-rail="1"]');
  expect(rail?.querySelector('#album-play[data-brass-hex="1"]')).not.toBeNull();
  expect(rail?.querySelector('[data-shuffle-wrap="1"]')).not.toBeNull();
  expect(rail?.querySelector('#album-lyrics-toggle')).not.toBeNull();
  expect(rail?.querySelector('#album-more')).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Enter' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(2);
});

test('the kebab opens a menu of wired album actions only, and escape closes it', () => {
  const onOpenArtist = vi.fn();
  const { container } = renderAlbum({ onOpenArtist });
  // The kebab is queried by id: every track row also carries a "More" button.
  const more = required(container.querySelector('#album-more'), '#album-more');
  expect(more.getAttribute('aria-haspopup')).toStrictEqual('menu');
  expect(more.getAttribute('aria-expanded')).toStrictEqual('false');

  fireEvent.click(more);
  expect(more.getAttribute('aria-expanded')).toStrictEqual('true');
  expect(container.querySelector('[data-album-menu="1"]')).not.toBeNull();
  // Only wired actions render — a menu item that does nothing would lie.
  expect(screen.getByRole('menuitem', { name: 'Play album' })).not.toBeNull();
  expect(screen.getByRole('menuitem', { name: 'Go to artist' })).not.toBeNull();
  expect(screen.queryByRole('menuitem', { name: 'Play next' })).toBeNull();
  expect(screen.queryByRole('menuitem', { name: 'Add to queue' })).toBeNull();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(onOpenArtist).toHaveBeenCalledWith('mira-sol');
  expect(container.querySelector('[data-album-menu="1"]')).toBeNull();
  expect(more.getAttribute('aria-expanded')).toStrictEqual('false');

  fireEvent.keyDown(more, { key: 'Enter' });
  expect(container.querySelector('[data-album-menu="1"]')).not.toBeNull();
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(container.querySelector('[data-album-menu="1"]')).toBeNull();

  fireEvent.keyDown(more, { key: ' ' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: 'Enter' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(more, { key: 'Enter' });
  expect(container.querySelector('[data-album-menu="1"]')).not.toBeNull();
  // Tab through a menu never activates it; the menu stays until Escape.
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: 'Tab' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  expect(container.querySelector('[data-album-menu="1"]')).not.toBeNull();
});

test('album-level queue actions join the kebab once the shell wires them', () => {
  const onPlayNextAlbum = vi.fn();
  const onAddAlbumToQueue = vi.fn();
  const onPlayAlbum = vi.fn();
  const { container } = renderAlbum({ onPlayNextAlbum, onAddAlbumToQueue, onPlayAlbum });
  const more = required(container.querySelector('#album-more'), '#album-more');
  fireEvent.click(more);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNextAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.click(more);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddAlbumToQueue).toHaveBeenCalledWith('demo-album-01');
  // Space opens the kebab exactly as Enter does; other keys are left alone
  // by the kebab (while the open menu still hears Escape and closes).
  fireEvent.keyDown(more, { key: ' ' });
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play album' }));
  fireEvent.keyDown(more, { key: 'Enter' });
  fireEvent.keyDown(more, { key: 'Escape' });
  fireEvent.keyDown(more, { key: ' ' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Play album' }), { key: ' ' });
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
});

test('album artist control goes to the artist when the opener is provided', () => {
  const onOpenArtist = vi.fn();
  renderAlbum({ onOpenArtist });
  fireEvent.click(screen.getByRole('button', { name: 'Go to artist' }));
  expect(onOpenArtist).toHaveBeenCalledWith('mira-sol');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to artist' }), { key: 'Enter' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to artist' }), { key: ' ' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Go to artist' }), { key: 'Tab' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
});

test('track rows mark the current track as now playing', () => {
  const onPlayTrack = vi.fn();
  const { container, rerender } = render(
    <AlbumDetail
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      album={album}
      messages={destinationMessages()}
      currentTrackId="demo-track-01-02"
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={onPlayTrack}
    />,
  );
  const current = container.querySelector('#track-row-demo-track-01-02');
  const other = container.querySelector('#track-row-demo-track-01-01');
  expect(current?.getAttribute('data-current')).toStrictEqual('1');
  expect(current?.querySelector('[data-now-playing="1"]')).not.toBeNull();
  expect(other?.getAttribute('data-current')).toStrictEqual('0');
  expect(other?.querySelector('[data-now-playing="1"]')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  rerender(
    <AlbumDetail
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      album={album}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={onPlayTrack}
    />,
  );
  expect(container.querySelector('#track-row-demo-track-01-02')?.getAttribute('data-current')).toStrictEqual('0');
});

test('album lyrics toggle paints fixture lines as Text and highlights synced first line', () => {
  const withLyrics = {
    ...album,
    tracks: [
      fixtureTrackA,
      {
        ...fixtureTrackB,
        id: 'demo-track-02-02',
        title: 'Freight Elevator',
        lyricsKind: 'synced' as const,
      },
    ],
  };
  const { container } = render(
    <AlbumDetail
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      album={withLyrics}
      messages={destinationMessages()}
      currentTrackId="demo-track-02-02"
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  expect(screen.getByRole('button', { name: 'Lyrics' }).id).toStrictEqual('album-lyrics-toggle');
  expect(document.querySelector('#album-lyrics')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  expect(container.querySelector('#album-lyrics')?.getAttribute('data-synced')).toStrictEqual('1');
  expect(
    [...container.querySelectorAll('#album-lyrics [data-lyrics-line="1"]')].map((node) => node.textContent),
  ).toStrictEqual(['Hello, hello through the static', 'handshake in the noise', 'hold the line']);
  expect(container.querySelector('#album-lyrics [data-current="1"]')?.textContent).toStrictEqual(
    'Hello, hello through the static',
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Enter' });
  expect(document.querySelector('#album-lyrics')).toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: ' ' });
  expect(document.querySelector('#album-lyrics')).not.toBeNull();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Tab' });
  expect(document.querySelector('#album-lyrics')).not.toBeNull();
});

test('a licensed album paints SPDX attribution and source; others omit the row', () => {
  const licensed: ShellAlbum = {
    ...album,
    license: {
      spdx: 'CC-BY-4.0',
      attribution: 'Cylinders by Chris Zabriskie',
      source: 'chriszabriskie.com',
    },
  };
  const { unmount } = render(
    <AlbumDetail
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      album={licensed}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  const row = document.querySelector('#album-license');
  expect(row?.getAttribute('data-album-license')).toStrictEqual('CC-BY-4.0');
  expect(row?.textContent).toStrictEqual('License CC-BY-4.0 · Cylinders by Chris Zabriskie · chriszabriskie.com');
  unmount();

  render(
    <AlbumDetail
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      album={album}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  expect(document.querySelector('#album-license')).toBeNull();
});

test('album lyrics fall back to the first plain or synced track when the current row has none', () => {
  const withLyrics = {
    ...album,
    tracks: [
      fixtureTrackA,
      {
        ...fixtureTrackB,
        id: 'demo-track-01-03',
        title: 'Letter Under Glass',
        lyricsKind: 'plain' as const,
      },
    ],
  };
  const { container, rerender } = render(
    <AlbumDetail
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      album={withLyrics}
      messages={destinationMessages()}
      currentTrackId="demo-track-01-01"
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Lyrics' }));
  expect(container.querySelector('#album-lyrics')?.getAttribute('data-synced')).toStrictEqual('0');
  expect(
    [...container.querySelectorAll('#album-lyrics [data-lyrics-line="1"]')].map((node) => node.textContent),
  ).toStrictEqual(['Hello, hello through the static', 'handshake in the noise', 'hold the line']);
  expect(container.querySelector('#album-lyrics [data-current="1"]')).toBeNull();
  rerender(
    <AlbumDetail
      lyricsFor={() => ['Hello, hello through the static', 'handshake in the noise', 'hold the line']}
      album={withLyrics}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  expect(document.querySelector('#album-lyrics')).not.toBeNull();
  expect(
    [...container.querySelectorAll('#album-lyrics [data-lyrics-line="1"]')].map((node) => node.textContent)[0],
  ).toStrictEqual('Hello, hello through the static');
});

test('multi-disc albums keep sticky disc headers with titles and a per-disc play affordance', () => {
  const staged: ShellAlbum = {
    ...album,
    id: 'demo-album-05',
    title: 'Stages',
    discs: [
      { index: 1, title: 'Act One' },
      { index: 2, title: '' },
    ],
    tracks: [
      { ...fixtureTrackA, id: 'demo-track-05-01', discIndex: 1, number: 1 },
      { ...fixtureTrackB, id: 'demo-track-05-02', discIndex: 1, number: 2 },
      {
        ...fixtureTrackA,
        id: 'demo-track-05-03',
        title: 'Intermission Tone',
        discIndex: 2,
        number: 1,
      },
    ],
  };
  const onPlayTrack = vi.fn();
  render(
    <AlbumDetail
      album={staged}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={onPlayTrack}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Act One' })).not.toBeNull();
  expect(screen.getByRole('heading', { name: 'Discs 2' })).not.toBeNull();
  // Both disc headers carry the sticky row marker; the rail pins above them.
  expect(document.querySelectorAll('[data-disc-header-row="1"]')).toHaveLength(2);
  // Each disc gets its own column row, between its header and its tracks.
  const discOrders = [...document.querySelectorAll('[data-disc-block]')].map((disc) =>
    [...disc.children].map((child) => {
      if (child.hasAttribute('data-disc-header-row')) {
        return 'disc';
      }
      return child.hasAttribute('data-track-table-head') ? 'columns' : 'row';
    }),
  );
  expect(discOrders).toStrictEqual([
    ['disc', 'columns', 'row', 'row'],
    ['disc', 'columns', 'row'],
  ]);
  expect(document.querySelector('[data-album-rail="1"]')).not.toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Play disc · Act One' }));
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-05', 'demo-track-05-01');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play disc 2' }), { key: 'Enter' });
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-05', 'demo-track-05-03');
  fireEvent.click(screen.getByRole('button', { name: 'Pier at Dusk' }));
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-05', 'demo-track-05-01');
});

test('guest artists stay labelled and same-artist rows drop the repeat; flags badge honestly', () => {
  const mixed: ShellAlbum = {
    ...album,
    tracks: [
      fixtureTrackA,
      {
        ...fixtureTrackB,
        title: 'Shortwave Map',
        artistName: 'Ivy North',
        flag: 'damaged',
      },
      {
        ...fixtureTrackA,
        id: 'demo-track-01-03',
        title: 'Codec Mirage',
        artistName: 'Ivy North',
        flag: 'unplayable',
      },
    ],
  };
  const { container } = render(
    <AlbumDetail
      album={mixed}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  const own = container.querySelector('[data-album-row="1"][data-guest="0"]');
  const guest = container.querySelector('[data-album-row="1"][data-guest="1"]');
  expect(own).not.toBeNull();
  expect(guest?.querySelector('[data-track-title]')?.textContent).toStrictEqual('Shortwave Map');
  expect(container.querySelectorAll('[data-album-row="1"][data-guest="0"]')).toHaveLength(1);
  const damaged = container.querySelector('[data-album-row="1"] [data-track-row][data-flagged="1"]');
  expect(damaged?.querySelector('[data-track-flag]')?.textContent).toStrictEqual('Damaged');
  expect(screen.getByText('Cannot play')).not.toBeNull();
  expect(container.querySelector('[data-album-row="1"] [data-track-row][data-flagged="0"]')).not.toBeNull();
});

test('a hostile album swaps chrome labels and never renders corpus text in its rows', () => {
  const payload = '"><img src=x onerror=alert(1)><script>window.__gm_xss=1</script>';
  const hostile: ShellAlbum = {
    ...album,
    hostile: true,
    title: payload,
    artistName: payload,
    tracks: [
      { ...fixtureTrackA, title: payload, artistName: payload },
      { ...fixtureTrackB, title: payload, artistName: payload },
    ],
  };
  const onPlayTrack = vi.fn();
  const { container } = render(
    <AlbumDetail
      album={hostile}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={onPlayTrack}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Hostile metadata (fixture)' })).not.toBeNull();
  expect(screen.getByText('Security corpus')).not.toBeNull();
  expect(container.textContent).not.toContain('onerror');
  expect(container.textContent).not.toContain('__gm_xss');
  expect(container.querySelectorAll('[data-hostile-row="1"]')).toHaveLength(2);
  expect(screen.getAllByText('Track title hidden (hostile metadata)')).toHaveLength(2);
  fireEvent.click(
    required(
      container.querySelector('#track-row-demo-track-01-02 [data-hostile-row-play="1"]'),
      '#track-row-demo-track-01-02 [data-hostile-row-play="1"]',
    ),
  );
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-02');
  // The hidden row plays from the keyboard, too; other keys are left alone.
  fireEvent.keyDown(
    required(
      container.querySelector('#track-row-demo-track-01-01 [data-hostile-row-play="1"]'),
      '#track-row-demo-track-01-01 [data-hostile-row-play="1"]',
    ),
    { key: 'Enter' },
  );
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-01');
  fireEvent.keyDown(
    required(
      container.querySelector('#track-row-demo-track-01-01 [data-hostile-row-play="1"]'),
      '#track-row-demo-track-01-01 [data-hostile-row-play="1"]',
    ),
    { key: ' ' },
  );
  expect(onPlayTrack).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(
    required(
      container.querySelector('#track-row-demo-track-01-01 [data-hostile-row-play="1"]'),
      '#track-row-demo-track-01-01 [data-hostile-row-play="1"]',
    ),
    { key: 'Escape' },
  );
  expect(onPlayTrack).toHaveBeenCalledTimes(3);
  // A hostile multi-disc album hides its rows per disc block, all safe.
  const hostileDiscs: ShellAlbum = {
    ...hostile,
    discs: [
      { index: 1, title: '' },
      { index: 2, title: 'Named Disc' },
    ],
    tracks: [
      { ...fixtureTrackA, id: 'demo-track-01-h1', title: payload, artistName: payload, discIndex: 1 },
      { ...fixtureTrackB, id: 'demo-track-01-h2', title: payload, artistName: payload, discIndex: 2 },
    ],
  };
  const discsView = render(
    <AlbumDetail
      album={hostileDiscs}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
      currentTrackId="demo-track-01-h1"
    />,
  );
  expect(screen.getByRole('heading', { name: 'Discs 1' })).not.toBeNull();
  expect(discsView.container.querySelectorAll('[data-hostile-row="1"]').length).toStrictEqual(2);
  expect(discsView.container.querySelectorAll('[data-hostile-row-label="1"]').length).toStrictEqual(2);
  expect(discsView.container.textContent).not.toContain('__gm_xss');
  // The playing hidden row carries the brass current marker like any row.
  const currentRow = discsView.container.querySelector('#track-row-demo-track-01-h1');
  expect(currentRow?.getAttribute('data-current')).toStrictEqual('1');
  expect(currentRow?.querySelector('[data-now-playing="1"]')).not.toBeNull();
  // A disc with no tracks offers no play control: a button over nothing
  // would lie.
  const emptyDiscs: ShellAlbum = {
    ...album,
    discs: [
      { index: 1, title: 'Named Disc' },
      { index: 2, title: '' },
    ],
    tracks: [{ ...fixtureTrackA, discIndex: 1 }],
  };
  const emptyView = render(
    <AlbumDetail
      album={emptyDiscs}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Discs 2' })).not.toBeNull();
  expect(emptyView.container.querySelector('[data-disc-block="2"] [data-disc-play="1"]')).toBeNull();
  expect(emptyView.container.querySelector('[data-disc-block="1"] [data-disc-play="1"]')).not.toBeNull();
  // The named disc's play reports itself with the disc's title and plays
  // its first track from the keyboard, too.
  const namedPlay = required(
    emptyView.container.querySelector('[data-disc-block="1"] [data-disc-play="1"]'),
    'named disc play',
  );
  fireEvent.click(namedPlay);
  expect(namedPlay.getAttribute('aria-label')).toStrictEqual('Play disc · Named Disc');
  fireEvent.keyDown(namedPlay, { key: ' ' });
  fireEvent.keyDown(namedPlay, { key: 'Escape' });
});

test('without a lyricsFor the album falls back to the honest empty lines', () => {
  const singable: ShellAlbum = {
    ...album,
    tracks: [{ ...fixtureTrackA, lyricsKind: 'plain' as const }],
  };
  render(
    <AlbumDetail
      album={singable}
      messages={destinationMessages()}
      onBack={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
  fireEvent.click(required(document.querySelector('#album-lyrics-toggle'), '#album-lyrics-toggle'));
  // The default resolver answers the quiet empty state, never invented words.
  expect(document.querySelector('#album-lyrics')?.textContent).toContain('This file has no lyrics.');
});

test('the rail controls carry icons and the track list sits under a hidden column row', () => {
  const singable: ShellAlbum = {
    ...album,
    tracks: album.tracks.map((track, index) => (index === 1 ? { ...track, lyricsKind: 'plain' as const } : track)),
  };
  const { container } = renderAlbum({ album: singable });
  expect(container.querySelector('#album-shuffle svg')?.getAttribute('data-icon')).toStrictEqual('shuffle');
  expect(container.querySelector('#album-lyrics-toggle svg')?.getAttribute('data-icon')).toStrictEqual('lyrics');
  expect(container.querySelector('#album-lyrics-toggle')?.textContent).toStrictEqual('Lyrics');
  // The kebab is an icon alone; its name is the aria-label.
  expect(container.querySelector('#album-more svg')?.getAttribute('data-icon')).toStrictEqual('more');
  expect(container.querySelector('#album-more')?.textContent).toStrictEqual('');
  expect(container.querySelector('#album-more')?.getAttribute('aria-label')).toStrictEqual('More');
  // One presentational column row for a single-disc album: # · Title · clock.
  const heads = container.querySelectorAll('[data-track-table-head="1"]');
  expect(heads).toHaveLength(1);
  expect(heads[0]?.getAttribute('aria-hidden')).toStrictEqual('true');
  expect(heads[0]?.querySelector('[data-track-table-number="1"]')?.textContent).toStrictEqual('#');
  expect(heads[0]?.querySelector('[data-track-table-title="1"]')?.textContent).toStrictEqual('Title');
  expect(heads[0]?.querySelector('[data-track-table-time="1"] svg')?.getAttribute('data-icon')).toStrictEqual('clock');
  // The column row comes before the first track row.
  const block = container.querySelector('[data-disc-block]');
  const order = [...(block?.children ?? [])].map((child) => {
    if (child.hasAttribute('data-track-table-head')) {
      return 'columns';
    }
    return child.hasAttribute('data-album-row') ? 'row' : 'heading';
  });
  expect(order).toStrictEqual(['heading', 'columns', 'row', 'row']);
});

test('other releases by the artist follow the tracks as tiles that open and play', () => {
  const onOpenAlbum = vi.fn();
  const onPlayAlbum = vi.fn();
  const other: ShellAlbum = { ...album, id: 'demo-album-02', title: 'Night Shift' };
  const bare = renderAlbum();
  // No other releases handed in: the section is absent, not an empty shell.
  expect(bare.container.querySelector('#album-more-by')).toStrictEqual(null);
  expect(screen.queryByRole('heading', { name: 'More by Mira Sol' })).toStrictEqual(null);
  bare.unmount();

  const { container } = renderAlbum({ onPlayAlbum, moreBy: { albums: [other], onOpenAlbum } });
  const section = container.querySelector('#album-more-by');
  expect(section?.getAttribute('data-album-more-by')).toStrictEqual('1');
  expect(screen.getByRole('heading', { name: 'More by Mira Sol' }).getAttribute('data-section-heading')).toStrictEqual(
    '1',
  );
  // The section comes after the track list, and holds exactly the tiles given.
  expect(container.querySelector('#destination-album')?.lastElementChild).toStrictEqual(section);
  expect(
    [...(section?.querySelectorAll('[data-album-more-by-grid="1"] > [data-album-tile]') ?? [])].map((tile) => tile.id),
  ).toStrictEqual(['album-tile-demo-album-02']);
  fireEvent.click(screen.getByRole('button', { name: 'Night Shift' }));
  expect(onOpenAlbum).toHaveBeenCalledTimes(1);
  expect(onOpenAlbum).toHaveBeenCalledWith('demo-album-02');
  // The tile's own play starts that release, not the one on the page.
  const tilePlay = section?.querySelector('[data-album-play="1"]') as HTMLElement;
  fireEvent.click(tilePlay);
  expect(onPlayAlbum).toHaveBeenCalledTimes(1);
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-02');
});

test('the album kebab is wired to its menu, toggles it, and a press elsewhere closes it', () => {
  const { container } = renderAlbum({ onOpenArtist: vi.fn() });
  const more = container.querySelector('#album-more') as HTMLElement;
  fireEvent.click(more);
  const menu = screen.getByRole('menu', { name: 'Actions' });
  expect(menu.getAttribute('data-album-menu')).toStrictEqual('1');
  expect(menu.getAttribute('data-menu-id')).toStrictEqual(more.getAttribute('aria-controls'));
  // A press on the kebab is the kebab's business; the click that follows toggles.
  fireEvent.pointerDown(more);
  expect(more.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.click(more);
  expect(more.getAttribute('aria-expanded')).toStrictEqual('false');
  expect(screen.queryByRole('menu')).toStrictEqual(null);
  // Open again: a press inside keeps it, a press on the page closes it.
  fireEvent.click(more);
  fireEvent.pointerDown(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(more.getAttribute('aria-expanded')).toStrictEqual('true');
  fireEvent.pointerDown(screen.getByRole('heading', { name: 'Harbour Lights' }));
  expect(more.getAttribute('aria-expanded')).toStrictEqual('false');
  expect(screen.queryByRole('menu')).toStrictEqual(null);
});

test('area-album.css keeps the album and artist tables on the render discipline', async () => {
  const { readFile } = await import('node:fs/promises');
  const { dirname, join } = await import('node:path');
  const { fileURLToPath } = await import('node:url');
  const here = dirname(fileURLToPath(import.meta.url));
  const css = await readFile(join(here, '../../../../../apps/demo/public/area-album.css'), 'utf8');
  // Off-screen rows and artist-grid tiles skip layout and paint.
  expect(css.includes('content-visibility: auto')).toStrictEqual(true);
  expect(css.includes('contain-intrinsic-size: auto 52px;')).toStrictEqual(true);
  expect(css.includes('#destination-artist #artist-album-grid [data-album-tile]')).toStrictEqual(true);
  expect(css.includes('contain-intrinsic-size: auto 240px;')).toStrictEqual(true);
});
