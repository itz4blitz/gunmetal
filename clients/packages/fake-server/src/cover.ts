/**
 * Solid colour cover placeholder as a data-URI SVG.
 * Colours are generated here for the SVG bytes; UI tiles also use CSS data-cover tones.
 */
export function coverDataUri(tone: string, label: string): string {
  const fill = toneFill(tone);
  const safe = escapeXml(label);
  const svg = [
    '<svg xmlns="http://www.w3.org/2000/svg" width="320" height="320" viewBox="0 0 320 320">',
    `<rect width="320" height="320" fill="${fill}"/>`,
    `<text x="24" y="292" fill="#e9eef2" font-size="18" font-family="system-ui,sans-serif">${safe}</text>`,
    '</svg>',
  ].join('');
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function toneFill(tone: string): string {
  if (tone === '01') {
    return '#3a5a6e';
  }
  if (tone === '02') {
    return '#5c4a3a';
  }
  if (tone === '03') {
    return '#2f4f4f';
  }
  if (tone === '04') {
    return '#4a3f5c';
  }
  if (tone === '05') {
    return '#3f4a32';
  }
  if (tone === '06') {
    return '#5a3a3a';
  }
  if (tone === '07') {
    return '#2a3a4a';
  }
  return '#1f262d';
}

function escapeXml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&apos;');
}
