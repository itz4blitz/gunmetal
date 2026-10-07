import { cleanup, render } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { BrandMark } from './brand-mark.tsx';

afterEach(cleanup);

test('the brand mark draws the nut with its brass bore play as decorative svg', () => {
  const { container } = render(<BrandMark size={22} />);
  const svg = container.querySelector('svg[data-brand-mark="1"]');
  expect(svg).not.toBeNull();
  expect(svg?.getAttribute('viewBox')).toStrictEqual('0 0 24 24');
  expect(svg?.getAttribute('aria-hidden')).toStrictEqual('true');
  // The nut face and the brass play are separate painted shapes.
  expect((svg as HTMLElement).querySelector('[data-nut-face]')).not.toBeNull();
  expect((svg as HTMLElement).querySelector('[data-nut-play]')).not.toBeNull();
});

test('the mark scales with its size prop', () => {
  const { container } = render(<BrandMark size={30} />);
  const svg = container.querySelector('svg[data-brand-mark="1"]') as HTMLElement;
  expect(svg.getAttribute('width')).toStrictEqual('30');
  expect(svg.getAttribute('height')).toStrictEqual('30');
});
