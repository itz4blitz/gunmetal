import { expect, test } from 'vitest';
import { screens } from './screens.ts';

test('the sweep registry lists the shell and the unsupported-browser page', () => {
  expect(screens()).toStrictEqual([
    { name: 'shell', path: '/' },
    { name: 'unsupported-browser', path: '/unsupported.html' },
  ]);
});
