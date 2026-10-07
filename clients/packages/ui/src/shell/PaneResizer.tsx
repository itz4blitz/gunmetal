import { useEffect, useState } from 'react';
import { clampPaneWidth, paneLimit, paneWidthForKey, type PaneId } from './pane-widths.ts';

export type PaneResizerProps = {
  pane: PaneId;
  /** The accessible name of the separator, from the catalogue. */
  label: string;
  /** The pane's committed width in CSS px. */
  widthPx: number;
  /** Called on every pointer move while dragging, with the clamped width. */
  onResize: (widthPx: number) => void;
  /** Called once when a drag ends, on a resize key and on a double-click reset. */
  onCommit: (widthPx: number) => void;
};

type Drag = { startX: number; startWidth: number };

/**
 * The draggable edge of a pane: a WAI-ARIA window splitter. The sidebar's
 * edge is its trailing one, so dragging right widens it; the queue's is its
 * leading one, so dragging left widens it. Pointer moves are followed on the
 * window, so the drag survives the pointer leaving the 8px handle.
 */
export function PaneResizer({ pane, label, widthPx, onResize, onCommit }: PaneResizerProps) {
  const [drag, setDrag] = useState<Drag | undefined>(undefined);
  const limit = paneLimit(pane);
  const direction = pane === 'sidebar' ? 1 : -1;

  useEffect(() => {
    if (drag === undefined) {
      return;
    }
    const widthAt = (clientX: number) => clampPaneWidth(pane, drag.startWidth + (clientX - drag.startX) * direction);
    const onMove = (event: PointerEvent) => {
      onResize(widthAt(event.clientX));
    };
    const onUp = (event: PointerEvent) => {
      onCommit(widthAt(event.clientX));
      setDrag(undefined);
    };
    globalThis.addEventListener('pointermove', onMove);
    globalThis.addEventListener('pointerup', onUp);
    globalThis.addEventListener('pointercancel', onUp);
    return () => {
      globalThis.removeEventListener('pointermove', onMove);
      globalThis.removeEventListener('pointerup', onUp);
      globalThis.removeEventListener('pointercancel', onUp);
    };
  }, [drag, pane, direction, onResize, onCommit]);

  return (
    <div
      id={`pane-resizer-${pane}`}
      data-pane-resizer={pane}
      data-dragging={drag === undefined ? '0' : '1'}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuemin={limit.min}
      aria-valuemax={limit.max}
      aria-valuenow={widthPx}
      tabIndex={0}
      onPointerDown={(event) => {
        event.preventDefault();
        setDrag({ startX: event.clientX, startWidth: widthPx });
      }}
      onDoubleClick={() => {
        onCommit(limit.fallback);
      }}
      onKeyDown={(event) => {
        const next = paneWidthForKey(
          pane,
          widthPx,
          event.key,
          event.shiftKey,
          pane === 'sidebar' ? 'ArrowRight' : 'ArrowLeft',
        );
        if (next !== undefined) {
          event.preventDefault();
          onCommit(next);
        }
      }}
    >
      <div data-pane-resizer-grip="1" />
    </div>
  );
}
