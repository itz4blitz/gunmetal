import { cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { Text, View } from 'react-native-web';
import { transportKeyAction, useTransportKeys, type TransportKeyHandlers } from './use-transport-keys.ts';

afterEach(cleanup);

function Harness({ handlers }: { handlers: TransportKeyHandlers }) {
  useTransportKeys(handlers);
  return (
    <View>
      <Text>transport keys harness</Text>
    </View>
  );
}

test('space plays and pauses, arrows seek, enter skips forward — nothing else', () => {
  const calls: string[] = [];
  render(
    <Harness
      handlers={{
        onPlayPause: () => {
          calls.push('play-pause');
        },
        onNext: () => {
          calls.push('next');
        },
        onSeekBy: (delta) => {
          calls.push(`seek ${delta}`);
        },
      }}
    />,
  );
  fireEvent.keyDown(document.body, { key: ' ' });
  fireEvent.keyDown(document.body, { key: 'Enter' });
  fireEvent.keyDown(document.body, { key: 'ArrowRight' });
  fireEvent.keyDown(document.body, { key: 'ArrowLeft' });
  fireEvent.keyDown(document.body, { key: 'a' });
  fireEvent.keyDown(document.body, { key: 'Escape' });
  expect(calls).toStrictEqual(['play-pause', 'next', 'seek 5000', 'seek -5000']);
});

test('keystrokes that belong to a field or a focused control never reach the transport', () => {
  const onPlayPause = vi.fn();
  render(
    <View>
      <input aria-label="Search field" />
      <View accessibilityRole="button" accessibilityLabel="A player control" tabIndex={0}>
        <Text>control</Text>
      </View>
      <div contentEditable={true} />
      <Harness handlers={{ onPlayPause }} />
    </View>,
  );
  fireEvent.keyDown(document.querySelector('input') as HTMLElement, { key: ' ' });
  fireEvent.keyDown(document.querySelector('input') as HTMLElement, { key: 'ArrowRight' });
  fireEvent.keyDown(document.querySelector('[role="button"]') as HTMLElement, { key: ' ' });
  fireEvent.keyDown(document.querySelector('[contenteditable="true"]') as HTMLElement, { key: 'Enter' });
  expect(onPlayPause).toHaveBeenCalledTimes(0);
  // Away from any control, the same key is the transport's.
  fireEvent.keyDown(document.body, { key: ' ' });
  expect(onPlayPause).toHaveBeenCalledTimes(1);
});

test('an event already consumed by composition is left alone', () => {
  const onPlayPause = vi.fn();
  render(<Harness handlers={{ onPlayPause }} />);
  fireEvent.keyDown(document.body, { key: ' ', isComposing: true });
  expect(onPlayPause).toHaveBeenCalledTimes(0);
});

test('a handled key is consumed; the rest pass through; unwired actions stay quiet', () => {
  const calls: string[] = [];
  render(
    <Harness
      handlers={{
        onSeekBy: (delta) => {
          calls.push(`seek ${delta}`);
        },
      }}
    />,
  );
  const consumed = fireEvent.keyDown(document.body, { key: 'ArrowRight' });
  expect(consumed).toStrictEqual(false);
  const passed = fireEvent.keyDown(document.body, { key: 'Tab' });
  expect(passed).toStrictEqual(true);
  fireEvent.keyDown(document.body, { key: 'ArrowLeft' });
  fireEvent.keyDown(document.body, { key: 'Enter' });
  fireEvent.keyDown(document.body, { key: ' ' });
  expect(calls).toStrictEqual(['seek 5000', 'seek -5000']);
});

test('the key table is the one the transport and tests share', () => {
  expect(transportKeyAction(' ')).toStrictEqual('play-pause');
  expect(transportKeyAction('Enter')).toStrictEqual('next');
  expect(transportKeyAction('ArrowRight')).toStrictEqual('seek-forward');
  expect(transportKeyAction('ArrowLeft')).toStrictEqual('seek-back');
  expect(transportKeyAction('Home')).toStrictEqual(undefined);
});
