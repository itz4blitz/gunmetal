import { expect, test } from 'vitest';
import {
  clampPaneWidth,
  defaultPaneWidths,
  noLayoutStore,
  paneLimit,
  paneWidthForKey,
  parsePaneWidths,
  serializePaneWidths,
} from './pane-widths.ts';

test('each pane has literal limits and the defaults are their fallbacks', () => {
  expect(paneLimit('sidebar')).toStrictEqual({ min: 200, max: 420, fallback: 256 });
  expect(paneLimit('queue')).toStrictEqual({ min: 280, max: 560, fallback: 340 });
  expect(defaultPaneWidths()).toStrictEqual({ sidebar: 256, queue: 340 });
});

test('a width is rounded and held inside the limits, edges included', () => {
  expect(clampPaneWidth('sidebar', 300.4)).toStrictEqual(300);
  expect(clampPaneWidth('sidebar', 300.5)).toStrictEqual(301);
  expect(clampPaneWidth('sidebar', 200)).toStrictEqual(200);
  expect(clampPaneWidth('sidebar', 199)).toStrictEqual(200);
  expect(clampPaneWidth('sidebar', 420)).toStrictEqual(420);
  expect(clampPaneWidth('sidebar', 421)).toStrictEqual(420);
  expect(clampPaneWidth('queue', 0)).toStrictEqual(280);
  expect(clampPaneWidth('queue', 280)).toStrictEqual(280);
  expect(clampPaneWidth('queue', 560)).toStrictEqual(560);
  expect(clampPaneWidth('queue', 9000)).toStrictEqual(560);
  expect(clampPaneWidth('queue', -50)).toStrictEqual(280);
});

test('anything that is not a finite number is the default width', () => {
  expect(clampPaneWidth('sidebar', Number.NaN)).toStrictEqual(256);
  expect(clampPaneWidth('sidebar', Number.POSITIVE_INFINITY)).toStrictEqual(256);
  expect(clampPaneWidth('queue', Number.NEGATIVE_INFINITY)).toStrictEqual(340);
  expect(clampPaneWidth('queue', '400')).toStrictEqual(340);
  expect(clampPaneWidth('queue', null)).toStrictEqual(340);
  expect(clampPaneWidth('sidebar', undefined)).toStrictEqual(256);
  expect(clampPaneWidth('sidebar', { width: 300 })).toStrictEqual(256);
});

test('a stored layout is read back, clamped field by field', () => {
  expect(parsePaneWidths('{"sidebar":312,"queue":400}')).toStrictEqual({ sidebar: 312, queue: 400 });
  expect(parsePaneWidths('{"sidebar":10,"queue":99999}')).toStrictEqual({ sidebar: 200, queue: 560 });
  expect(parsePaneWidths('{"sidebar":"wide","queue":300}')).toStrictEqual({ sidebar: 256, queue: 300 });
  expect(parsePaneWidths('{"queue":300}')).toStrictEqual({ sidebar: 256, queue: 300 });
  expect(parsePaneWidths('{"sidebar":300,"extra":true}')).toStrictEqual({ sidebar: 300, queue: 340 });
});

test('nothing stored, broken JSON and non-objects are all the default layout', () => {
  expect(parsePaneWidths(null)).toStrictEqual({ sidebar: 256, queue: 340 });
  expect(parsePaneWidths('')).toStrictEqual({ sidebar: 256, queue: 340 });
  expect(parsePaneWidths('{"sidebar":')).toStrictEqual({ sidebar: 256, queue: 340 });
  expect(parsePaneWidths('null')).toStrictEqual({ sidebar: 256, queue: 340 });
  expect(parsePaneWidths('312')).toStrictEqual({ sidebar: 256, queue: 340 });
  expect(parsePaneWidths('"312"')).toStrictEqual({ sidebar: 256, queue: 340 });
  expect(parsePaneWidths('[312,400]')).toStrictEqual({ sidebar: 256, queue: 340 });
});

test('a layout is written as two clamped pixel counts and nothing else', () => {
  expect(serializePaneWidths({ sidebar: 312, queue: 400 })).toStrictEqual('{"sidebar":312,"queue":400}');
  expect(serializePaneWidths({ sidebar: 1, queue: 5000.7 })).toStrictEqual('{"sidebar":200,"queue":560}');
  expect(serializePaneWidths({ sidebar: Number.NaN, queue: 300.5 })).toStrictEqual('{"sidebar":256,"queue":301}');
});

test('arrows move the sidebar edge 16px, or 48px with Shift, toward the arrow', () => {
  expect(paneWidthForKey('sidebar', 256, 'ArrowRight', false, 'ArrowRight')).toStrictEqual(272);
  expect(paneWidthForKey('sidebar', 256, 'ArrowLeft', false, 'ArrowRight')).toStrictEqual(240);
  expect(paneWidthForKey('sidebar', 256, 'ArrowRight', true, 'ArrowRight')).toStrictEqual(304);
  expect(paneWidthForKey('sidebar', 256, 'ArrowLeft', true, 'ArrowRight')).toStrictEqual(208);
  // The limits hold at both ends.
  expect(paneWidthForKey('sidebar', 412, 'ArrowRight', false, 'ArrowRight')).toStrictEqual(420);
  expect(paneWidthForKey('sidebar', 210, 'ArrowLeft', true, 'ArrowRight')).toStrictEqual(200);
});

test('the queue grows toward the left arrow because its edge is the leading one', () => {
  expect(paneWidthForKey('queue', 340, 'ArrowLeft', false, 'ArrowLeft')).toStrictEqual(356);
  expect(paneWidthForKey('queue', 340, 'ArrowRight', false, 'ArrowLeft')).toStrictEqual(324);
  expect(paneWidthForKey('queue', 340, 'ArrowLeft', true, 'ArrowLeft')).toStrictEqual(388);
  expect(paneWidthForKey('queue', 290, 'ArrowRight', false, 'ArrowLeft')).toStrictEqual(280);
});

test('Home, End and Enter jump to the minimum, the maximum and the default', () => {
  expect(paneWidthForKey('sidebar', 300, 'Home', false, 'ArrowRight')).toStrictEqual(200);
  expect(paneWidthForKey('sidebar', 300, 'End', false, 'ArrowRight')).toStrictEqual(420);
  expect(paneWidthForKey('sidebar', 300, 'Enter', false, 'ArrowRight')).toStrictEqual(256);
  expect(paneWidthForKey('queue', 300, 'Home', true, 'ArrowLeft')).toStrictEqual(280);
  expect(paneWidthForKey('queue', 300, 'End', true, 'ArrowLeft')).toStrictEqual(560);
  expect(paneWidthForKey('queue', 300, 'Enter', true, 'ArrowLeft')).toStrictEqual(340);
});

test('any other key is not a resize key', () => {
  expect(paneWidthForKey('sidebar', 300, 'Tab', false, 'ArrowRight')).toStrictEqual(undefined);
  expect(paneWidthForKey('sidebar', 300, 'ArrowUp', false, 'ArrowRight')).toStrictEqual(undefined);
  expect(paneWidthForKey('queue', 300, ' ', false, 'ArrowLeft')).toStrictEqual(undefined);
  expect(paneWidthForKey('queue', 300, 'a', true, 'ArrowLeft')).toStrictEqual(undefined);
});

test('the unwired store reads nothing and swallows writes', () => {
  const store = noLayoutStore();
  expect(store.read()).toStrictEqual(null);
  expect(store.write('{"sidebar":300,"queue":400}')).toStrictEqual(undefined);
  expect(store.read()).toStrictEqual(null);
});
