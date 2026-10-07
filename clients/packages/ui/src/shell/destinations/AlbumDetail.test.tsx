import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import type { ShellAlbum } from '../library-types.ts';
import { AlbumDetail } from './AlbumDetail.tsx';

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

test('album detail paints a full-bleed cover-tone header with meta line and brass play', () => {
  const onPlayAlbum = vi.fn();
  const { container } = renderAlbum({ onPlayAlbum });
  const root = container.querySelector('#destination-album');
  expect(root?.getAttribute('data-art-tone')).toStrictEqual('01');
  expect(container.querySelector('[data-album-header-large="1"]')).toBeTruthy();
  expect(container.querySelector('[data-album-header-bleed="1"]')).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Harbour Lights' }).id).toStrictEqual('destination-headline');
  expect(container.querySelector('[data-album-artist]')?.textContent).toStrictEqual('Mira Sol');
  // Meta line: year · computed track count · computed total time (tabular segments).
  expect(container.querySelector('#album-year')?.textContent).toStrictEqual('2021');
  expect(container.querySelector('[data-album-meta="1"]')).toBeTruthy();
  expect(screen.getByText('2 tracks').id).toStrictEqual('album-track-count');
  expect(container.querySelector('#album-duration-total')?.textContent).toStrictEqual('6:52');
  expect(container.querySelector('#album-play[data-brass-hex="1"]')).toBeTruthy();
  // Eyebrow over the title, and an icon on the back control.
  expect(container.querySelector('[data-album-header-text="1"] [data-detail-eyebrow="1"]')?.textContent).toStrictEqual(
    'Album',
  );
  expect(container.querySelector('#album-back svg')?.getAttribute('data-icon')).toStrictEqual('back');
  expect(container.querySelector('#album-back')?.textContent).toStrictEqual('Back');
  fireEvent.click(container.querySelector('[data-album-artist]')!);
  fireEvent.keyDown(container.querySelector('[data-album-artist]')!, { key: 'Enter' });
  fireEvent.keyDown(container.querySelector('[data-album-artist]')!, { key: ' ' });
  fireEvent.keyDown(container.querySelector('[data-album-artist]')!, { key: 'Tab' });
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
  expect(bareBloom).toBeTruthy();
  expect(bareBloom.style.backgroundImage).toStrictEqual('');
});

test('shuffle sits in the action rail, disabled with its hint until the shell wires it', () => {
  const onShuffleAlbum = vi.fn();
  const unwired = renderAlbum();
  const rail = document.querySelector('[data-album-rail="1"]');
  expect(rail?.getAttribute('data-album-actions')).toStrictEqual('1');
  expect(document.querySelector('[data-shuffle-wrap="1"][data-wired="0"]')).toBeTruthy();
  expect(document.querySelector('#album-shuffle[data-hex-face="1"]')).toBeTruthy();
  expect(screen.getByText('Shuffle is not wired in this demo yet')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Shuffle' }).getAttribute('aria-disabled')).toStrictEqual('true');
  fireEvent.click(screen.getByRole('button', { name: 'Shuffle' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Shuffle' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Shuffle' }), { key: ' ' });
  expect(onShuffleAlbum).not.toHaveBeenCalled();
  unwired.unmount();

  const wired = renderAlbum({ onShuffleAlbum });
  expect(document.querySelector('[data-shuffle-wrap="1"][data-wired="1"]')).toBeTruthy();
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
  expect(screen.getByText('Shuffle')).toBeTruthy();
  expect(wired.container.querySelector('[data-album-actions="1"]')).toBeTruthy();
});

test('the sticky rail holds play, shuffle, lyrics and the kebab in one reachable row', () => {
  const onPlayAlbum = vi.fn();
  const singable: ShellAlbum = {
    ...album,
    tracks: album.tracks.map((track, index) => (index === 1 ? { ...track, lyricsKind: 'plain' as const } : track)),
  };
  const { container } = renderAlbum({ album: singable, onPlayAlbum });
  const rail = container.querySelector('[data-album-rail="1"]');
  expect(rail?.querySelector('#album-play[data-brass-hex="1"]')).toBeTruthy();
  expect(rail?.querySelector('[data-shuffle-wrap="1"]')).toBeTruthy();
  expect(rail?.querySelector('#album-lyrics-toggle')).toBeTruthy();
  expect(rail?.querySelector('#album-more')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Play album' }));
  expect(onPlayAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play album' }), { key: 'Enter' });
  expect(onPlayAlbum).toHaveBeenCalledTimes(2);
});

test('the kebab opens a menu of wired album actions only, and escape closes it', () => {
  const onOpenArtist = vi.fn();
  const { container } = renderAlbum({ onOpenArtist });
  // The kebab is queried by id: every track row also carries a "More" button.
  const more = container.querySelector('#album-more')!;
  expect(more.getAttribute('aria-haspopup')).toStrictEqual('menu');
  expect(more.getAttribute('aria-expanded')).toStrictEqual('false');

  fireEvent.click(more);
  expect(more.getAttribute('aria-expanded')).toStrictEqual('true');
  expect(container.querySelector('[data-album-menu="1"]')).toBeTruthy();
  // Only wired actions render — a menu item that does nothing would lie.
  expect(screen.getByRole('menuitem', { name: 'Play album' })).toBeTruthy();
  expect(screen.getByRole('menuitem', { name: 'Go to artist' })).toBeTruthy();
  expect(screen.queryByRole('menuitem', { name: 'Play next' })).toBeNull();
  expect(screen.queryByRole('menuitem', { name: 'Add to queue' })).toBeNull();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(onOpenArtist).toHaveBeenCalledWith('mira-sol');
  expect(container.querySelector('[data-album-menu="1"]')).toBeNull();
  expect(more.getAttribute('aria-expanded')).toStrictEqual('false');

  fireEvent.keyDown(more, { key: 'Enter' });
  expect(container.querySelector('[data-album-menu="1"]')).toBeTruthy();
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(container.querySelector('[data-album-menu="1"]')).toBeNull();

  fireEvent.keyDown(more, { key: ' ' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: 'Enter' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(more, { key: 'Enter' });
  expect(container.querySelector('[data-album-menu="1"]')).toBeTruthy();
  // Tab through a menu never activates it; the menu stays until Escape.
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: 'Tab' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  expect(container.querySelector('[data-album-menu="1"]')).toBeTruthy();
});

test('album-level queue actions join the kebab once the shell wires them', () => {
  const onPlayNextAlbum = vi.fn();
  const onAddAlbumToQueue = vi.fn();
  const onPlayAlbum = vi.fn();
  const { container } = renderAlbum({ onPlayNextAlbum, onAddAlbumToQueue, onPlayAlbum });
  const more = container.querySelector('#album-more')!;
  fireEvent.click(more);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onPlayNextAlbum).toHaveBeenCalledWith('demo-album-01');
  fireEvent.click(more);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  expect(onAddAlbumToQueue).toHaveBeenCalledWith('demo-album-01');
  fireEvent.keyDown(more, { key: 'Enter' });
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
  expect(current?.querySelector('[data-now-playing="1"]')).toBeTruthy();
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
      album.tracks[0]!,
      {
        ...album.tracks[1]!,
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
  expect(document.querySelector('#album-lyrics')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Lyrics' }), { key: 'Tab' });
  expect(document.querySelector('#album-lyrics')).toBeTruthy();
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
      album.tracks[0]!,
      {
        ...album.tracks[1]!,
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
  expect(document.querySelector('#album-lyrics')).toBeTruthy();
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
      { ...album.tracks[0]!, id: 'demo-track-05-01', discIndex: 1, number: 1 },
      { ...album.tracks[1]!, id: 'demo-track-05-02', discIndex: 1, number: 2 },
      {
        ...album.tracks[0]!,
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
  expect(screen.getByRole('heading', { name: 'Act One' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Discs 2' })).toBeTruthy();
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
  expect(document.querySelector('[data-album-rail="1"]')).toBeTruthy();
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
      album.tracks[0]!,
      {
        ...album.tracks[1]!,
        title: 'Shortwave Map',
        artistName: 'Ivy North',
        flag: 'damaged',
      },
      {
        ...album.tracks[0]!,
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
  expect(own).toBeTruthy();
  expect(guest?.querySelector('[data-track-title]')?.textContent).toStrictEqual('Shortwave Map');
  expect(container.querySelectorAll('[data-album-row="1"][data-guest="0"]')).toHaveLength(1);
  const damaged = container.querySelector('[data-album-row="1"] [data-track-row][data-flagged="1"]');
  expect(damaged?.querySelector('[data-track-flag]')?.textContent).toStrictEqual('Damaged');
  expect(screen.getByText('Cannot play')).toBeTruthy();
  expect(container.querySelector('[data-album-row="1"] [data-track-row][data-flagged="0"]')).toBeTruthy();
});

test('a hostile album swaps chrome labels and never renders corpus text in its rows', () => {
  const payload = '"><img src=x onerror=alert(1)><script>window.__gm_xss=1</script>';
  const hostile: ShellAlbum = {
    ...album,
    hostile: true,
    title: payload,
    artistName: payload,
    tracks: [
      { ...album.tracks[0]!, title: payload, artistName: payload },
      { ...album.tracks[1]!, title: payload, artistName: payload },
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
  expect(screen.getByRole('heading', { name: 'Hostile metadata (fixture)' })).toBeTruthy();
  expect(screen.getByText('Security corpus')).toBeTruthy();
  expect(container.textContent).not.toContain('onerror');
  expect(container.textContent).not.toContain('__gm_xss');
  expect(container.querySelectorAll('[data-hostile-row="1"]')).toHaveLength(2);
  expect(screen.getAllByText('Track title hidden (hostile metadata)')).toHaveLength(2);
  fireEvent.click(container.querySelector('#track-row-demo-track-01-02 [data-hostile-row-play="1"]')!);
  expect(onPlayTrack).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-02');
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
