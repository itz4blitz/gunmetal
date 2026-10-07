import type { DemoLyricsKind, DemoTrackFlag, FixtureAlbum, FixtureTrack } from './types.ts';

function cylindersTrack(number: number, title: string, durationMs: number): FixtureTrack {
  return {
    id: `demo-track-09-0${number}`,
    albumId: 'demo-album-09',
    discIndex: 1,
    number,
    title,
    artistName: 'Chris Zabriskie',
    durationMs,
    flag: 'ok',
    lyricsKind: 'none',
  };
}

/** Real CC BY 4.0 album; lengths from chriszabriskie.com/cylinders (2014). */
export function cylindersAlbum(): FixtureAlbum {
  return {
    id: 'demo-album-09',
    title: 'Cylinders',
    artistName: 'Chris Zabriskie',
    artistKey: 'chris-zabriskie',
    year: 2014,
    coverTone: '03',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    license: {
      spdx: 'CC-BY-4.0',
      attribution: 'Cylinders by Chris Zabriskie',
      source: 'chriszabriskie.com',
    },
    tracks: [
      cylindersTrack(1, 'Cylinder One', 176_000),
      cylindersTrack(2, 'Cylinder Two', 228_000),
      cylindersTrack(3, 'Cylinder Three', 166_000),
      cylindersTrack(4, 'Cylinder Four', 177_000),
      cylindersTrack(5, 'Cylinder Five', 174_000),
      cylindersTrack(6, 'Cylinder Six', 106_000),
      cylindersTrack(7, 'Cylinder Seven', 532_000),
      cylindersTrack(8, 'Cylinder Eight', 339_000),
      cylindersTrack(9, 'Cylinder Nine', 323_000),
    ],
  };
}

/**
 * Real Creative Commons albums (demo-album-10 and up). Track titles and
 * durations are the artists' published ones; the audio stays a generated
 * tone (the demo never ships or fetches real media). Licences are recorded
 * per album; where the licence could not be pinned to a version, the
 * conservative artist-site/FMA source is cited instead.
 *
 * Deliberately NOT included: Bio Unit and Chad Crouch FMA copies checked
 * out CC BY-NC / BY-NC-SA / BY-NC-ND (NonCommercial variants are not in the
 * demo's licence union), and "Ring My Bell (Kai Engel Piano Edit)" (a
 * commercial Ann Lee release, not CC).
 */

function realTrack(
  albumNumber: number,
  discIndex: number,
  number: number,
  title: string,
  durationMs: number,
  flag: DemoTrackFlag = 'ok',
  lyricsKind: DemoLyricsKind = 'none',
): FixtureTrack {
  const padded = String(albumNumber).padStart(2, '0');
  return {
    id: `demo-track-${padded}-${String(number).padStart(2, '0')}`,
    albumId: `demo-album-${padded}`,
    discIndex,
    number,
    title,
    artistName: '',
    durationMs,
    flag,
    lyricsKind,
  };
}

function withArtist(tracks: readonly FixtureTrack[], artistName: string): FixtureTrack[] {
  return tracks.map((entry) => ({ ...entry, artistName }));
}

/** Real CC BY 4.0 album; lengths from chriszabriskie.com/divider (2011). */
export function dividerAlbum(): FixtureAlbum {
  return {
    id: 'demo-album-10',
    title: 'Divider',
    artistName: 'Chris Zabriskie',
    artistKey: 'chris-zabriskie',
    year: 2011,
    coverTone: '03',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    license: {
      spdx: 'CC-BY-4.0',
      attribution: 'Divider by Chris Zabriskie',
      source: 'chriszabriskie.com',
    },
    tracks: withArtist(
      [
        realTrack(10, 1, 1, 'Heliograph', 339_000, 'ok', 'plain'),
        realTrack(10, 1, 2, 'Candlepower', 340_000),
        realTrack(10, 1, 3, 'CGI Snake', 390_000),
        realTrack(10, 1, 4, 'Wonder Cycle', 345_000),
        realTrack(10, 1, 5, 'Oxygen Garden', 363_000, 'ok', 'plain'),
        realTrack(10, 1, 6, 'Divider', 201_000),
      ],
      'Chris Zabriskie',
    ),
  };
}

/**
 * Real Kai Engel album (2017). Kai Engel's site licenses his pre-2025
 * catalogue under CC BY 4.0; lengths from the official album release.
 */
export function sustainsAlbum(): FixtureAlbum {
  return {
    id: 'demo-album-11',
    title: 'Sustains',
    artistName: 'Kai Engel',
    artistKey: 'kai-engel',
    year: 2017,
    coverTone: '05',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    license: {
      spdx: 'CC-BY-4.0',
      attribution: 'Sustains by Kai Engel',
      source: 'kai-engel.com',
    },
    tracks: withArtist(
      [
        realTrack(11, 1, 1, 'Brand New World', 169_000),
        realTrack(11, 1, 2, 'Meekness', 163_000),
        realTrack(11, 1, 3, 'Holiday Gift', 195_000),
        realTrack(11, 1, 4, 'Headway', 148_000),
        realTrack(11, 1, 5, 'Slum Canto', 209_000),
        realTrack(11, 1, 6, 'Evermore', 175_000),
        realTrack(11, 1, 7, 'Cold War Echo', 296_000, 'ok', 'plain'),
        realTrack(11, 1, 8, 'Global Warming', 188_000),
        realTrack(11, 1, 9, 'Plague', 169_000),
        realTrack(11, 1, 10, 'Mercy', 182_000),
      ],
      'Kai Engel',
    ),
  };
}

/**
 * Real Scott Buckley library release presented as the two-volume set the
 * artist publishes: the discs are the real albums "Library Songs 8" (2022)
 * and "Library Songs 9" (2023), bundled under their real series name to
 * exercise the multi-disc UI. scottbuckley.com.au licenses the whole library
 * CC BY 4.0; lengths from the Bandcamp releases.
 */
export function librarySongsAlbum(): FixtureAlbum {
  return {
    id: 'demo-album-12',
    title: 'Library Songs',
    artistName: 'Scott Buckley',
    artistKey: 'scott-buckley',
    year: 2022,
    coverTone: '01',
    discs: [
      { index: 1, title: 'Library Songs 8' },
      { index: 2, title: 'Library Songs 9' },
    ],
    hostile: false,
    license: {
      spdx: 'CC-BY-4.0',
      attribution: 'Library Songs 8 and Library Songs 9 by Scott Buckley',
      source: 'scottbuckley.com.au',
    },
    tracks: withArtist(
      [
        realTrack(12, 1, 1, 'Age Of Wonder', 350_000, 'ok', 'synced'),
        realTrack(12, 1, 2, 'Bring Me The Sky', 232_000),
        realTrack(12, 1, 3, 'Artemis', 375_000),
        realTrack(12, 1, 4, 'Into The Unknown', 261_000),
        realTrack(12, 1, 5, 'Where Stars Fall', 270_000),
        realTrack(12, 1, 6, 'In Dreams', 248_000),
        realTrack(12, 1, 7, 'Legacy', 389_000),
        realTrack(12, 1, 8, 'Race The Sun', 291_000),
        realTrack(12, 1, 9, 'The Climb', 347_000),
        realTrack(12, 1, 10, 'Wayfarer', 222_000),
        realTrack(12, 1, 11, 'Terminus', 263_000),
        realTrack(12, 1, 12, 'The Miracle Of Flight', 211_000),
        realTrack(12, 1, 13, 'Clarion', 195_000),
        realTrack(12, 2, 14, 'Last and First Light', 468_000),
        realTrack(12, 2, 15, 'Permafrost', 448_000),
        realTrack(12, 2, 16, 'A Kind of Hope', 342_000),
        realTrack(12, 2, 17, 'Decoherence', 618_000),
        realTrack(12, 2, 18, 'Hymn to the Dawn', 418_000),
        realTrack(12, 2, 19, 'Adrift Among Infinite Stars', 405_000),
        realTrack(12, 2, 20, 'Cirrus', 418_000),
        realTrack(12, 2, 21, 'In Search of Solitude', 445_000),
        realTrack(12, 2, 22, 'Aurora', 498_000),
      ],
      'Scott Buckley',
    ),
  };
}

/**
 * Real incompetech collection (2014), mirrored on the Free Music Archive,
 * which licenses it Attribution 3.0 International; lengths from the FMA
 * album page. "The North" carries the new-album unplayable flag.
 */
export function thatchedVillagersAlbum(): FixtureAlbum {
  return {
    id: 'demo-album-13',
    title: 'Thatched Villagers',
    artistName: 'Kevin MacLeod',
    artistKey: 'kevin-macleod',
    year: 2014,
    coverTone: '07',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    license: {
      spdx: 'CC-BY-3.0',
      attribution: 'Thatched Villagers by Kevin MacLeod',
      source: 'freemusicarchive.org',
    },
    tracks: withArtist(
      [
        realTrack(13, 1, 1, "Miri's Magic Dance", 94_000),
        realTrack(13, 1, 2, 'Nu Flute', 82_000),
        realTrack(13, 1, 3, 'Cattails', 159_000),
        realTrack(13, 1, 4, 'Angevin 70', 280_000),
        realTrack(13, 1, 5, 'Baltic Levity', 48_000),
        realTrack(13, 1, 6, 'Padanaya Blokov', 127_000),
        realTrack(13, 1, 7, 'The North', 20_000, 'unplayable'),
        realTrack(13, 1, 8, 'Unanswered Questions', 165_000),
        realTrack(13, 1, 9, 'Pippin the Hunchback', 193_000),
        realTrack(13, 1, 10, 'Ritual', 264_000),
        realTrack(13, 1, 11, 'Bushwick Tarentella', 203_000),
        realTrack(13, 1, 12, 'Temple of the Manes', 155_000),
        realTrack(13, 1, 13, 'Living Voyage', 208_000),
        realTrack(13, 1, 14, 'For Originz', 218_000),
        realTrack(13, 1, 15, 'Thatched Villagers', 245_000, 'ok', 'plain'),
        realTrack(13, 1, 16, 'Rites', 126_000),
      ],
      'Kevin MacLeod',
    ),
  };
}

/**
 * Real incompetech collection (2014), mirrored on the Free Music Archive
 * under Attribution 3.0 International; lengths from the FMA album page.
 * "Impact intermezzo" carries the new-album damaged flag.
 */
export function impactAlbum(): FixtureAlbum {
  return {
    id: 'demo-album-14',
    title: 'Impact',
    artistName: 'Kevin MacLeod',
    artistKey: 'kevin-macleod',
    year: 2014,
    coverTone: '02',
    discs: [{ index: 1, title: '' }],
    hostile: false,
    license: {
      spdx: 'CC-BY-3.0',
      attribution: 'Impact by Kevin MacLeod',
      source: 'freemusicarchive.org',
    },
    tracks: withArtist(
      [
        realTrack(14, 1, 1, 'Impact Prelude', 202_000, 'ok', 'plain'),
        realTrack(14, 1, 2, 'Impact Andante', 168_000),
        realTrack(14, 1, 3, 'Impact Moderato', 75_000),
        realTrack(14, 1, 4, 'Impact intermezzo', 64_000, 'damaged'),
        realTrack(14, 1, 5, 'Impact Allegretto', 182_000),
        realTrack(14, 1, 6, 'Impact Lento', 210_000),
      ],
      'Kevin MacLeod',
    ),
  };
}

/**
 * Real Kai Engel "Chapter" seasons project presented as the two volumes the
 * artist released first: the discs are the real albums "Chapter One / Cold"
 * (2015) and "Chapter Two / Mild" (2016). Same CC BY 4.0 basis and length
 * sourcing as Sustains; the bundle exists to exercise the multi-disc UI.
 */
export function chapterAlbum(): FixtureAlbum {
  return {
    id: 'demo-album-15',
    title: 'Chapter',
    artistName: 'Kai Engel',
    artistKey: 'kai-engel',
    year: 2016,
    coverTone: '04',
    discs: [
      { index: 1, title: 'Chapter One / Cold' },
      { index: 2, title: 'Chapter Two / Mild' },
    ],
    hostile: false,
    license: {
      spdx: 'CC-BY-4.0',
      attribution: 'Chapter One / Cold and Chapter Two / Mild by Kai Engel',
      source: 'kai-engel.com',
    },
    tracks: withArtist(
      [
        realTrack(15, 1, 1, 'Snowfall (Intro)', 441_000),
        realTrack(15, 1, 2, 'December', 229_000, 'ok', 'synced'),
        realTrack(15, 1, 3, 'Blizzard (PON I)', 182_000),
        realTrack(15, 1, 4, 'Snowmen', 103_000),
        realTrack(15, 1, 5, 'January', 155_000),
        realTrack(15, 1, 6, 'Morbid Imagination', 190_000),
        realTrack(15, 1, 7, 'February', 135_000),
        realTrack(15, 1, 8, 'Thaw (Outro)', 117_000),
        realTrack(15, 1, 9, 'Nothing (Bonus Track)', 292_000),
        realTrack(15, 2, 10, 'Floret', 282_000),
        realTrack(15, 2, 11, 'March', 145_000),
        realTrack(15, 2, 12, 'Brooks', 128_000),
        realTrack(15, 2, 13, 'River', 174_000),
        realTrack(15, 2, 14, 'April', 145_000),
        realTrack(15, 2, 15, 'Daylight (PON II)', 197_000),
        realTrack(15, 2, 16, 'May', 132_000),
        realTrack(15, 2, 17, 'The Blossom (PON III)', 168_000),
        realTrack(15, 2, 18, 'Something (Bonus Track)', 271_000, 'ok', 'plain'),
      ],
      'Kai Engel',
    ),
  };
}
