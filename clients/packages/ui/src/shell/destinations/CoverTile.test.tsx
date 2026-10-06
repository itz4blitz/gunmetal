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

test('cover tile layers wash sheen and an uppercase glyph for each size', () => {
  const grid = render(<CoverTile tone="03" label="signal loss" size="grid" />);
  const root = grid.container.querySelector('[data-cover="03"][data-size="grid"]');
  expect(root).toBeTruthy();
  expect(root?.querySelector('[data-cover-wash="1"]')).toBeTruthy();
  expect(root?.querySelector('[data-cover-sheen="1"]')).toBeTruthy();
  expect(root?.querySelector('[data-cover-label]')?.textContent).toStrictEqual('S');
  grid.unmount();

  const detail = render(<CoverTile tone="08" label="a" size="detail" coverId="cover-detail-x" />);
  expect(detail.container.querySelector('#cover-detail-x')?.getAttribute('data-size')).toStrictEqual(
    'detail',
  );
  expect(detail.container.querySelector('[data-cover-label]')?.textContent).toStrictEqual('A');
  detail.unmount();

  const row = render(<CoverTile tone="05" label="Stages" size="row" />);
  expect(row.container.querySelector('[data-size="row"] [data-cover-label]')?.textContent).toStrictEqual(
    'S',
  );
  row.unmount();

  const full = render(<CoverTile tone="01" label="Pier" size="full" coverId="cover-full-x" />);
  expect(full.container.querySelector('#cover-full-x')?.getAttribute('data-size')).toStrictEqual(
    'full',
  );
  expect(full.container.querySelector('[data-cover-label]')?.textContent).toStrictEqual('P');
});
