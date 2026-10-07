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
  fireEvent.click(screen.getByRole('button', { name: 'Close queue' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: 'Tab' });
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  view.unmount();

  const empty = render(<QueuePane messages={messages} playback={emptySnapshot()} compactSheet={false} />);
  expect(screen.getByText('Queue is empty').id).toStrictEqual('queue-empty');
  expect(document.querySelector('#queue-empty')?.getAttribute('data-empty-state')).toStrictEqual('queue');
  expect(document.querySelector('#right-pane [data-empty-title="1"]')?.textContent).toStrictEqual('Queue');
  expect(document.querySelector('#right-pane [data-empty-mark="1"]')).toBeTruthy();
  expect(document.querySelector('#right-pane')).toBeTruthy();
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
