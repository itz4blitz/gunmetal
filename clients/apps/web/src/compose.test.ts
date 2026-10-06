import { expect, test } from 'vitest';
import { clockLabel, nextCount, shellProps, tokenClass } from './compose.ts';

test('the composition root names the empty player', () => {
  expect(shellProps(0)).toStrictEqual({
    wordmark: 'Gunmetal',
    title: 'Nothing is playing',
    hint: 'The web client is running. Library and playback are not wired yet.',
    clock: '0:00',
  });
  expect(shellProps(83)).toStrictEqual({
    wordmark: 'Gunmetal',
    title: 'Nothing is playing',
    hint: 'The web client is running. Library and playback are not wired yet.',
    clock: '1:23',
  });
});

test('the clock pads a single-digit second and does not pad ten or more', () => {
  expect(clockLabel(0)).toStrictEqual('0:00');
  expect(clockLabel(9)).toStrictEqual('0:09');
  expect(clockLabel(10)).toStrictEqual('0:10');
  expect(clockLabel(60)).toStrictEqual('1:00');
});

test('the runtime value advances by one', () => {
  expect(nextCount(0)).toStrictEqual(1);
  expect(nextCount(7)).toStrictEqual(8);
});

test('the token colour is applied through the token class', () => {
  expect(tokenClass()).toStrictEqual('token-text');
});
