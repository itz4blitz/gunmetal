import { expect, test } from 'vitest';
import { landmarks, landmarksForClass, widthClass } from './width.ts';

test('width classes follow the design-language breakpoints', () => {
  expect(widthClass(319)).toStrictEqual('compact');
  expect(widthClass(359)).toStrictEqual('compact');
  expect(widthClass(599)).toStrictEqual('compact');
  expect(widthClass(600)).toStrictEqual('medium');
  expect(widthClass(800)).toStrictEqual('medium');
  expect(widthClass(1023)).toStrictEqual('medium');
  expect(widthClass(1024)).toStrictEqual('expanded');
  expect(widthClass(1200)).toStrictEqual('expanded');
  expect(widthClass(1439)).toStrictEqual('expanded');
  expect(widthClass(1440)).toStrictEqual('wide');
  expect(widthClass(1600)).toStrictEqual('wide');
});

test('landmarks at 360, 800, 1200 and 1600 equal the literal lists', () => {
  expect(landmarks(360)).toStrictEqual(['content', 'player-bar', 'nav-tabs']);
  expect(landmarks(800)).toStrictEqual(['nav-rail', 'content', 'player-bar']);
  expect(landmarks(1200)).toStrictEqual(['nav-sidebar', 'content', 'player-bar']);
  expect(landmarks(1600)).toStrictEqual(['nav-sidebar', 'content', 'right-pane', 'player-bar']);
  expect(landmarksForClass('compact')).toStrictEqual(['content', 'player-bar', 'nav-tabs']);
  expect(landmarksForClass('medium')).toStrictEqual(['nav-rail', 'content', 'player-bar']);
  expect(landmarksForClass('expanded')).toStrictEqual(['nav-sidebar', 'content', 'player-bar']);
  expect(landmarksForClass('wide')).toStrictEqual(['nav-sidebar', 'content', 'right-pane', 'player-bar']);
});
