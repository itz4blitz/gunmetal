import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { catalogue } from '../messages/catalogue.ts';
import { emptyPlayback, playbackFromAlbum } from './playback.ts';
import { QueuePane } from './QueuePane.tsx';
import { demoLibrary } from '../../../fake-server/src/catalogue.ts';
import { findAlbum } from './playback.ts';

afterEach(cleanup);

test('queue pane sheet close is optional and ignores non-activation keys', () => {
  const messages = catalogue().shell;
  const library = demoLibrary();
  const playback = playbackFromAlbum(findAlbum(library, 'demo-album-01')!);
  const view = render(
    <QueuePane messages={messages} playback={playback} compactSheet />,
  );
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  fireEvent.click(screen.getByRole('button', { name: 'Close queue' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Close queue' }), { key: 'Tab' });
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('1');
  view.unmount();

  const empty = render(
    <QueuePane messages={messages} playback={emptyPlayback()} compactSheet={false} />,
  );
  expect(screen.getByText('Queue is empty').id).toStrictEqual('queue-empty');
  expect(document.querySelector('#queue-empty')?.getAttribute('data-empty-state')).toStrictEqual(
    'queue',
  );
  expect(document.querySelector('#right-pane [data-empty-title="1"]')?.textContent).toStrictEqual(
    'Queue',
  );
  expect(document.querySelector('#right-pane [data-empty-mark="1"]')).toBeTruthy();
  expect(document.querySelector('#right-pane')).toBeTruthy();
  empty.unmount();

  const closed = render(
    <QueuePane messages={messages} playback={{ ...playback, queueOpen: false }} compactSheet />,
  );
  expect(document.querySelector('#queue-sheet')?.getAttribute('data-queue-open')).toStrictEqual('0');
  closed.unmount();
});
