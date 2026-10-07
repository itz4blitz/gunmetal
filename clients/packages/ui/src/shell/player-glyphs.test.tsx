import { cleanup, render } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { RepeatGlyph } from './player-glyphs.tsx';

afterEach(cleanup);

test('the repeat glyph draws one stroked path in the shell icon grammar', () => {
  const { container } = render(<RepeatGlyph />);
  const svg = container.querySelector('svg');
  expect(svg?.getAttribute('data-icon')).toStrictEqual('repeat');
  expect(svg?.getAttribute('viewBox')).toStrictEqual('0 0 24 24');
  expect(svg?.getAttribute('stroke')).toStrictEqual('currentColor');
  expect(svg?.getAttribute('stroke-width')).toStrictEqual('1.8');
  expect(svg?.getAttribute('aria-hidden')).toStrictEqual('true');
  expect(container.querySelectorAll('path')).toHaveLength(1);
});

test('repeat one adds the mark that names the track itself', () => {
  const { container } = render(<RepeatGlyph one size={18} />);
  const svg = container.querySelector('svg');
  expect(svg?.getAttribute('data-icon')).toStrictEqual('repeat-one');
  expect(svg?.getAttribute('width')).toStrictEqual('18');
  expect(container.querySelectorAll('path')).toHaveLength(2);
});
