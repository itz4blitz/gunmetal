// The one place the demo touches a real HTMLAudioElement (CP-002 ground
// rule: browser interfaces live in dumb adapters with no logic).
export function createAudioElement(): HTMLAudioElement {
  const element = new Audio();
  element.preload = 'auto';
  return element;
}

export type OutputDevice = { id: string; label: string };

/**
 * Audio outputs the browser will name. No preference: an entry stays only
 * when its kind is `audiooutput`, and an entry the browser left unnamed
 * falls back to its place in the list (`Output 1`, `Output 2`, ...). No
 * mediaDevices means there is nothing to list.
 */
export async function listOutputDevices(): Promise<OutputDevice[]> {
  const mediaDevices = navigator.mediaDevices as MediaDevices | undefined;
  if (mediaDevices === undefined) {
    return [];
  }
  const devices = await mediaDevices.enumerateDevices();
  return devices
    .filter((device) => device.kind === 'audiooutput')
    .map((device, index) => ({
      id: device.deviceId,
      label: device.label === '' ? `Output ${index + 1}` : device.label,
    }));
}
