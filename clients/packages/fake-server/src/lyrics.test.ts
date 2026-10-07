import { expect, test } from 'vitest';
import { demoLibrary } from './catalogue.ts';
import { demoFallbackVerse, demoLyricsLines, demoLyricsTable, demoLyricsVerse, noLyricsLine } from './lyrics.ts';

test('lyrics fixtures are plain strings with a none-kind placeholder', () => {
  expect(noLyricsLine()).toStrictEqual('This file has no lyrics.');
  expect(demoFallbackVerse()).toStrictEqual('A short demo verse\nlives in this fixture\nno markup, only lines');
  expect(demoLyricsTable()).toStrictEqual({
    'demo-track-01-03': 'The harbour keeps the letter\nfolded under glass\nuntil the tide comes back',
    'demo-track-02-02': 'Floors count themselves in the dark\nsteel doors, a held breath\nthen the motor starts',
    'demo-track-03-03': 'The river writes the same name\ntwice, once for each bank\nand the bridge forgets',
    'demo-track-04-03': 'Glass holds the hour still\na lattice of quiet\ncounting the light',
    'demo-track-05-04': 'The wire remembers the bow\none last bright measure\nthen the house lights rise',
    'demo-track-07-01': 'Hello, hello through the static\nhandshake in the noise\nhold the line',
    'demo-track-07-03': 'A lullaby for broken bits\nsleep in the checksum\nwake on retry',
    'demo-track-08-01': 'A short demo verse\nlives in this fixture\nno markup, only lines',
    'demo-track-08-02': 'First line stays lit\nthe rest wait their turn\nno clock in the demo',
    'demo-track-10-01': 'A mirror to the sun\none flash across the bay\nand the ships reply',
    'demo-track-10-05': 'Green glass over the beds\nthe leaves breathe out\nwhat the lamps breathe in',
    'demo-track-11-07': 'Two towers hold their breath\none voice comes back\nyears too late',
    'demo-track-12-01': 'First light finds the map\nwe name the stars we know\nand hum the rest',
    'demo-track-13-15': 'Thatch holds the warm air\nlanterns learn the wind\nthe village hums low',
    'demo-track-14-01': 'Count the quiet in\none note leans on the next\nthe hall leans back',
    'demo-track-15-02': 'First line stays lit\nwindows count the snow\nthe year turns slow',
    'demo-track-15-18': 'One more than the nine\nkept for the road home\na light left on',
  });
  expect(demoLyricsVerse('demo-track-01-01', 'none')).toStrictEqual('This file has no lyrics.');
  expect(demoLyricsVerse('demo-track-01-03', 'plain')).toStrictEqual(
    'The harbour keeps the letter\nfolded under glass\nuntil the tide comes back',
  );
  expect(demoLyricsVerse('demo-track-02-02', 'synced')).toStrictEqual(
    'Floors count themselves in the dark\nsteel doors, a held breath\nthen the motor starts',
  );
  expect(demoLyricsVerse('missing-track', 'plain')).toStrictEqual(
    'A short demo verse\nlives in this fixture\nno markup, only lines',
  );
  expect(demoLyricsLines(demoLyricsVerse('demo-track-08-02', 'synced'))).toStrictEqual([
    'First line stays lit',
    'the rest wait their turn',
    'no clock in the demo',
  ]);
  expect(demoLyricsLines(noLyricsLine())).toStrictEqual(['This file has no lyrics.']);
  expect(demoLyricsVerse('demo-track-01-03', 'none')).toStrictEqual('This file has no lyrics.');
});

test('the new real-album verses are plain except two synced fixtures', () => {
  const library = demoLibrary();
  const newTracks = library.albums.filter((album) => album.id >= 'demo-album-10').flatMap((album) => album.tracks);
  const withLyrics = newTracks.filter((track) => track.lyricsKind !== 'none');
  expect(withLyrics.map((track) => [track.id, track.lyricsKind])).toStrictEqual([
    ['demo-track-10-01', 'plain'],
    ['demo-track-10-05', 'plain'],
    ['demo-track-11-07', 'plain'],
    ['demo-track-12-01', 'synced'],
    ['demo-track-13-15', 'plain'],
    ['demo-track-14-01', 'plain'],
    ['demo-track-15-02', 'synced'],
    ['demo-track-15-18', 'plain'],
  ]);
  for (const track of withLyrics) {
    expect(demoLyricsTable()[track.id]).toBeDefined();
    expect(demoLyricsVerse(track.id, track.lyricsKind).split('\n').length).toBeGreaterThan(1);
  }
});
