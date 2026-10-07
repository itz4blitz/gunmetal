// Generates the demo's media: per-album WAV tones, per-album SVG covers and
// per-artist SVG images. Checked-in script, deterministic output, no deps.
// CP-011 formalises the SHA-256 manifest; until then `node generate.mjs`
// must be run after any change here and its output committed.
// Audio is placeholder tone (design-language: honest fixtures, not fake art).
//
// Covers are generative printed art, not CSS gradients. Every cover layers
// the same four passes so the set reads as one label:
//   1. mesh base — three soft radial fields in neighbouring hues, blended
//      soft-light / screen / multiply over an angled tone gradient,
//   2. one signature composition family (six: arcs, bars, grid, topo, dots,
//      orbits), picked by album index with a coprime stride so shelf
//      neighbours never repeat a family,
//   3. print passes — low-frequency mottle plus high-frequency grain
//      (feTurbulence) so it reads as stock rather than markup,
//   4. machined frame — hairline border, brass corner ticks, index mark.
// All randomness comes from mulberry32 seeded by the album index: same seed,
// byte-identical file.
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));

// Same families as the cover-tone tokens in shell.css.
const palettes = {
  'demo-album-01': { hue: 205, deep: '#243844', mid: '#3a5a6e', hi: '#6a92a8', ink: '#e9eef2' },
  'demo-album-02': { hue: 30, deep: '#3a2e24', mid: '#5c4a3a', hi: '#8a7058', ink: '#f2ece4' },
  'demo-album-03': { hue: 175, deep: '#1e3434', mid: '#2f4f4f', hi: '#4e7c7c', ink: '#e4efee' },
  'demo-album-04': { hue: 268, deep: '#322a3e', mid: '#4a3f5c', hi: '#76648e', ink: '#ece8f4' },
  'demo-album-05': { hue: 88, deep: '#2a3220', mid: '#3f4a32', hi: '#6a7c4e', ink: '#ecf1e6' },
  'demo-album-06': { hue: 356, deep: '#3a2424', mid: '#5a3a3a', hi: '#8a5c5c', ink: '#f4e9e9' },
  'demo-album-07': { hue: 215, deep: '#1a2634', mid: '#2a3a4a', hi: '#4a6680', ink: '#e7edf4' },
  'demo-album-08': { hue: 210, deep: '#151a1f', mid: '#1f262d', hi: '#3a4650', ink: '#e9eef2' },
  'demo-album-09': { hue: 165, deep: '#1c302e', mid: '#2e4b48', hi: '#587874', ink: '#e6efed' },
  // Real CC albums (10-15): same muted steel/deep-warm family, one hue per
  // album, no pure saturation, no purple-gradient territory.
  'demo-album-10': { hue: 148, deep: '#20362e', mid: '#345448', hi: '#5f8375', ink: '#e8f0ec' },
  'demo-album-11': { hue: 20, deep: '#38291f', mid: '#5a4433', hi: '#8a6b52', ink: '#f2ebe3' },
  'demo-album-12': { hue: 190, deep: '#16303a', mid: '#26495a', hi: '#4f7d92', ink: '#e6eff3' },
  'demo-album-13': { hue: 60, deep: '#33331f', mid: '#545433', hi: '#7f7f52', ink: '#f1f1e6' },
  'demo-album-14': { hue: 330, deep: '#382430', mid: '#5a3f4c', hi: '#8a6478', ink: '#f3eaef' },
  'demo-album-15': { hue: 245, deep: '#232a3e', mid: '#3a4460', hi: '#647199', ink: '#eaedf4' },
};

// —— deterministic randomness ————————————————————————————————
// mulberry32: tiny, fast, stable across runs and Node versions. Every visual
// decision below draws from this stream; nothing reads the clock or Math.random.
function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const between = (rng, lo, hi) => lo + rng() * (hi - lo);
// Fixed-decimal formatting keeps output byte-stable and files compact;
// "-0.00" never appears because a zero result drops its sign.
const f = (n, d = 2) => {
  const s = n.toFixed(d);
  return s.startsWith('-') && Number(s) === 0 ? s.slice(1) : s;
};

// —— colour helpers (hex <-> HSL, muted re-tuning) ———————————
function hexToHsl(hex) {
  const n = parseInt(hex.slice(1), 16);
  const r = ((n >> 16) & 255) / 255;
  const g = ((n >> 8) & 255) / 255;
  const b = (n & 255) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return { h: 0, s: 0, l };
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h;
  if (max === r) h = (g - b) / d + (g < b ? 6 : 0);
  else if (max === g) h = (b - r) / d + 2;
  else h = (r - g) / d + 4;
  return { h: h * 60, s, l };
}
function hslToHex(h, s, l) {
  h = ((h % 360) + 360) % 360;
  s = Math.min(1, Math.max(0, s));
  l = Math.min(1, Math.max(0, l));
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  let r = 0;
  let g = 0;
  let b = 0;
  if (h < 60) [r, g, b] = [c, x, 0];
  else if (h < 120) [r, g, b] = [x, c, 0];
  else if (h < 180) [r, g, b] = [0, c, x];
  else if (h < 240) [r, g, b] = [0, x, c];
  else if (h < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  const to = (v) =>
    Math.round((v + m) * 255)
      .toString(16)
      .padStart(2, '0');
  return `#${to(r)}${to(g)}${to(b)}`;
}
// Re-tune a palette colour: hue drift, saturation scale, lightness nudge.
const tone = (hex, dh, ds, dl) => {
  const { h, s, l } = hexToHsl(hex);
  return hslToHex(h + dh, s * ds, l + dl);
};

// —— SVG covers ——————————————————————————————————————————————
// No text in the art (LIB-146: badges are data, never baked into posters).

// Family 0 · concentric arc system: a dial of tapered rings — every third
// broken into a major arc — bleeding past the frame, one brass survey ring.
function arcsLayer(x) {
  const { rng, p, C } = x;
  const cx = between(rng, 240, 400);
  const cy = between(rng, 230, 370);
  const count = 7 + Math.floor(rng() * 4);
  const r0 = between(rng, 50, 92);
  const step = Math.min(between(rng, 26, 38), (430 - 92) / count);
  const phase = between(rng, 0, Math.PI * 2);
  const rot = between(rng, 0, 360);
  const rings = [];
  for (let i = 0; i < count; i += 1) {
    const r = r0 + i * step;
    const w = 1.5 + 9 * (0.5 + 0.5 * Math.sin(i * 0.85 + phase));
    const stroke = i % 3 === 0 ? C.glow : i % 3 === 1 ? p.hi : p.mid;
    const dash =
      i % 3 === 2
        ? ` stroke-dasharray="${f(2 * Math.PI * r * between(rng, 0.42, 0.62), 1)} ${f(2 * Math.PI * r, 1)}"`
        : '';
    rings.push(
      `<circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="${f(r, 1)}" fill="none" stroke="${stroke}" stroke-width="${f(w, 1)}" opacity="${f(0.9 - i * 0.055)}"${dash} transform="rotate(${f(rot + i * between(rng, -7, 7), 1)} ${f(cx, 1)} ${f(cy, 1)})"/>`,
    );
  }
  const dial = `<circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="${f(r0 + count * step + 10, 1)}" fill="none" stroke="${C.brass}" stroke-width="1.5" stroke-dasharray="2 9" opacity="0.6"/>`;
  const aDot = between(rng, 0, Math.PI * 2);
  const rDot = r0 + 2 * step;
  const dx = cx + Math.cos(aDot) * rDot;
  const dy = cy + Math.sin(aDot) * rDot;
  const mark = `<circle cx="${f(dx, 1)}" cy="${f(dy, 1)}" r="7" fill="${C.glow}"/><circle cx="${f(dx, 1)}" cy="${f(dy, 1)}" r="13" fill="none" stroke="${C.glow}" stroke-width="1.5" opacity="0.5"/>`;
  return rings.join('') + dial + mark;
}

// Family 1 · staggered bar field: an equalizer frozen mid-breath — full
// columns anchored alternately to the top and bottom rails, brass every 4th.
function barsLayer(x) {
  const { rng, p, C } = x;
  const margin = 58;
  const count = 20 + Math.floor(rng() * 7);
  const gap = between(rng, 5, 8);
  const width = (640 - margin * 2 - gap * (count - 1)) / count;
  const f1 = between(rng, 0.4, 0.8);
  const f2 = between(rng, 1.2, 2.1);
  const p1 = between(rng, 0, Math.PI * 2);
  const p2 = between(rng, 0, Math.PI * 2);
  const block = 3 + Math.floor(rng() * 4);
  const flip = rng() < 0.5;
  const bars = [];
  for (let i = 0; i < count; i += 1) {
    const bx = margin + i * (width + gap);
    const t = count > 1 ? i / (count - 1) : 0;
    const swell = 0.5 + 0.5 * Math.sin(t * Math.PI * f1 + p1);
    const chop = 0.6 + 0.4 * Math.sin(t * Math.PI * f2 + p2);
    const h = Math.max(96, 128 + 240 * swell * chop + between(rng, -14, 14));
    const up = (Math.floor(i / block) + (flip ? 1 : 0)) % 2 === 0;
    const by = up ? margin : 640 - margin - h;
    const fill = i % 4 === 3 ? C.brass : i % 2 === 0 ? p.hi : p.mid;
    bars.push(
      `<rect x="${f(bx, 1)}" y="${f(by, 1)}" width="${f(width, 1)}" height="${f(h, 1)}" rx="2.5" fill="${fill}" opacity="${f(0.45 + 0.35 * (h / 380))}"/>`,
    );
  }
  const rail = f(640 - margin * 2, 1);
  const rails = `<rect x="${margin}" y="${f(margin - 9.5, 1)}" width="${rail}" height="1.5" fill="${p.hi}" opacity="0.35"/><rect x="${margin}" y="${f(640 - margin + 8, 1)}" width="${rail}" height="1.5" fill="${p.hi}" opacity="0.35"/>`;
  return bars.join('') + rails;
}

// Family 2 · fine line grids: two crossed hairline meshes clipped to a disc,
// a brass cardinal line and a dashed inner ring.
function gridLayer(x) {
  const { rng, p, C, u } = x;
  const cx = between(rng, 270, 370);
  const cy = between(rng, 270, 370);
  const R = between(rng, 250, 315);
  const a1 = between(rng, -24, 24);
  const s1 = between(rng, 15, 21);
  const a2 = a1 + between(rng, 55, 125);
  const s2 = between(rng, 27, 36);
  const mesh = (spacing, angle, stroke, opacity) => {
    const out = [];
    const span = R + 12;
    for (let off = -span; off <= span; off += spacing) {
      out.push(`<line x1="${f(cx + off, 1)}" y1="${f(cy - span, 1)}" x2="${f(cx + off, 1)}" y2="${f(cy + span, 1)}"/>`);
    }
    return `<g stroke="${stroke}" stroke-width="1" opacity="${f(opacity)}" transform="rotate(${f(angle, 1)} ${f(cx, 1)} ${f(cy, 1)})">${out.join('')}</g>`;
  };
  const cardinal = `<line x1="${f(cx, 1)}" y1="${f(cy - R, 1)}" x2="${f(cx, 1)}" y2="${f(cy + R, 1)}" stroke="${C.brass}" stroke-width="2" opacity="0.7" transform="rotate(${f(a1 + 90, 1)} ${f(cx, 1)} ${f(cy, 1)})"/>`;
  const outline = `<circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="${f(R, 1)}" fill="none" stroke="${p.hi}" stroke-width="1.5" opacity="0.55"/><circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="${f(R * 0.64, 1)}" fill="none" stroke="${p.hi}" stroke-width="1" stroke-dasharray="3 7" opacity="0.45"/>`;
  return `<clipPath id="${u}disc"><circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="${f(R, 1)}"/></clipPath><g clip-path="url(#${u}disc)">${mesh(s1, a1, p.mid, 0.55)}${mesh(s2, a2, C.glow, 0.4)}${cardinal}</g>${outline}`;
}

// Family 3 · topographic contours: nested wobbly rings around a summit that
// drifts as it climbs, one contour picked out in brass.
function topoLayer(x) {
  const { rng, p, C } = x;
  const cx = between(rng, 260, 380);
  const cy = between(rng, 260, 380);
  const count = 8 + Math.floor(rng() * 4);
  const base = between(rng, 30, 50);
  const step = between(rng, 24, 32);
  const amp1 = between(rng, 10, 18);
  const amp2 = between(rng, 14, 26);
  const amp3 = between(rng, 5, 10);
  const ph1 = between(rng, 0, Math.PI * 2);
  const ph2 = between(rng, 0, Math.PI * 2);
  const ph3 = between(rng, 0, Math.PI * 2);
  const ph4 = between(rng, 0, Math.PI * 2);
  const driftA = between(rng, 0, Math.PI * 2);
  const driftD = between(rng, 2.2, 4.2);
  const highlight = 1 + Math.floor(rng() * (count - 2));
  const K = 84;
  const rings = [];
  for (let k = 0; k < count; k += 1) {
    const R = base + k * step;
    const scale = 0.5 + 0.5 * (k / Math.max(1, count - 1));
    const amp = Math.min(step * 0.6, (amp1 + amp2 + amp3) * scale * 0.62);
    const ox = cx + Math.cos(driftA) * driftD * k;
    const oy = cy + Math.sin(driftA) * driftD * k;
    const pts = [];
    for (let j = 0; j < K; j += 1) {
      const th = (j * 2 * Math.PI) / K;
      const w =
        Math.sin(2 * th + ph1) * 0.4 +
        Math.sin(3 * th + ph2) * 0.32 +
        Math.sin(5 * th + ph3) * 0.18 +
        Math.sin(7 * th + ph4) * 0.1;
      const r = R + amp * w;
      pts.push(`${f(ox + Math.cos(th) * r, 1)},${f(oy + Math.sin(th) * r, 1)}`);
    }
    const isHi = k === highlight;
    // Dark presses swallow mid-tone ink: lift stroke lightness inside the
    // palette hue so contours stay readable on the deepest sleeves.
    const stroke = isHi ? C.brass : k % 2 === 0 ? tone(p.hi, 0, 1, 0.05) : tone(p.mid, 0, 1, 0.12);
    rings.push(
      `<polygon points="${pts.join(' ')}" fill="none" stroke="${stroke}" stroke-width="${isHi ? 2.2 : 1.4}" opacity="${f(isHi ? 0.9 : Math.max(0.42, 0.78 - k * 0.03))}"/>`,
    );
  }
  const mark = `<circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="4" fill="${C.glow}"/><circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="10" fill="none" stroke="${C.brass}" stroke-width="1.5" stroke-dasharray="2 5" opacity="0.7"/><line x1="${f(cx - 16, 1)}" y1="${f(cy, 1)}" x2="${f(cx - 8, 1)}" y2="${f(cy, 1)}" stroke="${C.glow}" stroke-width="1.5"/><line x1="${f(cx + 8, 1)}" y1="${f(cy, 1)}" x2="${f(cx + 16, 1)}" y2="${f(cy, 1)}" stroke="${C.glow}" stroke-width="1.5"/>`;
  return rings.join('') + mark;
}

// Family 4 · dot-matrix halftone: a dot grid whose radius follows a wave and
// a seeded focus, fading out where the plate would go blank.
function dotsLayer(x) {
  const { rng, p, C } = x;
  const spacing = between(rng, 25, 31);
  const cols = Math.floor((640 - 72) / spacing);
  const rows = cols;
  const ox = (640 - (cols - 1) * spacing) / 2;
  const oy = ox;
  const fx = between(rng, 0.018, 0.04);
  const fy = between(rng, 0.018, 0.04);
  const p1 = between(rng, 0, Math.PI * 2);
  const p2 = between(rng, 0, Math.PI * 2);
  const focusX = between(rng, 180, 460);
  const focusY = between(rng, 180, 460);
  const maxR = spacing * 0.44;
  const candidates = [];
  for (let gy = 0; gy < rows; gy += 1) {
    for (let gx = 0; gx < cols; gx += 1) {
      const cx = ox + gx * spacing;
      const cy = oy + gy * spacing;
      const wave = 0.5 + 0.5 * Math.sin(cx * fx + p1) * Math.sin(cy * fy + p2);
      const fall = Math.max(0, 1 - Math.hypot(cx - focusX, cy - focusY) / 430);
      const v = wave * 0.5 + fall * 0.5;
      candidates.push({ cx, cy, r: maxR * Math.pow(v, 2.2), v });
    }
  }
  // Deterministic density cap: keep the 320 largest so files stay light.
  const radii = candidates.map((c) => c.r).sort((a, b) => b - a);
  const cut = radii.length > 320 ? radii[319] : 0;
  const dots = [];
  for (const c of candidates) {
    if (c.r < Math.max(cut, 1.4)) continue;
    const hot = c.r > maxR * 0.7;
    dots.push(
      `<circle cx="${f(c.cx, 1)}" cy="${f(c.cy, 1)}" r="${f(c.r, 1)}" fill="${hot ? C.glow : p.hi}" opacity="${f(0.3 + c.v * 0.5)}"/>`,
    );
  }
  return `<g>${dots.join('')}</g>`;
}

// Family 5 · orbit rings: crossed ellipses around a lit core, satellites
// pinned on the paths, one dashed orbit, a brass survey ring.
function orbitsLayer(x) {
  const { rng, p, C, u } = x;
  const cx = between(rng, 275, 365);
  const cy = between(rng, 260, 350);
  const count = 3 + Math.floor(rng() * 3);
  const coreR = between(rng, 40, 64);
  const dashed = 1 + Math.floor(rng() * Math.max(1, count - 1));
  const parts = [];
  for (let k = 0; k < count; k += 1) {
    const rx = Math.min(coreR + 96 + k * between(rng, 50, 66), 332);
    const ry = rx * between(rng, 0.32, 0.56);
    const rot = between(rng, 0, 180);
    const stroke = k % 2 === 0 ? p.hi : p.mid;
    const dash = k === dashed ? ` stroke-dasharray="${f(between(rng, 10, 24), 1)} ${f(between(rng, 7, 15), 1)}"` : '';
    parts.push(
      `<ellipse cx="${f(cx, 1)}" cy="${f(cy, 1)}" rx="${f(rx, 1)}" ry="${f(ry, 1)}" fill="none" stroke="${stroke}" stroke-width="${k % 3 === 0 ? 2.4 : 1.6}" opacity="${f(0.85 - k * 0.07)}"${dash} transform="rotate(${f(rot, 1)} ${f(cx, 1)} ${f(cy, 1)})"/>`,
    );
    const sats = 1 + (rng() < 0.5 ? 1 : 0);
    for (let s = 0; s < sats; s += 1) {
      const ang = between(rng, 0, Math.PI * 2);
      const ex = Math.cos(ang) * rx;
      const ey = Math.sin(ang) * ry;
      const rr = (rot * Math.PI) / 180;
      const sx = cx + ex * Math.cos(rr) - ey * Math.sin(rr);
      const sy = cy + ex * Math.sin(rr) + ey * Math.cos(rr);
      const r = between(rng, 6, 10.5);
      parts.push(
        `<circle cx="${f(sx, 1)}" cy="${f(sy, 1)}" r="${f(r, 1)}" fill="${C.glow}"/><circle cx="${f(sx, 1)}" cy="${f(sy, 1)}" r="${f(r + 6, 1)}" fill="none" stroke="${C.glow}" stroke-width="1.4" opacity="0.5"/>`,
      );
    }
  }
  const survey = Math.min(coreR + 96 + count * 58 + 18, 316);
  const brassRing = `<circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="${f(survey, 1)}" fill="none" stroke="${C.brass}" stroke-width="1.5" stroke-dasharray="1 8" opacity="0.55"/>`;
  const core = `<circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="${f(coreR * 2.4, 1)}" fill="url(#${u}core)"/><circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="${f(coreR, 1)}" fill="none" stroke="${p.hi}" stroke-width="2.5" opacity="0.85"/><circle cx="${f(cx, 1)}" cy="${f(cy, 1)}" r="4.5" fill="${C.glow}"/>`;
  return core + parts.join('') + brassRing;
}

const FAMILY_NAMES = ['arcs', 'bars', 'grid', 'topo', 'dots', 'orbits'];
const LAYERS = [arcsLayer, barsLayer, gridLayer, topoLayer, dotsLayer, orbitsLayer];

function coverSvg(index) {
  const id = Object.keys(palettes)[index];
  const p = palettes[id];
  const rng = mulberry32(index * 747796405 + 2891336453);
  const u = `c${String(index + 1).padStart(2, '0')}`;
  // Coprime stride 5 over 6 families: neighbours on the shelf never share a
  // composition, and all six families stay in print across the fifteen.
  const family = (index * 5 + 4) % 6;
  const dir = rng() < 0.5 ? 1 : -1;
  const C = {
    nbr1: tone(p.mid, dir * between(rng, 16, 30), 0.9, -0.05),
    nbr2: tone(p.hi, -dir * between(rng, 14, 26), 0.85, -0.12),
    glow: tone(p.hi, between(rng, -8, 8), 0.75, 0.04),
    brass: hslToHex(38 + between(rng, -7, 7), 0.3, 0.56),
  };
  const bg0 = tone(p.mid, 0, 1, -0.1);
  const bg1 = tone(p.deep, 0, 1, -0.02);
  const b1x = between(rng, 170, 470);
  const b1y = between(rng, 100, 280);
  const b1r = between(rng, 300, 430);
  const b2x = between(rng, 170, 470);
  const b2y = between(rng, 380, 560);
  const b2r = between(rng, 280, 400);
  const b3x = between(rng, 60, 580);
  const b3y = between(rng, 60, 580);
  const b3r = between(rng, 330, 460);
  // The sheen rotation is drawn from the stream to keep every later number
  // where it was; the sheen itself does not use it.
  between(rng, -40, 40);
  const grainFreq = between(rng, 0.78, 1.05);
  const grainSeed = 1 + Math.floor(rng() * 900);
  const grainOpacity = between(rng, 0.5, 0.68);
  const mottleSeed = 1 + Math.floor(rng() * 900);
  const ix = between(rng, 80, 526);
  const defs = `<defs>
<linearGradient id="${u}bg" x1="0" y1="0" x2="1" y2="1" gradientTransform="rotate(${f(between(rng, -30, 30), 1)} 0.5 0.5)"><stop offset="0" stop-color="${bg0}"/><stop offset="1" stop-color="${bg1}"/></linearGradient>
<radialGradient id="${u}b1" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="${C.nbr1}" stop-opacity="0.62"/><stop offset="0.6" stop-color="${C.nbr1}" stop-opacity="0.28"/><stop offset="1" stop-color="${C.nbr1}" stop-opacity="0"/></radialGradient>
<radialGradient id="${u}b2" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="${C.glow}" stop-opacity="0.56"/><stop offset="0.55" stop-color="${C.glow}" stop-opacity="0.22"/><stop offset="1" stop-color="${C.glow}" stop-opacity="0"/></radialGradient>
<radialGradient id="${u}b3" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="${C.nbr2}" stop-opacity="0.66"/><stop offset="0.6" stop-color="${C.nbr2}" stop-opacity="0.3"/><stop offset="1" stop-color="${C.nbr2}" stop-opacity="0"/></radialGradient>
<linearGradient id="${u}sheen" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ffffff" stop-opacity="0.13"/><stop offset="0.4" stop-color="#ffffff" stop-opacity="0"/><stop offset="1" stop-color="#000000" stop-opacity="0.14"/></linearGradient>
<radialGradient id="${u}vig" cx="0.5" cy="0.46" r="0.75"><stop offset="0.55" stop-color="${p.deep}" stop-opacity="0"/><stop offset="1" stop-color="${tone(p.deep, 0, 1, -0.04)}" stop-opacity="0.66"/></radialGradient>
<radialGradient id="${u}core" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="${C.glow}" stop-opacity="0.9"/><stop offset="0.45" stop-color="${C.glow}" stop-opacity="0.32"/><stop offset="1" stop-color="${C.glow}" stop-opacity="0"/></radialGradient>
<filter id="${u}mottle" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.011" numOctaves="3" seed="${mottleSeed}" stitchTiles="stitch"/><feColorMatrix type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0.5 0.5 0.5 0 -0.32"/></filter>
<filter id="${u}grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="${f(grainFreq, 3)}" numOctaves="2" seed="${grainSeed}" stitchTiles="stitch"/><feColorMatrix type="matrix" values="0 0 0 0 0.87 0 0 0 0 0.85 0 0 0 0 0.8 0.28 0.28 0.28 0 0"/></filter>
</defs>`;
  const mesh =
    `<rect width="640" height="640" fill="url(#${u}bg)"/>` +
    `<circle cx="${f(b1x, 1)}" cy="${f(b1y, 1)}" r="${f(b1r, 1)}" fill="url(#${u}b1)" style="mix-blend-mode:soft-light"/>` +
    `<circle cx="${f(b2x, 1)}" cy="${f(b2y, 1)}" r="${f(b2r, 1)}" fill="url(#${u}b2)" style="mix-blend-mode:screen" opacity="0.55"/>` +
    `<circle cx="${f(b3x, 1)}" cy="${f(b3y, 1)}" r="${f(b3r, 1)}" fill="url(#${u}b3)" style="mix-blend-mode:multiply" opacity="0.5"/>`;
  const layer = LAYERS[family]({ rng, p, C, u });
  const prints =
    `<rect width="640" height="640" fill="url(#${u}sheen)" style="mix-blend-mode:soft-light" opacity="0.7"/>` +
    `<rect width="640" height="640" filter="url(#${u}mottle)" style="mix-blend-mode:soft-light" opacity="0.42"/>` +
    `<rect width="640" height="640" filter="url(#${u}grain)" opacity="${f(grainOpacity)}"/>` +
    `<rect width="640" height="640" fill="url(#${u}vig)"/>`;
  const frame =
    `<rect x="24" y="24" width="592" height="592" fill="none" stroke="${p.hi}" stroke-opacity="0.4" stroke-width="1.5"/>` +
    `<path d="M24 48 V24 H48 M592 24 H616 V48 M616 592 V616 H592 M48 616 H24 V592" fill="none" stroke="${C.brass}" stroke-width="2.5" opacity="0.75"/>` +
    `<rect x="${f(ix, 1)}" y="614.5" width="34" height="3" fill="${C.brass}" opacity="0.8"/>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="640" viewBox="0 0 640 640" data-family="${FAMILY_NAMES[family]}" role="img">
${defs}
${mesh}
<g>${layer}</g>
${prints}
${frame}
</svg>
`;
}

// —— Artist images ———————————————————————————————————————————
// Hex-nut mark on a machined field, per design-language §1: ring system,
// nut with chamfer + bore, the initial stamped in the bore, grain pass.
function hexPoints(cx, cy, r, rotDeg) {
  const pts = [];
  for (let i = 0; i < 6; i += 1) {
    const a = ((rotDeg + i * 60) * Math.PI) / 180 - Math.PI / 2;
    pts.push(`${f(cx + Math.cos(a) * r)},${f(cy + Math.sin(a) * r)}`);
  }
  return pts.join(' ');
}

function artistSvg(seed, name) {
  const tones = Object.keys(palettes);
  const id = tones[seed % tones.length];
  const p = palettes[id];
  const rng = mulberry32(seed * 374761393 + 668265263);
  const u = `a${seed}`;
  const initial = name.trim().slice(0, 1).toUpperCase();
  const cx = 240;
  const cy = 234;
  // Alternating pointy/flat orientation plus a slight settle angle: the two
  // Alex Reed presses read as different stampings, not twins.
  const rot = (seed % 2 === 0 ? 0 : 30) + between(rng, -8, 8);
  const glow = tone(p.hi, between(rng, -8, 8), 0.75, 0.04);
  const brass = hslToHex(38 + between(rng, -6, 6), 0.3, 0.56);
  const dashMid = f(between(rng, 5, 12), 1);
  const grainSeed = 1 + Math.floor(rng() * 900);
  const defs = `<defs>
<linearGradient id="${u}bg" x1="0" y1="0" x2="1" y2="1" gradientTransform="rotate(${f(between(rng, -25, 25), 1)} 0.5 0.5)"><stop offset="0" stop-color="${tone(p.mid, 0, 1, -0.12)}"/><stop offset="1" stop-color="${p.deep}"/></linearGradient>
<radialGradient id="${u}glow" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="${glow}" stop-opacity="0.45"/><stop offset="0.6" stop-color="${glow}" stop-opacity="0.16"/><stop offset="1" stop-color="${glow}" stop-opacity="0"/></radialGradient>
<radialGradient id="${u}shadow" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="#000000" stop-opacity="0.42"/><stop offset="0.55" stop-color="#000000" stop-opacity="0.16"/><stop offset="1" stop-color="#000000" stop-opacity="0"/></radialGradient>
<linearGradient id="${u}nut" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${p.hi}"/><stop offset="1" stop-color="${tone(p.mid, 0, 1, -0.04)}"/></linearGradient>
<linearGradient id="${u}bevel" x1="0" y1="0" x2="0.7" y2="1"><stop offset="0" stop-color="#ffffff" stop-opacity="0.28"/><stop offset="0.5" stop-color="#ffffff" stop-opacity="0.04"/><stop offset="1" stop-color="#000000" stop-opacity="0.16"/></linearGradient>
<radialGradient id="${u}vig" cx="0.5" cy="0.47" r="0.8"><stop offset="0.6" stop-color="${p.deep}" stop-opacity="0"/><stop offset="1" stop-color="${tone(p.deep, 0, 1, -0.04)}" stop-opacity="0.55"/></radialGradient>
<filter id="${u}grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="${grainSeed}" stitchTiles="stitch"/><feColorMatrix type="matrix" values="0 0 0 0 0.87 0 0 0 0 0.85 0 0 0 0 0.8 0.28 0.28 0.28 0 0"/></filter>
</defs>`;
  const field =
    `<rect width="480" height="480" fill="url(#${u}bg)"/>` +
    `<circle cx="${cx}" cy="${f(cy - 70, 1)}" r="260" fill="url(#${u}glow)"/>`;
  const rings =
    `<circle cx="${cx}" cy="${cy}" r="224" fill="none" stroke="${brass}" stroke-width="1.5" stroke-dasharray="1 7" opacity="0.5"/>` +
    `<circle cx="${cx}" cy="${cy}" r="196" fill="none" stroke="${p.hi}" stroke-width="1.5" opacity="0.55"/>` +
    `<circle cx="${cx}" cy="${cy}" r="168" fill="none" stroke="${p.mid}" stroke-width="1" stroke-dasharray="${dashMid} 10" opacity="0.6"/>`;
  const nut =
    `<ellipse cx="${cx}" cy="${f(cy + 12, 1)}" rx="152" ry="146" fill="url(#${u}shadow)" opacity="0.55"/>` +
    `<g transform="rotate(${f(rot, 1)} ${cx} ${cy})">` +
    `<polygon points="${hexPoints(cx, cy, 118, 0)}" fill="url(#${u}nut)" stroke="${p.deep}" stroke-width="2.5"/>` +
    `<polygon points="${hexPoints(cx, cy, 118, 0)}" fill="url(#${u}bevel)"/>` +
    `<polygon points="${hexPoints(cx, cy, 112, 0)}" fill="none" stroke="${p.ink}" stroke-width="1" opacity="0.22"/>` +
    `<polygon points="${hexPoints(cx, cy, 104, 0)}" fill="none" stroke="${tone(p.deep, 0, 1, -0.06)}" stroke-width="1.5" opacity="0.6"/>` +
    `</g>` +
    `<circle cx="${cx}" cy="${cy}" r="58" fill="${p.deep}"/>` +
    `<circle cx="${cx}" cy="${cy}" r="58" fill="none" stroke="${p.hi}" stroke-width="3" opacity="0.8"/>` +
    `<circle cx="${cx}" cy="${cy}" r="52" fill="none" stroke="${p.ink}" stroke-width="1" opacity="0.2"/>`;
  const stamp = `<text x="${cx}" y="${f(cy + 20, 1)}" text-anchor="middle" font-family="Inter, 'Helvetica Neue', Arial, sans-serif" font-size="58" font-weight="600" letter-spacing="3" fill="${p.ink}" opacity="0.94">${initial}</text>`;
  const prints =
    `<rect width="480" height="480" filter="url(#${u}grain)" opacity="0.6"/>` +
    `<rect width="480" height="480" fill="url(#${u}vig)"/>`;
  const frame =
    `<rect x="16" y="16" width="448" height="448" fill="none" stroke="${p.hi}" stroke-opacity="0.35" stroke-width="1.5"/>` +
    `<path d="M16 38 V16 H38 M442 16 H464 V38 M464 442 V464 H442 M38 464 H16 V442" fill="none" stroke="${brass}" stroke-width="2" opacity="0.7"/>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="480" height="480" viewBox="0 0 480 480" role="img">
${defs}
${field}
${rings}
${nut}
${stamp}
${prints}
${frame}
</svg>
`;
}

// —— WAV tones ———————————————————————————————————————————————
// One 8-second loop per album: root + fifth + octave sines with a slow
// amplitude pulse, 22050 Hz mono 16-bit. Deterministic per album.
function toneWav(seed) {
  const rate = 22050;
  const seconds = 8;
  const count = rate * seconds;
  const root_ = 110 * Math.pow(2, (seed % 7) / 12);
  const fifth = root_ * 1.4983;
  const octave = root_ * 2;
  const data = Buffer.alloc(count * 2);
  for (let i = 0; i < count; i += 1) {
    const t = i / rate;
    const pulse = 0.72 + 0.28 * Math.sin((2 * Math.PI * t) / seconds);
    const fade = Math.min(1, t / 0.15, (seconds - t) / 0.25);
    const sample =
      0.42 * Math.sin(2 * Math.PI * root_ * t) +
      0.3 * Math.sin(2 * Math.PI * fifth * t + seed) +
      0.16 * Math.sin(2 * Math.PI * octave * t + seed * 2);
    const value = Math.max(-1, Math.min(1, sample * pulse * fade)) * 32767;
    data.writeInt16LE(Math.round(value), i * 2);
  }
  const header = Buffer.alloc(44);
  header.write('RIFF', 0);
  header.writeUInt32LE(36 + data.length, 4);
  header.write('WAVE', 8);
  header.write('fmt ', 12);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20); // PCM
  header.writeUInt16LE(2, 22); // mono
  header.writeUInt32LE(rate, 24);
  header.writeUInt32LE(rate * 2, 28);
  header.writeUInt16LE(2, 32);
  header.writeUInt16LE(16, 34);
  header.write('data', 36);
  header.writeUInt32LE(data.length, 40);
  return Buffer.concat([header, data]);
}

// One image per non-hostile catalogue artist key (catalogue.ts derives
// /media/artists/<key>.svg from the key, including both Alex Reed entries).
// The seed picks the tone field; existing files keep their seeds so their
// bytes stay stable across regenerations.
const artists = [
  ['mira-sol', 'Mira Sol', 0],
  ['alex-reed-north', 'Alex Reed', 1],
  ['alex-reed-south', 'Alex Reed', 8],
  ['the-compound', 'The Compound', 2],
  ['various-artists', 'Various Artists', 3],
  ['keratin', 'Keratin', 4],
  ['chris-zabriskie', 'Chris Zabriskie', 5],
  ['kai-engel', 'Kai Engel', 6],
  ['scott-buckley', 'Scott Buckley', 7],
  ['kevin-macleod', 'Kevin MacLeod', 9],
];

mkdirSync(join(root, 'audio'), { recursive: true });
mkdirSync(join(root, 'covers'), { recursive: true });
mkdirSync(join(root, 'artists'), { recursive: true });

const albumIds = Object.keys(palettes);
albumIds.forEach((id, index) => {
  writeFileSync(join(root, 'audio', `${id}.wav`), toneWav(index));
  writeFileSync(join(root, 'covers', `${id}.svg`), coverSvg(index));
});
artists.forEach(([slug, name, seed]) => {
  writeFileSync(join(root, 'artists', `${slug}.svg`), artistSvg(seed, name));
});
console.log(`generated ${albumIds.length} tones, ${albumIds.length} covers, ${artists.length} artist images`);
