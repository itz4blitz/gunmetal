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
      { id: 'quiet', label: 'Output 2' },
    ]);
  } finally {
    restoreMediaDevices();
  }
});

test('an unnamed output falls back to its place among the audio outputs', async () => {
  Object.defineProperty(navigator, 'mediaDevices', {
    configurable: true,
    value: {
      enumerateDevices: () =>
        Promise.resolve([
          { kind: 'audiooutput', deviceId: 'first', label: '', groupId: 'g' },
          { kind: 'audioinput', deviceId: 'mic', label: '', groupId: 'g' },
          { kind: 'audiooutput', deviceId: 'second', label: '', groupId: 'g' },
          { kind: 'audiooutput', deviceId: 'named', label: 'Headphones', groupId: 'g' },
        ]),
    },
  });
  try {
    expect(await listOutputDevices()).toStrictEqual([
      { id: 'first', label: 'Output 1' },
      { id: 'second', label: 'Output 2' },
      { id: 'named', label: 'Headphones' },
    ]);
  } finally {
    restoreMediaDevices();
  }
});
