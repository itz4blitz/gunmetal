// Absolute URLs planted in a bundle. The production build is scanned the same way: a match fails the gate.

export function absoluteUrls(source: string): string[] {
  const matches = source.match(/https?:\/\/[^\s"'`\\]+/g);
  if (matches === null) {
    return [];
  }
  return matches;
}
