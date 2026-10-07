// The one place the demo touches a real HTMLAudioElement (CP-002 ground
// rule: browser interfaces live in dumb adapters with no logic).
export function createAudioElement(): HTMLAudioElement {
  const element = new Audio();
  element.preload = 'auto';
  return element;
}
