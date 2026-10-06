import { cleanup, render } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { CoverTile } from './CoverTile.tsx';

afterEach(cleanup);

test('cover tile uses a midpoint glyph when the label is blank', () => {
  const view = render(<CoverTile tone="01" label="   " size="bar" />);
  expect(view.container.querySelector('[data-cover-label]')?.textContent).toStrictEqual('·');
  view.unmount();
  const named = render(
    <CoverTile tone="02" label="Harbour" size="grid" coverId="cover-grid-demo" />,
  );
  expect(named.container.querySelector('#cover-grid-demo')).toBeTruthy();
  expect(named.container.querySelector('[data-cover-label]')?.textContent).toStrictEqual('H');
});
