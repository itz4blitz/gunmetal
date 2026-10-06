// The canary's browser adapter: the one place the browser's storage estimate is read. An adapter holds
// no branch, loop or arithmetic, so it passes its arguments to the browser and returns what comes back.
// It is outside the covered set: jsdom has no such interface, and it is proved in real browsers instead.
export function storageEstimate(): Promise<StorageEstimate> {
  return navigator.storage.estimate();
}
