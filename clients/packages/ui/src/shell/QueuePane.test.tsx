import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { catalogue } from '../messages/catalogue.ts';
import { emptySnapshot, queuedSnapshot } from './test-playback.ts';
import { QueuePane } from './QueuePane.tsx';

afterEach(cleanup);

test('queue pane sheet close is optional and ignores non-activation keys', () => {
  const messages = catalogue().shell;
  const playback = queuedSnapshot();
  const view = render(<QueuePane messages={messages} playback={playback} compactSheet />);
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  // The sheet's close is an icon button named by its label, at the end of the header.
  expect(screen.getByRole('button', { name: 'Close queue' }).id).toStrictEqual('queue-close');
  expect(
    screen.getByRole('button', { name: 'Close queue' }).querySelector('svg')?.getAttribute('data-icon'),
  ).toStrictEqual('close');
  expect(screen.getByRole('button', { name: 'Close queue' }).textContent).toStrictEqual('');
  expect(
    [...(document.querySelector('#queue-sheet [data-queue-header="1"]')?.children ?? [])].map(
      (node) => node.id || node.textContent,
    ),
  ).toStrictEqual(['Up next', '4', 'queue-close']);
  fireEvent.click(screen.getByRole('button', { name: 'Close queue' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: 'Tab' });
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  view.unmount();

  const empty = render(<QueuePane messages={messages} playback={emptySnapshot()} compactSheet={false} />);
  expect(screen.getByText('Queue is empty').id).toStrictEqual('queue-empty');
  expect(document.querySelector('#queue-empty')?.getAttribute('data-empty-state')).toStrictEqual('queue');
  // A calm empty state: the queue glyph over one quiet sentence — no boxed card.
  expect(document.querySelector('#right-pane [data-queue-empty="1"]')?.textContent).toStrictEqual('Queue is empty');
  expect(
    [...(document.querySelector('#right-pane [data-queue-empty="1"]')?.children ?? [])].map(
      (node) => node.id || node.getAttribute('data-queue-empty-mark'),
    ),
  ).toStrictEqual(['1', 'queue-empty']);
  expect(
    document.querySelector('#right-pane [data-queue-empty-mark="1"] svg')?.getAttribute('data-icon'),
  ).toStrictEqual('queue');
  expect(
    document.querySelector('#right-pane [data-queue-empty-mark="1"] svg')?.getAttribute('aria-hidden'),
  ).toStrictEqual('true');
  expect(document.querySelector('#right-pane [data-empty-card]')).toBeNull();
  expect(document.querySelector('#right-pane #queue-list')).toBeNull();
  // The pane keeps its header when empty: the heading alone, no count, no close.
  expect(
    [...(document.querySelector('#right-pane [data-queue-header="1"]')?.children ?? [])].map(
      (node) => node.textContent,
    ),
  ).toStrictEqual(['Up next']);
  expect(screen.getByRole('heading', { name: 'Up next' }).getAttribute('data-queue-heading')).toStrictEqual('1');
  expect(document.querySelector('#right-pane')?.getAttribute('role')).toStrictEqual('complementary');
  empty.unmount();

  const closed = render(<QueuePane messages={messages} playback={{ ...playback, queueOpen: false }} compactSheet />);
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('0');
  closed.unmount();
});

test('queue lines reveal a play action that plays that line through the handler', () => {
  const messages = catalogue().shell;
  const playback = queuedSnapshot();
  const onPlayLine = vi.fn();
  const view = render(
    <QueuePane messages={messages} playback={playback} compactSheet={false} onPlayLine={onPlayLine} />,
  );
  const salt = screen.getByRole('button', { name: 'Play Salt Window' });
  expect(salt.getAttribute('data-queue-play')).toStrictEqual('1');
  expect(salt.id).toStrictEqual('queue-play-demo-track-01-02');
  fireEvent.click(salt);
  expect(onPlayLine).toHaveBeenCalledTimes(1);
  expect(onPlayLine).toHaveBeenCalledWith('demo-album-01', 'demo-track-01-02');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play Pier at Dusk' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Play Low Tide Letter' }), { key: ' ' });
  expect(onPlayLine).toHaveBeenCalledTimes(3);
  expect(onPlayLine).toHaveBeenLastCalledWith('demo-album-01', 'demo-track-01-03');
  fireEvent.keyDown(salt, { key: 'Tab' });
  expect(onPlayLine).toHaveBeenCalledTimes(3);
  view.unmount();

  // The wiring is optional: without a handler the action stays inert.
  const idle = render(<QueuePane messages={messages} playback={queuedSnapshot(1)} compactSheet />);
  fireEvent.click(screen.getByRole('button', { name: 'Play Salt Window' }));
  expect(document.querySelector('#queue-sheet')?.id).toStrictEqual('queue-sheet');
  idle.unmount();
});

test('the queue names the line that is playing and lists the rest in order', () => {
  const messages = catalogue().shell;
  const view = render(<QueuePane messages={messages} playback={queuedSnapshot(1)} compactSheet={false} />);
  // One "Now playing" label, directly above the current line; earlier and
  // later lines keep their queue order around it.
  expect([...document.querySelectorAll('#queue-list > *')].map((node) => node.id || node.textContent)).toStrictEqual([
    'queue-line-demo-track-01-01',
    'Now playing',
    'queue-line-demo-track-01-02',
    'queue-line-demo-track-01-03',
    'queue-line-demo-track-01-04',
  ]);
  expect(
    [...document.querySelectorAll('#queue-list [data-queue-label]')].map((node) =>
      node.getAttribute('data-queue-label'),
    ),
  ).toStrictEqual(['now']);
  expect(
    [...document.querySelectorAll('#queue-list [data-queue-line]')].map((node) => node.getAttribute('data-current')),
  ).toStrictEqual(['0', '1', '0', '0']);
  const current = document.querySelector('#queue-line-demo-track-01-02');
  expect(current?.querySelector('[data-queue-title="1"]')?.textContent).toStrictEqual('Salt Window');
  expect(current?.querySelector('[data-queue-artist="1"]')?.textContent).toStrictEqual('Mira Sol');
  expect(current?.querySelector('[data-queue-duration="1"]')?.textContent).toStrictEqual('3:18');
  expect(current?.querySelector('[data-now-playing]')?.getAttribute('data-now-playing')).toStrictEqual('1');
  expect(current?.querySelector('[data-size="row"]')?.id).toStrictEqual('queue-art-demo-track-01-02');
  expect(
    document.querySelector('#queue-line-demo-track-01-03 [data-now-playing]')?.getAttribute('data-now-playing'),
  ).toStrictEqual('0');
  expect(document.querySelector('#right-pane [data-queue-count="1"]')?.textContent).toStrictEqual('4');
  expect(document.querySelector('#right-pane #queue-close')).toBeNull();
  view.unmount();

  // Paused: the current line keeps its label and place, its bars stand still.
  const paused = render(
    <QueuePane messages={messages} playback={{ ...queuedSnapshot(1), playing: false }} compactSheet={false} />,
  );
  expect(
    document.querySelector('#queue-line-demo-track-01-02 [data-now-playing]')?.getAttribute('data-now-playing'),
  ).toStrictEqual('0');
  expect(document.querySelector('#queue-list [data-queue-label="now"]')?.nextElementSibling?.id).toStrictEqual(
    'queue-line-demo-track-01-02',
  );
  paused.unmount();

  // A queue that does not hold the playing track names no line.
  const elsewhere = render(
    <QueuePane
      messages={messages}
      playback={{ ...queuedSnapshot(), trackId: 'demo-track-09-09' }}
      compactSheet={false}
    />,
  );
  expect([...document.querySelectorAll('#queue-list > *')].map((node) => node.id)).toStrictEqual([
    'queue-line-demo-track-01-01',
    'queue-line-demo-track-01-02',
    'queue-line-demo-track-01-03',
    'queue-line-demo-track-01-04',
  ]);
  expect(document.querySelector('#queue-list [data-queue-label]')).toBeNull();
  elsewhere.unmount();
});

test('each queue line offers its removal, wired to the handler', () => {
  const messages = catalogue().shell;
  const onRemoveLine = vi.fn();
  const view = render(
    <QueuePane messages={messages} playback={queuedSnapshot()} compactSheet={false} onRemoveLine={onRemoveLine} />,
  );
  const remove = screen.getByRole('button', { name: 'Remove from queue: Salt Window' });
  expect(remove.getAttribute('data-queue-remove')).toStrictEqual('1');
  expect(remove.id).toStrictEqual('queue-remove-demo-track-01-02');
  expect(
    [...document.querySelectorAll('[data-queue-remove="1"]')].map((node) => node.getAttribute('aria-label')),
  ).toStrictEqual([
    'Remove from queue: Pier at Dusk',
    'Remove from queue: Salt Window',
    'Remove from queue: Low Tide Letter',
    'Remove from queue: Beacon',
  ]);
  fireEvent.click(remove);
  expect(onRemoveLine).toHaveBeenCalledTimes(1);
  expect(onRemoveLine).toHaveBeenCalledWith('demo-track-01-02');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Remove from queue: Pier at Dusk' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Remove from queue: Beacon' }), { key: ' ' });
  fireEvent.keyDown(remove, { key: 'Tab' });
  expect(onRemoveLine).toHaveBeenCalledTimes(3);
  expect(onRemoveLine).toHaveBeenLastCalledWith('demo-track-01-04');
  view.unmount();
  // Without a handler there is no remove control at all: nothing inert.
  const unwired = render(<QueuePane messages={messages} playback={queuedSnapshot()} compactSheet />);
  expect(document.querySelectorAll('[data-queue-remove="1"]')).toHaveLength(0);
  unwired.unmount();
  // The empty queue has nothing to remove.
  const empty = render(
    <QueuePane messages={messages} playback={emptySnapshot()} compactSheet={false} onRemoveLine={onRemoveLine} />,
  );
  expect(document.querySelectorAll('[data-queue-remove="1"]')).toHaveLength(0);
  empty.unmount();
});

test('the queue sheet closes from its button, its scrim and Escape, and only while it is open', () => {
  const messages = catalogue().shell;
  const onCloseSheet = vi.fn();
  const open = render(
    <QueuePane messages={messages} playback={queuedSnapshot()} compactSheet onCloseSheet={onCloseSheet} />,
  );
  const close = screen.getByRole('button', { name: 'Close queue' });
  expect(close.getAttribute('tabindex')).toStrictEqual('0');
  fireEvent.click(close);
  expect(onCloseSheet).toHaveBeenCalledTimes(1);
  fireEvent.keyDown(close, { key: 'Enter' });
  expect(onCloseSheet).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(close, { key: ' ' });
  expect(onCloseSheet).toHaveBeenCalledTimes(3);
  // The scrim sits beside the sheet in one layer and closes it on a press.
  expect([...(document.querySelector('[data-queue-layer="1"]')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'queue-scrim',
    'queue-sheet',
  ]);
  fireEvent.click(document.querySelector('#queue-scrim') as HTMLElement);
  expect(onCloseSheet).toHaveBeenCalledTimes(4);
  // Escape anywhere in the sheet closes it; other keys pass through.
  fireEvent.keyDown(document.querySelector('#queue-sheet') as HTMLElement, { key: 'Escape' });
  expect(onCloseSheet).toHaveBeenCalledTimes(5);
  fireEvent.keyDown(document.querySelector('#queue-sheet') as HTMLElement, { key: 'a' });
  expect(onCloseSheet).toHaveBeenCalledTimes(5);
  open.unmount();

  // Closed: no scrim, and the close control leaves the tab order.
  const closed = render(
    <QueuePane
      messages={messages}
      playback={{ ...queuedSnapshot(), queueOpen: false }}
      compactSheet
      onCloseSheet={onCloseSheet}
    />,
  );
  expect(document.querySelector('#queue-scrim')).toBeNull();
  expect(screen.getByRole('button', { name: 'Close queue', hidden: true }).getAttribute('tabindex')).toStrictEqual(
    '-1',
  );
  expect([...(document.querySelector('[data-queue-layer="1"]')?.children ?? [])].map((node) => node.id)).toStrictEqual([
    'queue-sheet',
  ]);
  closed.unmount();

  // Unwired, every way of closing is inert rather than an error.
  const unwired = render(<QueuePane messages={messages} playback={queuedSnapshot()} compactSheet />);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: 'Enter' });
  fireEvent.click(document.querySelector('#queue-scrim') as HTMLElement);
  fireEvent.keyDown(document.querySelector('#queue-sheet') as HTMLElement, { key: 'Escape' });
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  expect(onCloseSheet).toHaveBeenCalledTimes(5);
  unwired.unmount();

  // The wide pane is not a sheet: Escape is not its business.
  const pane = render(
    <QueuePane messages={messages} playback={queuedSnapshot()} compactSheet={false} onCloseSheet={onCloseSheet} />,
  );
  fireEvent.keyDown(document.querySelector('#right-pane') as HTMLElement, { key: 'Escape' });
  expect(onCloseSheet).toHaveBeenCalledTimes(5);
  expect(document.querySelector('[data-queue-layer="1"]')).toBeNull();
  pane.unmount();
});
