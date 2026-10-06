import { expect, test } from 'vitest';
import { composeDemo } from './compose.ts';

test('the demo composition root always labels demo data', () => {
  expect(composeDemo()).toStrictEqual({ showDemoLabel: true });
});
