import { expect, test } from 'vitest';
import { currentLineAt, FOLLOW_IDLE_MS, type SyncedLine } from './synced-lyrics.ts';

const VERSE: readonly SyncedLine[] = [
  { atMs: 0, text: 'First line stays lit' },
  { atMs: 4_000, text: 'the second arrives' },
  { atMs: 9_500, text: 'a late third' },
];

test('the line clock lights the line that is sounding, or none before the first', () => {
  expect(currentLineAt(VERSE, 0)).toStrictEqual(0);
  expect(currentLineAt(VERSE, 3_999)).toStrictEqual(0);
  expect(currentLineAt(VERSE, 4_000)).toStrictEqual(1);
  expect(currentLineAt(VERSE, 9_500)).toStrictEqual(2);
  expect(currentLineAt(VERSE, 600_000)).toStrictEqual(2);
});

test('before the first timestamp nothing is lit yet', () => {
  const late: readonly SyncedLine[] = [{ atMs: 2_000, text: 'starts late' }];
  expect(currentLineAt(late, 0)).toStrictEqual(-1);
  expect(currentLineAt(late, 1_999)).toStrictEqual(-1);
  expect(currentLineAt(late, 2_000)).toStrictEqual(0);
});

test('an empty or unsorted sheet lights nothing and never lies', () => {
  expect(currentLineAt([], 4_000)).toStrictEqual(-1);
  const unsorted: readonly SyncedLine[] = [
    { atMs: 9_500, text: 'a late third' },
    { atMs: 0, text: 'First line stays lit' },
  ];
  expect(currentLineAt(unsorted, 1_000)).toStrictEqual(1);
  expect(currentLineAt(unsorted, 10_000)).toStrictEqual(0);
});

test('the follow idle delay is a stated constant the pane can be tested against', () => {
  expect(FOLLOW_IDLE_MS).toStrictEqual(4_000);
});
