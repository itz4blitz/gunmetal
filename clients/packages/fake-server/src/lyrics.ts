import type { DemoLyricsKind } from './types.ts';

export function noLyricsLine(): string {
  return 'This file has no lyrics.';
}

export function demoFallbackVerse(): string {
  return 'A short demo verse\nlives in this fixture\nno markup, only lines';
}

/** Fixture verses keyed by track id. Returned by a function so tests own the table. */
export function demoLyricsTable(): Readonly<Record<string, string>> {
  return {
    'demo-track-01-03': 'The harbour keeps the letter\nfolded under glass\nuntil the tide comes back',
    'demo-track-02-02': 'Floors count themselves in the dark\nsteel doors, a held breath\nthen the motor starts',
    'demo-track-03-03': 'The river writes the same name\ntwice, once for each bank\nand the bridge forgets',
    'demo-track-04-03': 'Glass holds the hour still\na lattice of quiet\ncounting the light',
    'demo-track-05-04': 'The wire remembers the bow\none last bright measure\nthen the house lights rise',
    'demo-track-07-01': 'Hello, hello through the static\nhandshake in the noise\nhold the line',
    'demo-track-07-03': 'A lullaby for broken bits\nsleep in the checksum\nwake on retry',
    'demo-track-08-01': 'A short demo verse\nlives in this fixture\nno markup, only lines',
    'demo-track-08-02': 'First line stays lit\nthe rest wait their turn\nno clock in the demo',
  };
}

export function demoLyricsVerse(trackId: string, kind: DemoLyricsKind): string {
  if (kind === 'none') {
    return noLyricsLine();
  }
  const table = demoLyricsTable();
  return table[trackId] ?? demoFallbackVerse();
}

export function demoLyricsLines(verse: string): readonly string[] {
  return verse.split('\n');
}
