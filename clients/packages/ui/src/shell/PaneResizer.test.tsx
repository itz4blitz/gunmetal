import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { PaneResizer } from './PaneResizer.tsx';
import type { PaneId } from './pane-widths.ts';

afterEach(cleanup);

type Recorded = { resized: number[]; committed: number[] };

function mount(pane: PaneId, widthPx: number): Recorded & { separator: HTMLElement } {
  const resized: number[] = [];
  const committed: number[] = [];
  render(
    <PaneResizer
      pane={pane}
      label={pane === 'sidebar' ? 'Resize sidebar' : 'Resize queue'}
      widthPx={widthPx}
      onResize={(next) => {
        resized.push(next);
      }}
      onCommit={(next) => {
        committed.push(next);
      }}
    />,
  );
  const separator = screen.getByRole('separator');
  return { resized, committed, separator };
}

test('the sidebar edge is a focusable vertical separator with its limits and value', () => {
  const { separator } = mount('sidebar', 256);
  expect(separator.id).toStrictEqual('pane-resizer-sidebar');
  expect(separator.getAttribute('data-pane-resizer')).toStrictEqual('sidebar');
  expect(separator.getAttribute('aria-label')).toStrictEqual('Resize sidebar');
  expect(separator.getAttribute('aria-orientation')).toStrictEqual('vertical');
  expect(separator.getAttribute('aria-valuemin')).toStrictEqual('200');
  expect(separator.getAttribute('aria-valuemax')).toStrictEqual('420');
  expect(separator.getAttribute('aria-valuenow')).toStrictEqual('256');
  expect(separator.getAttribute('tabindex')).toStrictEqual('0');
  expect(separator.getAttribute('data-dragging')).toStrictEqual('0');
  expect(separator.querySelector('[data-pane-resizer-grip="1"]')?.tagName).toStrictEqual('DIV');
});

test('the queue edge carries the queue limits', () => {
  const { separator } = mount('queue', 340);
  expect(separator.id).toStrictEqual('pane-resizer-queue');
  expect(separator.getAttribute('aria-label')).toStrictEqual('Resize queue');
  expect(separator.getAttribute('aria-valuemin')).toStrictEqual('280');
  expect(separator.getAttribute('aria-valuemax')).toStrictEqual('560');
  expect(separator.getAttribute('aria-valuenow')).toStrictEqual('340');
});

test('dragging the sidebar edge right widens it live and commits once on release', () => {
  const { separator, resized, committed } = mount('sidebar', 256);
  fireEvent.pointerDown(separator, { clientX: 256 });
  expect(separator.getAttribute('data-dragging')).toStrictEqual('1');
  fireEvent.pointerMove(window, { clientX: 300 });
  fireEvent.pointerMove(window, { clientX: 180 });
  fireEvent.pointerMove(window, { clientX: 900 });
  expect(resized).toStrictEqual([300, 200, 420]);
  expect(committed).toStrictEqual([]);
  fireEvent.pointerUp(window, { clientX: 320 });
  expect(committed).toStrictEqual([320]);
  expect(separator.getAttribute('data-dragging')).toStrictEqual('0');
  // The drag is over: later moves and releases are not followed.
  fireEvent.pointerMove(window, { clientX: 350 });
  fireEvent.pointerUp(window, { clientX: 350 });
  expect(resized).toStrictEqual([300, 200, 420]);
  expect(committed).toStrictEqual([320]);
});

test('dragging the queue edge left widens it, and a cancelled pointer still commits', () => {
  const { separator, resized, committed } = mount('queue', 340);
  fireEvent.pointerDown(separator, { clientX: 1100 });
  fireEvent.pointerMove(window, { clientX: 1040 });
  fireEvent.pointerMove(window, { clientX: 1150 });
  expect(resized).toStrictEqual([400, 290]);
  fireEvent.pointerCancel(window, { clientX: 1000 });
  expect(committed).toStrictEqual([440]);
  expect(separator.getAttribute('data-dragging')).toStrictEqual('0');
});

test('moves before a press are ignored', () => {
  const { resized, committed } = mount('sidebar', 256);
  fireEvent.pointerMove(window, { clientX: 400 });
  fireEvent.pointerUp(window, { clientX: 400 });
  expect(resized).toStrictEqual([]);
  expect(committed).toStrictEqual([]);
});

test('unmounting mid-drag stops following the pointer', () => {
  const { separator, resized, committed } = mount('sidebar', 256);
  fireEvent.pointerDown(separator, { clientX: 256 });
  cleanup();
  fireEvent.pointerMove(window, { clientX: 300 });
  fireEvent.pointerUp(window, { clientX: 300 });
  expect(resized).toStrictEqual([]);
  expect(committed).toStrictEqual([]);
});

test('arrow keys, Home, End and Enter commit new widths; other keys do nothing', () => {
  const sidebar = mount('sidebar', 256);
  fireEvent.keyDown(sidebar.separator, { key: 'ArrowRight' });
  fireEvent.keyDown(sidebar.separator, { key: 'ArrowLeft' });
  fireEvent.keyDown(sidebar.separator, { key: 'ArrowRight', shiftKey: true });
  fireEvent.keyDown(sidebar.separator, { key: 'Home' });
  fireEvent.keyDown(sidebar.separator, { key: 'End' });
  fireEvent.keyDown(sidebar.separator, { key: 'Enter' });
  fireEvent.keyDown(sidebar.separator, { key: 'Tab' });
  fireEvent.keyDown(sidebar.separator, { key: 'a' });
  expect(sidebar.committed).toStrictEqual([272, 240, 304, 200, 420, 256]);
  expect(sidebar.resized).toStrictEqual([]);
  cleanup();

  const queue = mount('queue', 340);
  fireEvent.keyDown(queue.separator, { key: 'ArrowLeft' });
  fireEvent.keyDown(queue.separator, { key: 'ArrowRight' });
  expect(queue.committed).toStrictEqual([356, 324]);
});

test('a resize key is consumed and any other key is left to the browser', () => {
  const { separator } = mount('sidebar', 256);
  // fireEvent returns false when the handler called preventDefault.
  expect(fireEvent.keyDown(separator, { key: 'ArrowRight' })).toStrictEqual(false);
  expect(fireEvent.keyDown(separator, { key: 'Tab' })).toStrictEqual(true);
  // The press itself is consumed so the drag never starts a text selection.
  expect(fireEvent.pointerDown(separator, { clientX: 256 })).toStrictEqual(false);
});

test('a double-click returns the pane to its default width', () => {
  const sidebar = mount('sidebar', 400);
  fireEvent.doubleClick(sidebar.separator);
  expect(sidebar.committed).toStrictEqual([256]);
  cleanup();
  const queue = mount('queue', 500);
  fireEvent.doubleClick(queue.separator);
  expect(queue.committed).toStrictEqual([340]);
});
