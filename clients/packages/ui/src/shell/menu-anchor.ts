/**
 * Where a menu goes. The anchor is where the menu's top-left corner wants
 * to be: the pointer for a right-click, the trigger's bottom-left corner
 * for a kebab. The menu opens downward from there; the viewport has the
 * last word — it flips above the anchor when the bottom would not hold the
 * menu, and it never lets the menu poke past an edge. Pure, so every flip
 * and clamp is proven without a layout engine.
 */

export type MenuPoint = { x: number; y: number };

export type MenuSize = { width: number; height: number };

export type MenuViewport = { width: number; height: number };

/** Where the menu goes: CSS left/top against the window. */
export type MenuPlacement = { left: number; top: number };

/** How close to the window's edge a menu may sit, in pixels. */
export const MENU_EDGE_PX = 8;

/** The dropdown anchor: the trigger's bottom-left corner, from the live element. */
export function anchorOf(el: Element): MenuPoint {
  const rect = el.getBoundingClientRect();
  return { x: rect.left, y: rect.bottom };
}

export function placeMenu(anchor: MenuPoint, size: MenuSize, viewport: MenuViewport, edge: number): MenuPlacement {
  const maxLeft = viewport.width - edge - size.width;
  const maxTop = viewport.height - edge - size.height;

  let left = anchor.x;
  if (left > maxLeft) {
    left = maxLeft;
  }
  if (left < edge) {
    left = edge;
  }

  let top = anchor.y;
  if (top + size.height > viewport.height - edge) {
    top = anchor.y - size.height;
  }
  if (top > maxTop) {
    top = maxTop;
  }
  if (top < edge) {
    top = edge;
  }

  return { left, top };
}
