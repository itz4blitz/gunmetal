import { expect, test } from 'vitest';
import { nextCount, shellProps, tokenClass } from './compose.ts';

test('the composition root names the shell and the count', () => {
  expect(shellProps(0)).toStrictEqual({ label: 'Gunmetal', count: 0 });
  expect(shellProps(4)).toStrictEqual({ label: 'Gunmetal', count: 4 });
});

test('the runtime value advances by one', () => {
  expect(nextCount(0)).toStrictEqual(1);
  expect(nextCount(7)).toStrictEqual(8);
});

test('the token colour is applied through the token class', () => {
  expect(tokenClass()).toStrictEqual('token-text');
});
