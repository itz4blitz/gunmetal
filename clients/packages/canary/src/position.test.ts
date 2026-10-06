import { expect, test } from 'vitest';
import { position } from './position.ts';

test('a position counts from one and names the total', () => {
  expect(position(0, 12)).toStrictEqual('1 of 12');
  expect(position(11, 12)).toStrictEqual('12 of 12');
});
