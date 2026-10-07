import { cleanup, render } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { CoverTile } from './CoverTile.tsx';

afterEach(cleanup);

test('small covers keep a midpoint glyph when the label is blank', () => {
  const view = render(<CoverTile tone="01" label="   " size="bar" />);
  expect(view.container.querySelector('[data-cover-label]')?.textContent).toStrictEqual('·');
});

test('grid and row covers are quiet plates: no letter, no sheen layer', () => {
  const grid = render(<CoverTile tone="03" label="signal loss" size="grid" />);
  const root = grid.container.querySelector('[data-cover="03"][data-size="grid"]');
  expect(root?.hasAttribute('data-cover')).toStrictEqual(true);
  expect(root?.querySelector('[data-cover-wash="1"]')?.hasAttribute('data-cover-wash')).toStrictEqual(true);
  expect(root?.querySelector('[data-cover-plate="1"]')?.hasAttribute('data-cover-plate')).toStrictEqual(true);
  // LIB-142: a tiny placeholder, never a letter poster; the title lives
  // in the tile text below the art.
  expect(root?.querySelector('[data-cover-label]')).toStrictEqual(null);
  expect(root?.querySelector('[data-cover-sheen]')).toStrictEqual(null);
  grid.unmount();

  const row = render(<CoverTile tone="05" label="Stages" size="row" />);
  expect(row.container.querySelector('[data-size="row"] [data-cover-label]')).toStrictEqual(null);
});

test('art-first: real art marks the tile and silences the loading plate', () => {
  const withArt = render(<CoverTile tone="01" label="Harbour" size="grid" artUrl="/media/covers/demo-album-01.svg" />);
  const artRoot = withArt.container.querySelector('[data-cover="01"][data-size="grid"]');
  expect(artRoot?.getAttribute('data-cover-art')).toStrictEqual('1');
  // The plate and wash stay in the tree (loading base) but flagged off.
  expect(artRoot?.querySelector('[data-cover-plate="1"]')).toBeTruthy();
  expect(artRoot?.querySelector('[data-cover-wash="1"]')).toBeTruthy();
  // No letter glyph over real art, at any size.
  expect(artRoot?.querySelector('[data-cover-label]')).toBeNull();
  withArt.unmount();

  const detailWithArt = render(<CoverTile tone="02" label="Stages" size="detail" artUrl="/media/covers/x.svg" />);
  expect(detailWithArt.container.querySelector('[data-cover-art="1"] [data-cover-label]')).toBeNull();
  detailWithArt.unmount();

  const loading = render(<CoverTile tone="03" label="Harbour" size="grid" />);
  const plateRoot = loading.container.querySelector('[data-cover="03"][data-size="grid"]');
  expect(plateRoot?.getAttribute('data-cover-art')).toStrictEqual('0');
  expect(plateRoot?.querySelector('[data-cover-plate="1"]')).toBeTruthy();
});

test('bar, detail and full covers show the uppercase initial', () => {
  const bar = render(<CoverTile tone="02" label="Harbour" size="bar" coverId="cover-bar-demo" />);
  expect(bar.container.querySelector('#cover-bar-demo')?.getAttribute('data-size')).toStrictEqual('bar');
  expect(bar.container.querySelector('[data-cover-label]')?.textContent).toStrictEqual('H');
  bar.unmount();

  const detail = render(<CoverTile tone="08" label="a" size="detail" coverId="cover-detail-x" />);
  expect(detail.container.querySelector('#cover-detail-x')?.getAttribute('data-size')).toStrictEqual('detail');
  expect(detail.container.querySelector('[data-cover-label]')?.textContent).toStrictEqual('A');
  detail.unmount();

  const full = render(<CoverTile tone="01" label="Pier" size="full" coverId="cover-full-x" />);
  expect(full.container.querySelector('#cover-full-x')?.getAttribute('data-size')).toStrictEqual('full');
  expect(full.container.querySelector('[data-cover-label]')?.textContent).toStrictEqual('P');
});
