import { cylindersAlbum } from './commons.ts';
import { hostileCorpus } from './hostile.ts';
import type { DemoAlbum, DemoArtist, DemoLibrary, DemoTrack } from './types.ts';

function track(partial: DemoTrack): DemoTrack {
  return partial;
}

function harbourLights(): DemoAlbum {
  return {
    id: 'demo-album-01',
    title: 'Harbour Lights',
    artistName: 'Mira Sol',
    artistKey: 'mira-sol',
    year: 2021,
    coverTone: '01',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    tracks: [
      track({
        id: 'demo-track-01-01',
        albumId: 'demo-album-01',
        discIndex: 1,
        number: 1,
        title: 'Pier at Dusk',
        artistName: 'Mira Sol',
        durationMs: 214_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-01-02',
        albumId: 'demo-album-01',
        discIndex: 1,
        number: 2,
        title: 'Salt Window',
        artistName: 'Mira Sol',
        durationMs: 198_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-01-03',
        albumId: 'demo-album-01',
        discIndex: 1,
        number: 3,
        title: 'Low Tide Letter',
        artistName: 'Mira Sol',
        durationMs: 241_000,
        flag: 'ok',
        lyricsKind: 'plain',
      }),
      track({
        id: 'demo-track-01-04',
        albumId: 'demo-album-01',
        discIndex: 1,
        number: 4,
        title: 'Beacon',
        artistName: 'Mira Sol',
        durationMs: 187_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
    ],
  };
}

function nightShift(): DemoAlbum {
  return {
    id: 'demo-album-02',
    title: 'Night Shift',
    artistName: 'Mira Sol',
    artistKey: 'mira-sol',
    year: 2023,
    coverTone: '02',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    tracks: [
      track({
        id: 'demo-track-02-01',
        albumId: 'demo-album-02',
        discIndex: 1,
        number: 1,
        title: 'Clock In',
        artistName: 'Mira Sol',
        durationMs: 205_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-02-02',
        albumId: 'demo-album-02',
        discIndex: 1,
        number: 2,
        title: 'Freight Elevator',
        artistName: 'Mira Sol',
        durationMs: 232_000,
        flag: 'ok',
        lyricsKind: 'synced',
      }),
      track({
        id: 'demo-track-02-03',
        albumId: 'demo-album-02',
        discIndex: 1,
        number: 3,
        title: 'Vending Glow',
        artistName: 'Mira Sol',
        durationMs: 176_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
    ],
  };
}

function twinCities(): DemoAlbum {
  return {
    id: 'demo-album-03',
    title: 'Twin Cities',
    artistName: 'Alex Reed',
    artistKey: 'alex-reed-north',
    year: 2019,
    coverTone: '03',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    tracks: [
      track({
        id: 'demo-track-03-01',
        albumId: 'demo-album-03',
        discIndex: 1,
        number: 1,
        title: 'East Bank',
        artistName: 'Alex Reed',
        durationMs: 221_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-03-02',
        albumId: 'demo-album-03',
        discIndex: 1,
        number: 2,
        title: 'Bridge Freeze',
        artistName: 'Alex Reed',
        durationMs: 199_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-03-03',
        albumId: 'demo-album-03',
        discIndex: 1,
        number: 3,
        title: 'Skyway',
        artistName: 'Alex Reed',
        durationMs: 253_000,
        flag: 'ok',
        lyricsKind: 'plain',
      }),
    ],
  };
}

function glassHour(): DemoAlbum {
  return {
    id: 'demo-album-04',
    title: 'Glass Hour',
    artistName: 'Alex Reed',
    artistKey: 'alex-reed-south',
    year: 2020,
    coverTone: '04',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    tracks: [
      track({
        id: 'demo-track-04-01',
        albumId: 'demo-album-04',
        discIndex: 1,
        number: 1,
        title: 'Pane',
        artistName: 'Alex Reed',
        durationMs: 188_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-04-02',
        albumId: 'demo-album-04',
        discIndex: 1,
        number: 2,
        title: 'Second Hand',
        artistName: 'Alex Reed',
        durationMs: 210_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-04-03',
        albumId: 'demo-album-04',
        discIndex: 1,
        number: 3,
        title: 'Quiet Lattice',
        artistName: 'Alex Reed',
        durationMs: 267_000,
        flag: 'ok',
        lyricsKind: 'synced',
      }),
    ],
  };
}

function stages(): DemoAlbum {
  return {
    id: 'demo-album-05',
    title: 'Stages',
    artistName: 'The Compound',
    artistKey: 'the-compound',
    year: 2018,
    coverTone: '05',
    discs: [
      { index: 1, title: 'Act One' },
      { index: 2, title: 'Act Two' },
    ],
    hostile: false,
    tracks: [
      track({
        id: 'demo-track-05-01',
        albumId: 'demo-album-05',
        discIndex: 1,
        number: 1,
        title: 'Curtain',
        artistName: 'The Compound',
        durationMs: 145_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-05-02',
        albumId: 'demo-album-05',
        discIndex: 1,
        number: 2,
        title: 'Understudy',
        artistName: 'The Compound',
        durationMs: 203_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-05-03',
        albumId: 'demo-album-05',
        discIndex: 2,
        number: 1,
        title: 'Intermission Tone',
        artistName: 'The Compound',
        durationMs: 92_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-05-04',
        albumId: 'demo-album-05',
        discIndex: 2,
        number: 2,
        title: 'Encore Wire',
        artistName: 'The Compound',
        durationMs: 248_000,
        flag: 'ok',
        lyricsKind: 'plain',
      }),
    ],
  };
}

function radioAtlas(): DemoAlbum {
  return {
    id: 'demo-album-06',
    title: 'Radio Atlas',
    artistName: 'Various Artists',
    artistKey: 'various-artists',
    year: 2022,
    coverTone: '06',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    tracks: [
      track({
        id: 'demo-track-06-01',
        albumId: 'demo-album-06',
        discIndex: 1,
        number: 1,
        title: 'Shortwave Map',
        artistName: 'Ivy North',
        durationMs: 193_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-06-02',
        albumId: 'demo-album-06',
        discIndex: 1,
        number: 2,
        title: 'Call Sign',
        artistName: 'Jun Park',
        durationMs: 211_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-06-03',
        albumId: 'demo-album-06',
        discIndex: 1,
        number: 3,
        title: 'Skip Zone',
        artistName: 'Mira Sol',
        durationMs: 184_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-06-04',
        albumId: 'demo-album-06',
        discIndex: 1,
        number: 4,
        title: 'Carrier',
        artistName: 'Keratin',
        durationMs: 226_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
    ],
  };
}

function signalLoss(): DemoAlbum {
  return {
    id: 'demo-album-07',
    title: 'Signal Loss',
    artistName: 'Keratin',
    artistKey: 'keratin',
    year: 2024,
    coverTone: '07',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    tracks: [
      track({
        id: 'demo-track-07-01',
        albumId: 'demo-album-07',
        discIndex: 1,
        number: 1,
        title: 'Handshake',
        artistName: 'Keratin',
        durationMs: 172_000,
        flag: 'ok',
        lyricsKind: 'synced',
      }),
      track({
        id: 'demo-track-07-02',
        albumId: 'demo-album-07',
        discIndex: 1,
        number: 2,
        title: 'Codec Mirage',
        artistName: 'Keratin',
        durationMs: 201_000,
        flag: 'unplayable',
        lyricsKind: 'none',
      }),
      track({
        id: 'demo-track-07-03',
        albumId: 'demo-album-07',
        discIndex: 1,
        number: 3,
        title: 'Bitrot Lullaby',
        artistName: 'Keratin',
        durationMs: 239_000,
        flag: 'damaged',
        lyricsKind: 'plain',
      }),
      track({
        id: 'demo-track-07-04',
        albumId: 'demo-album-07',
        discIndex: 1,
        number: 4,
        title: 'Reconnect',
        artistName: 'Keratin',
        durationMs: 215_000,
        flag: 'ok',
        lyricsKind: 'none',
      }),
    ],
  };
}

function hostileAlbum(): DemoAlbum {
  const payload = hostileCorpus();
  return {
    id: 'demo-album-08',
    title: payload,
    artistName: payload,
    artistKey: 'hostile-artist',
    year: 2016,
    coverTone: '08',
    discs: [{ index: 1, title: payload }],
    hostile: true,
    tracks: [
      track({
        id: 'demo-track-08-01',
        albumId: 'demo-album-08',
        discIndex: 1,
        number: 1,
        title: payload,
        artistName: payload,
        durationMs: 120_000,
        flag: 'ok',
        lyricsKind: 'plain',
      }),
      track({
        id: 'demo-track-08-02',
        albumId: 'demo-album-08',
        discIndex: 1,
        number: 2,
        title: payload,
        artistName: payload,
        durationMs: 121_000,
        flag: 'ok',
        lyricsKind: 'synced',
      }),
    ],
  };
}

function albumTable(): readonly DemoAlbum[] {
  return [
    harbourLights(),
    nightShift(),
    twinCities(),
    glassHour(),
    stages(),
    radioAtlas(),
    signalLoss(),
    hostileAlbum(),
    cylindersAlbum(),
  ];
}

function artistsFrom(albums: readonly DemoAlbum[]): readonly DemoArtist[] {
  const byKey = new Map<string, { name: string; albumIds: string[] }>();
  for (const album of albums) {
    const existing = byKey.get(album.artistKey);
    if (existing === undefined) {
      byKey.set(album.artistKey, { name: album.artistName, albumIds: [album.id] });
    } else {
      existing.albumIds.push(album.id);
    }
  }
  return [...byKey.entries()]
    .sort((left, right) => left[0].localeCompare(right[0]))
    .map(([key, row]) => ({ key, name: row.name, albumIds: row.albumIds }));
}

/** Stage-A demo library: hand-written fixture albums (CP-011). */
export function demoLibrary(): DemoLibrary {
  const albums = albumTable();
  return {
    kind: 'demo-fixtures',
    albums,
    artists: artistsFrom(albums),
  };
}

export function albumById(library: DemoLibrary, id: string): DemoAlbum | undefined {
  return library.albums.find((album) => album.id === id);
}

export function trackById(library: DemoLibrary, id: string): DemoTrack | undefined {
  for (const album of library.albums) {
    const found = album.tracks.find((entry) => entry.id === id);
    if (found !== undefined) {
      return found;
    }
  }
  return undefined;
}

export function allTracks(library: DemoLibrary): readonly DemoTrack[] {
  const rows: DemoTrack[] = [];
  for (const album of library.albums) {
    for (const entry of album.tracks) {
      rows.push(entry);
    }
  }
  return rows;
}
