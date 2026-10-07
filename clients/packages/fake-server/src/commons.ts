import type { FixtureAlbum, FixtureTrack } from './types.ts';

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
