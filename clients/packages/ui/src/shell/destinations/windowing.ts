import { useEffect, useState } from 'react';

/**
 * Hand-rolled list windowing for the browsing tables (zero dependencies):
 * a pure row-window function plus the jsdom-friendly scroll read that feeds
 * it. The content pane (#content) is the scroll container — the page owns
 * the scrollbar, the lists inside it only ask what is visible.
 */

/** Half-open row window [start, end) the viewport can see. */
export type VisibleWindow = { start: number; end: number };

/** Rows of padding rendered above and below the visible window. */
export const LIST_OVERSCAN_ROWS = 8;

/** The content pane's scroll offset and visible height. */
export type ViewportRead = { scrollTop: number; viewportHeight: number };

/**
 * The rows a viewport shows: uniform row height, `overscan` rows of padding
 * on both sides. Anything unmeasurable — a zero or non-finite viewport or
 * row height, as in jsdom where nothing has layout — answers the whole list:
 * a wrong estimate must never hide a row.
 */
export function visibleRange(
  scrollTop: number,
  viewportHeight: number,
  rowHeight: number,
  count: number,
  overscan = 0,
): VisibleWindow {
  if (count <= 0) {
    return { start: 0, end: 0 };
  }
  if (!Number.isFinite(rowHeight) || rowHeight <= 0) {
    return { start: 0, end: count };
  }
  if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) {
    return { start: 0, end: count };
  }
  const top = Number.isFinite(scrollTop) && scrollTop > 0 ? scrollTop : 0;
  const visibleRows = Math.ceil(viewportHeight / rowHeight);
  const firstVisible = Math.floor(top / rowHeight);
  const start = Math.max(0, Math.min(firstVisible - overscan, count - 1));
  const end = Math.min(count, start + visibleRows + overscan * 2);
  return { start, end };
}

/** Reads a scroll container's viewport state as the pure function wants it. */
export function readContentViewport(scroller: HTMLElement): ViewportRead {
  return { scrollTop: scroller.scrollTop, viewportHeight: scroller.clientHeight };
}

const EMPTY_VIEWPORT: ViewportRead = { scrollTop: 0, viewportHeight: 0 };

/** The content pane, by the id the shell frame gives it. */
export function contentScroller(): HTMLElement | null {
  return document.getElementById('content');
}

/**
 * The viewport state of the scroller `getScroller` answers, kept current by
 * its scroll events and read once at mount. Without a scroller — jsdom, or
 * a host that renders no frame — the read stays at zero, which the window
 * function treats as "render everything".
 */
export function useContentViewport(getScroller: () => HTMLElement | null): ViewportRead {
  const [read, setRead] = useState<ViewportRead>(EMPTY_VIEWPORT);
  useEffect(() => {
    const scroller = getScroller();
    if (scroller === null) {
      return undefined;
    }
    const onScroll = () => {
      const next = readContentViewport(scroller);
      setRead((prev) =>
        prev.scrollTop === next.scrollTop && prev.viewportHeight === next.viewportHeight ? prev : next,
      );
    };
    onScroll();
    scroller.addEventListener('scroll', onScroll);
    return () => {
      scroller.removeEventListener('scroll', onScroll);
    };
  }, [getScroller]);
  return read;
}
