import { expect, test } from 'vitest';
import { formatDuration } from './format.ts';

test('formatDuration renders mm:ss for finite non-negative milliseconds', () => {
  expect(formatDuration(0)).toStrictEqual('0:00');
  expect(formatDuration(999)).toStrictEqual('0:00');
  expect(formatDuration(1000)).toStrictEqual('0:01');
  expect(formatDuration(65_000)).toStrictEqual('1:05');
  expect(formatDuration(600_000)).toStrictEqual('10:00');
  expect(formatDuration(-1)).toStrictEqual('0:00');
  expect(formatDuration(Number.NaN)).toStrictEqual('0:00');
});
