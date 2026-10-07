import { expect, test } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// The generated artwork under apps/demo/public/media is the demo label's
// face (see generate.mjs for the four-pass composition). These structural
// assertions keep every cover a self-contained, text-free, seeded piece
// under the size budget, and keep the six composition families distributed
// so neighbouring sleeves on a shelf never read as twins.
const mediaRoot = join(dirname(fileURLToPath(import.meta.url)), '..', 'public', 'media');
const coverFiles = readdirSync(join(mediaRoot, 'covers')).sort();

test('every cover is self-contained layered art under the size budget', () => {
  expect(coverFiles).toHaveLength(15);
  for (const file of coverFiles) {
    const path = join(mediaRoot, 'covers', file);
    const svg = readFileSync(path, 'utf8');
    expect(statSync(path).size).toBeLessThan(40 * 1024);
    expect(svg.startsWith('<svg')).toBe(true);
    expect(svg).toContain('feTurbulence'); // grain pass: printed stock, not CSS
    expect(svg).toContain('mix-blend-mode'); // mesh base blends, not flat fills
    expect(svg).toMatch(/data-family="[a-z]+"/); // composition identity marker
    expect(new Set(svg.match(/#[0-9a-f]{6}/g)).size).toBeGreaterThanOrEqual(6); // layered colour
    expect(svg).not.toContain('<image'); // no rasters
    expect(svg).not.toContain('<text'); // LIB-146: badges are data, never baked in
    expect(svg).not.toContain('font-family');
    expect(/href="http/.test(svg)).toBe(false); // same-origin only, no network refs
  }
});

test('shelf neighbours never share a composition family and all six stay in print', () => {
  const families = coverFiles.map(
    (file) => readFileSync(join(mediaRoot, 'covers', file), 'utf8').match(/data-family="([a-z]+)"/)?.[1],
  );
  expect(new Set(families).size).toBe(6);
  for (let i = 1; i < families.length; i += 1) {
    expect(families[i]).not.toBe(families[i - 1]);
  }
});

test('artist images keep the nut motif, grain pass and a single stamped initial', () => {
  const files = readdirSync(join(mediaRoot, 'artists')).sort();
  expect(files.length).toBeGreaterThanOrEqual(10);
  for (const file of files) {
    const path = join(mediaRoot, 'artists', file);
    const svg = readFileSync(path, 'utf8');
    expect(statSync(path).size).toBeLessThan(40 * 1024);
    expect(svg).toContain('feTurbulence'); // grain pass
    expect(svg.match(/<polygon/g)?.length ?? 0).toBeGreaterThanOrEqual(4); // nut, bevel, chamfers
    expect(svg.match(/<circle/g)?.length ?? 0).toBeGreaterThanOrEqual(4); // rings, bore
    const stamps = svg.match(/<text[^>]*>([^<]*)<\/text>/);
    expect(stamps?.[1]).toMatch(/^[A-Z]$/); // exactly one stamped initial
    expect(svg).not.toContain('<image');
    expect(/href="http/.test(svg)).toBe(false);
  }
});
