import { expect, test } from 'vitest';
import {
  demoFallbackVerse,
  demoLyricsLines,
  demoLyricsTable,
  demoLyricsVerse,
  noLyricsLine,
} from './lyrics.ts';

test('lyrics fixtures are plain strings with a none-kind placeholder', () => {
  expect(noLyricsLine()).toStrictEqual('This file has no lyrics.');
  expect(demoFallbackVerse()).toStrictEqual(
    'A short demo verse\nlives in this fixture\nno markup, only lines',
  );
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
