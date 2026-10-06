import { expect, test } from 'vitest';
import { kinds } from './kinds.ts';

test('the canary lists the four kinds of code and which of them are mutated', () => {
  expect(kinds()).toStrictEqual([
    { name: 'pure function', mutated: true },
    { name: 'component', mutated: true },
    { name: 'data table', mutated: true },
    { name: 'browser adapter', mutated: false },
  ]);
});
