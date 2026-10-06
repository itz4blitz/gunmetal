import { expect, test } from 'vitest';
import { artistInitial, formatDuration, staggerSlot } from './format.ts';

test('artistInitial uses the first visible character or a question mark', () => {
  expect(artistInitial('Keratin')).toStrictEqual('K');
  expect(artistInitial('mira sol')).toStrictEqual('M');
  expect(artistInitial('   ')).toStrictEqual('?');
  expect(artistInitial('')).toStrictEqual('?');
});

test('staggerSlot steps 20ms rows and caps at the sixth slot', () => {
  expect(staggerSlot(-4)).toStrictEqual('0');
  expect(staggerSlot(0)).toStrictEqual('0');
  expect(staggerSlot(3)).toStrictEqual('3');
  expect(staggerSlot(6)).toStrictEqual('6');
  expect(staggerSlot(12)).toStrictEqual('6');
});

test('formatDuration renders mm:ss for finite non-negative milliseconds', () => {
  expect(formatDuration(0)).toStrictEqual('0:00');
  expect(formatDuration(999)).toStrictEqual('0:00');
  expect(formatDuration(1000)).toStrictEqual('0:01');
  expect(formatDuration(65_000)).toStrictEqual('1:05');
  expect(formatDuration(600_000)).toStrictEqual('10:00');
  expect(formatDuration(-1)).toStrictEqual('0:00');
  expect(formatDuration(Number.NaN)).toStrictEqual('0:00');
});
