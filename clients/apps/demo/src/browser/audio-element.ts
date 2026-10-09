// The one place the demo touches a real HTMLAudioElement (CP-002 ground
// rule: browser interfaces live in dumb adapters with no logic).
export function createAudioElement(): HTMLAudioElement {
  const element = new Audio();
  element.preload = 'auto';
  return element;
}

export type OutputDevice = { id: string; label: string };

/**
 * Audio outputs the browser will name. No preference and no invented label:
 * an entry stays only when its kind is `audiooutput`. No mediaDevices means
 * there is nothing to list.
 */
export async function listOutputDevices(): Promise<OutputDevice[]> {
  const mediaDevices = navigator.mediaDevices as MediaDevices | undefined;
  if (mediaDevices === undefined) {
    return [];
  }
  const devices = await mediaDevices.enumerateDevices();
  return devices
    .filter((device) => device.kind === 'audiooutput')
    .map((device) => ({ id: device.deviceId, label: device.label }));
}
