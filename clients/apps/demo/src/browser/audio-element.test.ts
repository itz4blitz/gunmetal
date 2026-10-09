import { expect, test } from 'vitest';
import { listOutputDevices } from './audio-element.ts';

function restoreMediaDevices(): void {
  Reflect.deleteProperty(navigator, 'mediaDevices');
}

test('missing mediaDevices is an empty output list', async () => {
  Object.defineProperty(navigator, 'mediaDevices', { configurable: true, value: undefined });
  try {
    expect(await listOutputDevices()).toStrictEqual([]);
  } finally {
    restoreMediaDevices();
  }
});

test('only audiooutput entries are kept, with the id and label the browser gave', async () => {
  Object.defineProperty(navigator, 'mediaDevices', {
    configurable: true,
    value: {
      enumerateDevices: () =>
        Promise.resolve([
          { kind: 'audioinput', deviceId: 'mic', label: 'Mic', groupId: 'g' },
          { kind: 'audiooutput', deviceId: 'speakers', label: 'Studio speakers', groupId: 'g' },
          { kind: 'videoinput', deviceId: 'cam', label: 'Camera', groupId: 'g' },
          { kind: 'audiooutput', deviceId: 'quiet', label: '', groupId: 'g' },
        ]),
    },
  });
  try {
    expect(await listOutputDevices()).toStrictEqual([
      { id: 'speakers', label: 'Studio speakers' },
      { id: 'quiet', label: '' },
    ]);
  } finally {
    restoreMediaDevices();
  }
});
