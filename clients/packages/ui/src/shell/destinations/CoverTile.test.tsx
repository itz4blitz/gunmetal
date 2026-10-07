import { act } from 'react';
import { cleanup, render } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { CoverTile } from './CoverTile.tsx';
import { intersectionObserverFactory } from './near-view.ts';

afterEach(cleanup);

/** A scriptable stand-in for the browser's IntersectionObserver. */
class StubObserver {
  static instances: StubObserver[] = [];
  readonly observed: Element[] = [];
  disconnected = false;
  readonly callback: (entries: Array<{ isIntersecting: boolean }>) => void;
  constructor(callback: (entries: Array<{ isIntersecting: boolean }>) => void) {
    this.callback = callback;
    StubObserver.instances = [];
    StubObserver.instances.push(this);
  }
  observe(target: Element): void {
    this.observed.push(target);
  }
  unobserve(): void {}
  disconnect(): void {
    this.disconnected = true;
  }
  see(isIntersecting: boolean): void {
    this.callback([{ isIntersecting }]);
  }
}

const asIO = StubObserver as unknown as typeof IntersectionObserver;

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
  expect(artRoot?.querySelector('[data-cover-plate="1"]')).not.toBeNull();
  expect(artRoot?.querySelector('[data-cover-wash="1"]')).not.toBeNull();
  // No letter glyph over real art, at any size.
  expect(artRoot?.querySelector('[data-cover-label]')).toBeNull();
  withArt.unmount();

  const detailWithArt = render(<CoverTile tone="02" label="Stages" size="detail" artUrl="/media/covers/x.svg" />);
  expect(detailWithArt.container.querySelector('[data-cover-art="1"] [data-cover-label]')).toBeNull();
  detailWithArt.unmount();

  const loading = render(<CoverTile tone="03" label="Harbour" size="grid" />);
  const plateRoot = loading.container.querySelector('[data-cover="03"][data-size="grid"]');
  expect(plateRoot?.getAttribute('data-cover-art')).toStrictEqual('0');
  expect(plateRoot?.querySelector('[data-cover-plate="1"]')).not.toBeNull();
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

test('deferred art waits for its near-viewport report before painting the URL', () => {
  const factory = intersectionObserverFactory(asIO);
  if (factory === undefined) {
    throw new Error('stub observer missing');
  }
  const view = render(
    <CoverTile
      tone="01"
      label="Harbour"
      size="grid"
      coverId="cover-grid-deferred"
      artUrl="/media/covers/demo-album-01.svg"
      deferArt={factory}
    />,
  );
  const root = view.container.querySelector('#cover-grid-deferred');
  expect(root).not.toBeNull();
  // The tile observes itself in the live document…
  expect(StubObserver.instances[0]?.observed[0]).toStrictEqual(document.getElementById('cover-grid-deferred'));
  // …but the art URL waits: the tone plate stands in.
  expect(root?.getAttribute('style') ?? '').not.toContain('background-image');
  expect(root?.getAttribute('data-cover-art')).toStrictEqual('1');
  act(() => {
    StubObserver.instances[0]?.see(true);
  });
  expect(root?.getAttribute('style')).toContain('background-image');
  // The observation ends with the tile.
  view.unmount();
  expect(StubObserver.instances[0]?.disconnected).toStrictEqual(true);
});

test('without a factory the cover paints its art immediately (jsdom path)', () => {
  const view = render(
    <CoverTile
      tone="01"
      label="Harbour"
      size="grid"
      coverId="cover-grid-eager"
      artUrl="/media/covers/demo-album-01.svg"
    />,
  );
  expect(view.container.querySelector('#cover-grid-eager')?.getAttribute('style')).toContain('background-image');
});
