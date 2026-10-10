import { afterEach, describe, expect, test, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { cleanup } from '@testing-library/react';
import { readContentViewport, spacerHeights, useContentViewport, visibleRange } from './windowing.ts';

describe('visibleRange', () => {
  test('no rows window nothing', () => {
    expect(visibleRange(0, 400, 50, 0, 4)).toStrictEqual({ start: 0, end: 0 });
    expect(visibleRange(9999, 400, 50, -3, 4)).toStrictEqual({ start: 0, end: 0 });
  });

  test('a short list renders all of it, overscan or not', () => {
    expect(visibleRange(0, 1000, 50, 5, 0)).toStrictEqual({ start: 0, end: 5 });
    expect(visibleRange(0, 1000, 50, 5, 8)).toStrictEqual({ start: 0, end: 5 });
  });

  test('the window covers exactly the visible rows without overscan', () => {
    // 500px down at 50px rows = row 10 first; 200px tall viewport = 4 rows.
    expect(visibleRange(500, 200, 50, 1000, 0)).toStrictEqual({ start: 10, end: 14 });
    // An exact row boundary starts the window at that row.
    expect(visibleRange(250, 200, 50, 1000, 0)).toStrictEqual({ start: 5, end: 9 });
  });

  test('overscan extends the window on both sides, clamped at the top', () => {
    expect(visibleRange(500, 200, 50, 1000, 2)).toStrictEqual({ start: 8, end: 16 });
    // Near the top the start cannot go negative; the end keeps its size.
    expect(visibleRange(25, 200, 50, 1000, 3)).toStrictEqual({ start: 0, end: 10 });
  });

  test('the window clamps at the bottom edge of the list', () => {
    expect(visibleRange(500, 200, 50, 12, 0)).toStrictEqual({ start: 10, end: 12 });
    // Scrolled far past the end, at least the last row stays rendered.
    expect(visibleRange(100000, 200, 50, 20, 0)).toStrictEqual({ start: 19, end: 20 });
  });

  test('an unmeasurable viewport or row renders everything (jsdom has no layout)', () => {
    expect(visibleRange(500, 0, 50, 30, 4)).toStrictEqual({ start: 0, end: 30 });
    expect(visibleRange(500, Number.NaN, 50, 30, 4)).toStrictEqual({ start: 0, end: 30 });
    expect(visibleRange(500, Number.POSITIVE_INFINITY, 50, 30, 4)).toStrictEqual({ start: 0, end: 30 });
    expect(visibleRange(500, 200, 0, 30, 4)).toStrictEqual({ start: 0, end: 30 });
    expect(visibleRange(500, 200, Number.NaN, 30, 4)).toStrictEqual({ start: 0, end: 30 });
    expect(visibleRange(500, 200, -50, 30, 4)).toStrictEqual({ start: 0, end: 30 });
  });

  test('a hostile scroll position clamps to the top of the list', () => {
    expect(visibleRange(Number.NaN, 200, 50, 30, 0)).toStrictEqual({ start: 0, end: 4 });
    expect(visibleRange(-400, 200, 50, 30, 0)).toStrictEqual({ start: 0, end: 4 });
  });

  test('overscan defaults to none', () => {
    expect(visibleRange(500, 200, 50, 1000)).toStrictEqual({ start: 10, end: 14 });
  });
});

describe('spacerHeights', () => {
  test('the spacers are the rows above and below the window, in pixels', () => {
    expect(spacerHeights({ start: 10, end: 14 }, 100, 50)).toStrictEqual({ top: 500, bottom: 4300 });
  });

  test('a whole-list window needs no spacer, and a hostile window cannot go negative', () => {
    expect(spacerHeights({ start: 0, end: 30 }, 30, 50)).toStrictEqual({ top: 0, bottom: 0 });
    expect(spacerHeights({ start: -5, end: 999 }, 10, 50)).toStrictEqual({ top: 0, bottom: 0 });
    // A window past the count, an empty list, a row height with no layout.
    expect(spacerHeights({ start: 12, end: 20 }, 10, 50)).toStrictEqual({ top: 500, bottom: 0 });
    expect(spacerHeights({ start: 0, end: 0 }, 0, 50)).toStrictEqual({ top: 0, bottom: 0 });
    expect(spacerHeights({ start: 1, end: 2 }, 4, 0)).toStrictEqual({ top: 0, bottom: 0 });
    // A negative count is no rows at all.
    expect(spacerHeights({ start: 1, end: 2 }, -3, 50)).toStrictEqual({ top: 0, bottom: 0 });
  });
});

describe('readContentViewport', () => {
  test('reads the scroll offset and the client height of the content pane', () => {
    const scroller = document.createElement('div');
    Object.defineProperty(scroller, 'scrollTop', { value: 320, configurable: true });
    Object.defineProperty(scroller, 'clientHeight', { value: 480, configurable: true });
    expect(readContentViewport(scroller)).toStrictEqual({ scrollTop: 320, viewportHeight: 480 });
  });
});

describe('useContentViewport', () => {
  afterEach(cleanup);

  function fakeScroller(): { element: HTMLDivElement; setScroll: (top: number) => void } {
    const element = document.createElement('div');
    document.body.appendChild(element);
    let top = 0;
    Object.defineProperty(element, 'scrollTop', { get: () => top, configurable: true });
    Object.defineProperty(element, 'clientHeight', { value: 600, configurable: true });
    return {
      element,
      setScroll: (value) => {
        top = value;
      },
    };
  }

  test('reads the scroller once at mount and again on every scroll', () => {
    const { element, setScroll } = fakeScroller();
    setScroll(120);
    const { result, rerender, unmount } = renderHook(() => useContentViewport(() => element));
    expect(result.current).toStrictEqual({ scrollTop: 120, viewportHeight: 600 });
    setScroll(480);
    element.dispatchEvent(new Event('scroll'));
    rerender();
    expect(result.current).toStrictEqual({ scrollTop: 480, viewportHeight: 600 });
    // Reading the same state again keeps the old object: no re-render churn.
    element.dispatchEvent(new Event('scroll'));
    rerender();
    expect(result.current).toStrictEqual({ scrollTop: 480, viewportHeight: 600 });
    unmount();
  });

  test('without a scroller the read answers zeros and every row renders', () => {
    const getScroller = vi.fn(() => null);
    const { result } = renderHook(() => useContentViewport(getScroller));
    expect(result.current).toStrictEqual({ scrollTop: 0, viewportHeight: 0 });
    expect(getScroller).toHaveBeenCalledTimes(1);
  });

  test('unmount removes the scroll listener', () => {
    const { element } = fakeScroller();
    const removeEventListener = vi.spyOn(element, 'removeEventListener');
    const { unmount } = renderHook(() => useContentViewport(() => element));
    unmount();
    expect(removeEventListener).toHaveBeenCalledWith('scroll', expect.any(Function));
  });
});
