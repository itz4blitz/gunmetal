import { expect, test } from 'vitest';
import { anchorOf, MENU_EDGE_PX, placeMenu } from './menu-anchor.ts';

const VIEW = { width: 1200, height: 800 };

test('a menu with room below the pointer opens at the pointer', () => {
  expect(placeMenu({ x: 400, y: 200 }, { width: 196, height: 190 }, VIEW, MENU_EDGE_PX)).toStrictEqual({
    left: 400,
    top: 200,
  });
});

test('a menu that would pass the bottom edge opens above the pointer instead', () => {
  const placed = placeMenu({ x: 400, y: 700 }, { width: 196, height: 190 }, VIEW, MENU_EDGE_PX);
  // 700 + 190 passes 792: the menu's bottom sits on the pointer.
  expect(placed).toStrictEqual({ left: 400, top: 510 });
});

test('a menu that would pass the right edge is pulled back inside it', () => {
  expect(placeMenu({ x: 1100, y: 200 }, { width: 196, height: 190 }, VIEW, MENU_EDGE_PX)).toStrictEqual({
    left: 996,
    top: 200,
  });
});

test('a menu anchored past the left edge is held at the edge', () => {
  expect(placeMenu({ x: 2, y: 200 }, { width: 196, height: 190 }, VIEW, MENU_EDGE_PX)).toStrictEqual({
    left: 8,
    top: 200,
  });
});

test('a menu opened above that would pass the top edge is held there', () => {
  // Below passes the bottom, above would pass the top: the edge wins.
  const placed = placeMenu({ x: 400, y: 795 }, { width: 196, height: 190 }, VIEW, MENU_EDGE_PX);
  expect(placed).toStrictEqual({ left: 400, top: 602 });
});

test('a menu opened below that would pass the top edge is held there', () => {
  expect(placeMenu({ x: 400, y: 4 }, { width: 196, height: 190 }, VIEW, MENU_EDGE_PX)).toStrictEqual({
    left: 400,
    top: 8,
  });
});

test('a viewport smaller than the menu still places it at the edge', () => {
  const placed = placeMenu({ x: 100, y: 100 }, { width: 500, height: 900 }, { width: 400, height: 300 }, MENU_EDGE_PX);
  expect(placed).toStrictEqual({ left: 8, top: 8 });
});

test('the anchor of a trigger is its bottom-left corner, where a dropdown hangs from', () => {
  const el = document.createElement('div');
  el.getBoundingClientRect = () => ({
    x: 0,
    y: 0,
    left: 10,
    top: 20,
    right: 210,
    bottom: 120,
    width: 200,
    height: 100,
    toJSON: () => ({}),
  });
  expect(anchorOf(el)).toStrictEqual({ x: 10, y: 120 });
});
