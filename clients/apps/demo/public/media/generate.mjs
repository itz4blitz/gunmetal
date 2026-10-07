// Generates the demo's media: per-album WAV tones, per-album SVG covers and
// per-artist SVG images. Checked-in script, deterministic output, no deps.
// CP-011 formalises the SHA-256 manifest; until then `node generate.mjs`
// must be run after any change here and its output committed.
// Audio is placeholder tone (design-language: honest fixtures, not fake art).
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
  header.writeUInt16LE(1, 22); // mono
  header.writeUInt32LE(rate, 24);
  header.writeUInt32LE(rate * 2, 28);
  header.writeUInt16LE(2, 32);
  header.writeUInt16LE(16, 34);
  header.write('data', 36);
  header.writeUInt32LE(data.length, 40);
  return Buffer.concat([header, data]);
}

// —— SVG covers ——————————————————————————————————————————————
// Generative, per-album compositions in the tone palette. No text in the art
// (LIB-146: badges are data, never baked into posters).
function coverSvg(seed) {
  const id = Object.keys(palettes)[seed];
  const p = palettes[id];
  const a = (seed * 37) % 360;
  const circles = [0, 1, 2, 3, 4]
    .map((i) => {
      const r = 150 - i * 26 - ((seed * 13) % 18);
      const cx = 320 + Math.cos((a + i * 52) * (Math.PI / 180)) * (60 + i * 22);
      const cy = 320 + Math.sin((a + i * 52) * (Math.PI / 180)) * (60 + i * 22);
      return `<circle cx="${cx.toFixed(1)}" cy="${cy.toFixed(1)}" r="${r}" fill="none" stroke="${i % 2 === 0 ? p.hi : p.mid}" stroke-width="${14 - i * 2}" opacity="${0.9 - i * 0.14}"/>`;
    })
    .join('');
  const bars = [0, 1, 2, 3, 4, 5, 6, 7]
    .map((i) => {
      const h = 60 + ((seed * (i + 3) * 29) % 220);
      const x = 60 + i * 66;
      return `<rect x="${x}" y="${(640 - h).toFixed(0)}" width="30" height="${h.toFixed(0)}" fill="${i % 2 === 0 ? p.mid : p.hi}" opacity="${0.35 + (i % 3) * 0.18}"/>`;
    })
    .join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="640" viewBox="0 0 640 640">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="${p.mid}"/>
      <stop offset="0.55" stop-color="${p.deep}"/>
      <stop offset="1" stop-color="${p.deep}"/>
    </linearGradient>
  </defs>
  <rect width="640" height="640" fill="url(#g)"/>
  <g transform="rotate(${a % 40} 320 320)">${circles}</g>
  ${bars}
  <rect x="24" y="24" width="592" height="592" fill="none" stroke="${p.hi}" stroke-opacity="0.35" stroke-width="2"/>
</svg>
`;
}

// —— Artist images ———————————————————————————————————————————
// Hex-nut mark on a tone field, per design-language §1.
function artistSvg(seed, name) {
  const tones = Object.keys(palettes);
  const id = tones[seed % tones.length];
  const p = palettes[id];
  const initial = name.trim().slice(0, 1).toUpperCase();
  return `<svg xmlns="http://www.w3.org/2000/svg" width="480" height="480" viewBox="0 0 480 480">
  <rect width="480" height="480" fill="${p.deep}"/>
  <circle cx="240" cy="240" r="190" fill="${p.mid}"/>
  <circle cx="240" cy="240" r="190" fill="none" stroke="${p.hi}" stroke-width="6" stroke-opacity="0.7"/>
  <polygon points="240,96 359,168 359,312 240,384 121,312 121,168" fill="${p.deep}"/>
  <polygon points="240,96 359,168 359,312 240,384 121,312 121,168" fill="none" stroke="${p.hi}" stroke-width="5" stroke-opacity="0.8"/>
  <text x="240" y="298" text-anchor="middle" font-family="Inter, system-ui, sans-serif" font-size="150" font-weight="800" fill="${p.ink}">${initial}</text>
</svg>
`;
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
