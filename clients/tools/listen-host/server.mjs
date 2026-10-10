// Library host for the Gunmetal demo. It reads the music folder, writes a
// folder-library document the client accepts, and streams the audio.
// Paths never appear in URLs: a track id is a hash, and the file map is
// the only way to open one.

import { createHash } from 'node:crypto';
import { createReadStream, existsSync, mkdirSync, openSync, readSync, closeSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, relative, sep } from 'node:path';
import { readdir } from 'node:fs/promises';

const musicRoot = process.env.MUSIC ?? '/music';
const cacheRoot = process.env.CACHE ?? '/cache';
const staticRoot = process.env.STATIC ?? '';
const port = Number(process.env.PORT ?? 8788);
const host = process.env.HOST ?? '0.0.0.0';

const sourcesPath = join(cacheRoot, 'video-sources.json');
const VIDEO = new Set(['.mp4', '.m4v', '.mkv', '.webm', '.mov']);
const VIDEO_WALK_MAX_FILES = 20_000;
const VIDEO_WALK_MAX_DEPTH = 8;
const API_BODY_MAX = 64 * 1024;

const AUDIO = new Set(['.flac', '.mp3']);
const MAX_BLOCK = 20 * 1024 * 1024;
const MAX_HEADER = 2 * 1024 * 1024;

function idOf(text) {
  return createHash('sha256').update(text).digest('hex').slice(0, 16);
}

function toneOf(id) {
  return `0${(Number.parseInt(id.slice(0, 2), 16) % 7) + 1}`;
}

function clean(value, fallback) {
  const text = value.replaceAll('\0', '').trim().slice(0, 200);
  return text.length > 0 ? text : fallback;
}

function yearOf(value) {
  const match = /(\d{4})/.exec(value ?? '');
  if (match === null) {
    return 0;
  }
  const year = Number(match[1]);
  return year >= 0 && year <= 9999 ? year : 0;
}

function replayGainDb(value) {
  if (typeof value !== 'string') {
    return undefined;
  }
  const match = /^([+-]?\d+(?:\.\d+)?)\s*db$/i.exec(value.trim());
  if (match === null) {
    return undefined;
  }
  const parsed = Number(match[1]);
  if (!Number.isFinite(parsed)) {
    return undefined;
  }
  const sign = parsed < 0 ? -1 : 1;
  const db = (sign * Math.round(Math.abs(parsed) * 100)) / 100;
  return db >= -15 && db <= 15 ? db : undefined;
}

function trackNumber(value, filename) {
  const fromTag = /^(\d+)/.exec(value ?? '');
  if (fromTag !== null) {
    const number = Number(fromTag[1]);
    if (number >= 1 && number <= 999) {
      return number;
    }
  }
  const fromName = /^(\d+)/.exec(filename);
  if (fromName !== null) {
    const number = Number(fromName[1]);
    if (number >= 1 && number <= 999) {
      return number;
    }
  }
  return 1;
}

function bits(buffer, offset, width) {
  let value = 0;
  for (let index = 0; index < width; index += 1) {
    const position = offset + index;
    const byte = buffer[position >> 3] ?? 0;
    value = (value << 1) | ((byte >> (7 - (position & 7))) & 1);
  }
  return value;
}

function readAt(fd, length, position) {
  const buffer = Buffer.alloc(length);
  const got = readSync(fd, buffer, 0, length, position);
  return got === length ? buffer : buffer.subarray(0, got);
}

function flacMeta(file) {
  const fd = openSync(file, 'r');
  try {
    const magic = readAt(fd, 4, 0);
    if (magic.toString() !== 'fLaC') {
      return undefined;
    }
    let position = 4;
    let sampleRate = 0;
    let totalSamples = 0;
    const tags = new Map();
    let picture = undefined;
    for (let guard = 0; guard < 64; guard += 1) {
      const header = readAt(fd, 4, position);
      if (header.length < 4) {
        break;
      }
      position += 4;
      const last = (header[0] & 0x80) !== 0;
      const type = header[0] & 0x7f;
      const length = header.readUIntBE(1, 3);
      if (length > MAX_BLOCK) {
        break;
      }
      const body = length === 0 ? Buffer.alloc(0) : readAt(fd, length, position);
      position += length;
      if (type === 0 && body.length >= 18) {
        sampleRate = bits(body, 80, 20);
        totalSamples = bits(body, 108, 36);
      } else if (type === 4) {
        readVorbis(body, tags);
      } else if (type === 6) {
        const next = readPicture(body);
        if (next !== undefined && (picture === undefined || next.front)) {
          picture = next;
        }
      }
      if (last) {
        break;
      }
    }
    if (sampleRate <= 0 || totalSamples <= 0) {
      return undefined;
    }
    return { durationMs: Math.round((totalSamples * 1000) / sampleRate), tags, picture };
  } finally {
    closeSync(fd);
  }
}

function readVorbis(body, tags) {
  if (body.length < 8) {
    return;
  }
  const vendor = body.readUInt32LE(0);
  let offset = 4 + vendor;
  if (offset + 4 > body.length) {
    return;
  }
  const count = body.readUInt32LE(offset);
  offset += 4;
  for (let index = 0; index < count && offset + 4 <= body.length; index += 1) {
    const length = body.readUInt32LE(offset);
    offset += 4;
    if (offset + length > body.length) {
      return;
    }
    const row = body.toString('utf8', offset, offset + length);
    offset += length;
    const split = row.indexOf('=');
    if (split > 0) {
      tags.set(row.slice(0, split).toUpperCase(), row.slice(split + 1));
    }
  }
}

function pictureExt(mime, bytes) {
  const clean = mime.split(';')[0].trim().toLowerCase();
  if (clean === 'image/jpeg' || clean === 'image/jpg' || clean === 'image/pjpeg') {
    return 'jpg';
  }
  if (clean === 'image/png' || clean === 'image/x-png') {
    return 'png';
  }
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return 'jpg';
  }
  if (
    bytes.length >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47 &&
    bytes[4] === 0x0d &&
    bytes[5] === 0x0a &&
    bytes[6] === 0x1a &&
    bytes[7] === 0x0a
  ) {
    return 'png';
  }
  return undefined;
}

function readPicture(body) {
  if (body.length < 32) {
    return undefined;
  }
  let offset = 4;
  const mimeLength = body.readUInt32BE(offset);
  offset += 4;
  if (offset + mimeLength > body.length) {
    return undefined;
  }
  const mime = body.toString('ascii', offset, offset + mimeLength).toLowerCase();
  offset += mimeLength;
  if (offset + 4 > body.length) {
    return undefined;
  }
  const description = body.readUInt32BE(offset);
  offset += 4 + description + 16;
  if (offset + 4 > body.length) {
    return undefined;
  }
  const dataLength = body.readUInt32BE(offset);
  offset += 4;
  if (offset + dataLength > body.length) {
    return undefined;
  }
  const front = body.readUInt32BE(0) === 3;
  const bytes = body.subarray(offset, offset + dataLength);
  const ext = pictureExt(mime, bytes);
  if (ext === undefined) {
    return undefined;
  }
  return { bytes, ext, front };
}

function mp3Meta(file) {
  const size = statSync(file).size;
  const fd = openSync(file, 'r');
  try {
    const head = readAt(fd, Math.min(size, MAX_HEADER), 0);
    let offset = 0;
    const tags = new Map();
    let picture = undefined;
    if (head.subarray(0, 3).toString() === 'ID3' && head.length >= 10) {
      const version = head[3];
      const tagSize = synchsafe(head, 6);
      const tag = head.subarray(10, Math.min(head.length, 10 + tagSize));
      const parsed = readId3(tag, version);
      for (const [key, value] of parsed.tags) {
        tags.set(key, value);
      }
      picture = parsed.picture;
      offset = 10 + tagSize;
    }
    const frame = findFrame(head, offset);
    if (frame === undefined) {
      return undefined;
    }
    const audioBytes = Math.max(0, size - frame.offset);
    const durationMs = Math.round((audioBytes * 8 * 1000) / frame.bitrate);
    if (durationMs < 1) {
      return undefined;
    }
    return { durationMs, tags, picture };
  } finally {
    closeSync(fd);
  }
}

function synchsafe(buffer, offset) {
  return ((buffer[offset] & 0x7f) << 21) | ((buffer[offset + 1] & 0x7f) << 14) | ((buffer[offset + 2] & 0x7f) << 7) | (buffer[offset + 3] & 0x7f);
}

const BITRATE_V1_L3 = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0];
const BITRATE_V2_L3 = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0];
const RATE_V1 = [44100, 48000, 32000];
const RATE_V2 = [22050, 24000, 16000];

function findFrame(buffer, start) {
  for (let offset = start; offset + 4 < buffer.length; offset += 1) {
    if (buffer[offset] !== 0xff || (buffer[offset + 1] & 0xe0) !== 0xe0) {
      continue;
    }
    const header = buffer.readUInt32BE(offset);
    const version = (header >> 19) & 3;
    const layer = (header >> 17) & 3;
    const bitrateIndex = (header >> 12) & 0xf;
    const rateIndex = (header >> 10) & 3;
    if (version === 1 || layer !== 1 || bitrateIndex === 0 || bitrateIndex === 15 || rateIndex === 3) {
      continue;
    }
    const bitrate = (version === 3 ? BITRATE_V1_L3 : BITRATE_V2_L3)[bitrateIndex] * 1000;
    const rate = (version === 3 ? RATE_V1 : RATE_V2)[rateIndex];
    if (bitrate > 0 && rate > 0) {
      return { offset, bitrate };
    }
  }
  return undefined;
}

function readId3(tag, version) {
  const tags = new Map();
  let picture = undefined;
  let offset = 0;
  const v22 = version === 2;
  while (offset + (v22 ? 6 : 10) <= tag.length) {
    const id = tag.subarray(offset, offset + (v22 ? 3 : 4)).toString('ascii');
    if (id === '\0\0\0' || id === '\0\0\0\0' || !/^[A-Z0-9]+$/.test(id)) {
      break;
    }
    const size = v22 ? tag.readUIntBE(offset + 3, 3) : version === 4 ? synchsafe(tag, offset + 4) : tag.readUInt32BE(offset + 4);
    const dataStart = offset + (v22 ? 6 : 10);
    if (size < 0 || dataStart + size > tag.length) {
      break;
    }
    const data = tag.subarray(dataStart, dataStart + size);
    if (id === 'TIT2' || id === 'TT2') {
      tags.set('TITLE', decodeText(data));
    } else if (id === 'TPE1' || id === 'TP1') {
      tags.set('ARTIST', decodeText(data));
    } else if (id === 'TPE2' || id === 'TP2') {
      tags.set('ALBUMARTIST', decodeText(data));
    } else if (id === 'TALB' || id === 'TAL') {
      tags.set('ALBUM', decodeText(data));
    } else if (id === 'TRCK' || id === 'TRK') {
      tags.set('TRACKNUMBER', decodeText(data));
    } else if (id === 'TYER' || id === 'TDRC' || id === 'TYE') {
      tags.set('DATE', decodeText(data));
    } else if ((id === 'APIC' || id === 'PIC') && picture === undefined) {
      picture = readApic(data, id === 'PIC');
    }
    offset = dataStart + size;
  }
  return { tags, picture };
}

function decodeText(data) {
  if (data.length === 0) {
    return '';
  }
  const encoding = data[0];
  const bytes = data.subarray(1);
  if (encoding === 1 || encoding === 2) {
    return bytes.toString('utf16le').replaceAll('\0', '');
  }
  return bytes.toString('utf8').replaceAll('\0', '');
}

function readApic(data, v22) {
  let offset = 1;
  const mimeEnd = data.indexOf(0, offset);
  if (mimeEnd < 0) {
    return undefined;
  }
  let mime = data.toString('ascii', offset, mimeEnd).toLowerCase();
  if (v22 && mime === 'jpg') {
    mime = 'image/jpeg';
  }
  if (v22 && mime === 'png') {
    mime = 'image/png';
  }
  offset = mimeEnd + 1 + 1;
  const descEnd = data.indexOf(0, offset);
  if (descEnd < 0) {
    return undefined;
  }
  const bytes = data.subarray(descEnd + 1);
  const ext = pictureExt(mime, bytes);
  if (ext === undefined) {
    return undefined;
  }
  return { bytes, ext, front: true };
}

async function walk(dir, out) {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      await walk(full, out);
    } else if (AUDIO.has(extname(entry.name).toLowerCase())) {
      out.push(full);
    }
  }
}

function videoContentType(ext) {
  if (ext === '.mp4' || ext === '.m4v') return 'video/mp4';
  if (ext === '.webm') return 'video/webm';
  if (ext === '.mkv') return 'video/x-matroska';
  return 'video/quicktime';
}

function readJsonFile(path, fallback) {
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJsonFile(path, value) {
  writeFileSync(path, JSON.stringify(value, undefined, 2));
}

function loadSources() {
  const read = readJsonFile(sourcesPath, { sources: [] });
  if (!Array.isArray(read.sources)) {
    return [];
  }
  return read.sources.filter(
    (source) =>
      typeof source?.id === 'string' &&
      typeof source?.name === 'string' &&
      typeof source?.path === 'string' &&
      source.path.startsWith('/'),
  );
}

function saveSources() {
  writeJsonFile(
    sourcesPath,
    { sources: video.sources.map(({ id, name, path }) => ({ id, name, path })) },
  );
}

/** Episode numbering, S01E02 style, or a dated episode. Neither means a film. */
function videoEpisodeOf(name) {
  const numbered = /\bs(\d{1,2})[ ._-]?e(\d{1,3})\b/i.exec(name);
  if (numbered !== null) {
    return { season: Number(numbered[1]), episode: Number(numbered[2]) };
  }
  const dated = /\b(\d{4})[-.](\d{2})[-.](\d{2})\b/.exec(name);
  if (dated !== null) {
    return { season: Number(dated[1]), episode: Number(`${dated[2]}${dated[3]}`) };
  }
  return undefined;
}

function videoTitleOf(name) {
  return name.replace(/[._]+/g, ' ').replace(/\s+/g, ' ').trim();
}

async function videoWalk(dir, out, depth, budget) {
  if (depth > VIDEO_WALK_MAX_DEPTH || out.length >= budget.max) {
    return;
  }
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    if (out.length >= budget.max || entry.name.startsWith('.')) {
      continue;
    }
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      await videoWalk(full, out, depth + 1, budget);
      continue;
    }
    if (!VIDEO.has(extname(entry.name).toLowerCase())) {
      continue;
    }
    const stat = statSync(full);
    out.push({ file: full, bytes: stat.size, modified: stat.mtime.toISOString() });
  }
}

/** Reads one source's tree into titles and the id-to-file map, bounded. */
async function scanVideoSource(source) {
  const found = [];
  await videoWalk(source.path, found, 1, { max: VIDEO_WALK_MAX_FILES });
  const titles = [];
  const media = new Map();
  for (const item of found) {
    const base = item.file.slice(source.path.length + 1);
    const last = base.lastIndexOf(sep);
    const name = base.slice(last + 1, base.length - extname(item.file).length);
    const episode = videoEpisodeOf(name);
    const id = idOf(`video:${item.file}`);
    const folder = last === -1 ? source.name : videoTitleOf(base.slice(0, last));
    titles.push({
      id,
      title: videoTitleOf(name),
      kind: episode === undefined ? 'movie' : 'episode',
      ...(episode === undefined ? {} : { show: folder, ...episode }),
      container: extname(item.file).toLowerCase(),
      bytes: item.bytes,
      modified: item.modified,
    });
    media.set(id, { file: item.file, type: videoContentType(extname(item.file).toLowerCase()) });
  }
  titles.sort((a, b) => a.title.localeCompare(b.title) || a.id.localeCompare(b.id));
  return { titles, media };
}

function readJsonBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on('data', (chunk) => {
      size += chunk.length;
      if (size > API_BODY_MAX) {
        reject(new Error('the body is too large'));
        req.destroy();
        return;
      }
      chunks.push(chunk);
    });
    req.on('end', () => {
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString('utf8')));
      } catch (error) {
        reject(error);
      }
    });
    req.on('error', reject);
  });
}

function metaOf(file) {
  return extname(file).toLowerCase() === '.flac' ? flacMeta(file) : mp3Meta(file);
}

export function buildLibrary(files) {
  const albums = new Map();
  const media = new Map();
  let skipped = 0;
  for (const file of files) {
    let meta;
    try {
      meta = metaOf(file);
    } catch {
      skipped += 1;
      continue;
    }
    if (meta === undefined || meta.durationMs < 1 || meta.durationMs > 86_400_000) {
      skipped += 1;
      continue;
    }
    const base = file.slice(file.lastIndexOf(sep) + 1);
    const folder = relative(musicRoot, file).split(sep)[0] ?? 'Unknown artist';
    const tags = meta.tags;
    const albumArtist = clean(tags.get('ALBUMARTIST') || tags.get('ARTIST') || folder, 'Unknown artist');
    const trackArtist = clean(tags.get('ARTIST') || albumArtist, albumArtist);
    const albumTitle = clean(tags.get('ALBUM') || 'Unknown album', 'Unknown album');
    const title = clean(tags.get('TITLE') || base.replace(/\.[^.]+$/, '').replaceAll('_', ' '), 'Untitled');
    const artistKey = idOf(`artist\0${albumArtist.toLowerCase()}`);
    const albumId = idOf(`album\0${artistKey}\0${albumTitle.toLowerCase()}`);
    let trackId = idOf(`track\0${file}`);
    while (media.has(trackId)) {
      trackId = idOf(`track\0${trackId}`);
    }
    const row = {
      id: trackId,
      albumId,
      discIndex: 1,
      number: trackNumber(tags.get('TRACKNUMBER'), base),
      title,
      artistName: trackArtist,
      durationMs: meta.durationMs,
      flag: 'ok',
      lyricsKind: 'none',
      mediaUrl: `/media/library/${trackId}`,
    };
    const trackGainDb = replayGainDb(tags.get('REPLAYGAIN_TRACK_GAIN'));
    const albumGainDb = replayGainDb(tags.get('REPLAYGAIN_ALBUM_GAIN'));
    if (trackGainDb !== undefined) {
      row.trackGainDb = trackGainDb;
    }
    if (albumGainDb !== undefined) {
      row.albumGainDb = albumGainDb;
    }
    media.set(trackId, { file, type: extname(file).toLowerCase() === '.flac' ? 'audio/flac' : 'audio/mpeg' });
    let album = albums.get(albumId);
    if (album === undefined) {
      album = {
        id: albumId,
        title: albumTitle,
        artistName: albumArtist,
        artistKey,
        year: yearOf(tags.get('DATE')),
        coverTone: toneOf(albumId),
        coverUrl: '',
        discs: [{ index: 1, title: '' }],
        hostile: false,
        tracks: [],
        picture: meta.picture,
      };
      albums.set(albumId, album);
    } else if (album.picture === undefined && meta.picture !== undefined) {
      album.picture = meta.picture;
    } else if (album.picture !== undefined && meta.picture?.front === true && album.picture.front !== true) {
      album.picture = meta.picture;
    }
    album.tracks.push(row);
  }
  mkdirSync(join(cacheRoot, 'covers'), { recursive: true });
  const covers = new Map();
  const albumList = [...albums.values()].sort((left, right) =>
    left.artistName.localeCompare(right.artistName) || left.title.localeCompare(right.title),
  );
  for (const album of albumList) {
    album.tracks.sort((left, right) => left.number - right.number || left.title.localeCompare(right.title));
    if (album.picture !== undefined) {
      const coverFile = join(cacheRoot, 'covers', `${album.id}.${album.picture.ext}`);
      writeFileSync(coverFile, album.picture.bytes);
      album.coverUrl = `/media/library/covers/${album.id}.${album.picture.ext}`;
      covers.set(album.id, { file: coverFile, type: album.picture.ext === 'png' ? 'image/png' : 'image/jpeg' });
    }
    delete album.picture;
  }
  const byKey = new Map();
  for (const album of albumList) {
    const existing = byKey.get(album.artistKey);
    if (existing === undefined) {
      byKey.set(album.artistKey, { key: album.artistKey, name: album.artistName, albumIds: [album.id] });
    } else {
      existing.albumIds.push(album.id);
    }
  }
  const artists = [...byKey.values()].sort((left, right) => (left.key < right.key ? -1 : 1));
  return {
    library: { kind: 'folder', albums: albumList, artists },
    media,
    covers,
    skipped,
  };
}

// Missing embedded art is looked up from MusicBrainz, then the Cover Art
// Archive. HTTPS only. The archive answers with a redirect to archive.org,
// then to a download host under archive.org. Those two hops are allowed.
// Nothing else is.
const ARTWORK_UA = 'Gunmetal/0.0.0 (https://atomicstudio.taild1bbf.ts.net)';
const ARTWORK_HOSTS = new Set(['musicbrainz.org', 'coverartarchive.org']);
const ARTWORK_CAP = 40;
const ARTWORK_INTERVAL_MS = 1000;
const ARTWORK_BUDGET_MS = 180_000;
const ARTWORK_TIMEOUT_MS = 8000;
const ARTWORK_JSON_MAX = 1024 * 1024;
const ARTWORK_IMAGE_MAX = 8 * 1024 * 1024;
const ARTWORK_MIN_BYTES = 32;
const ARTWORK_MBID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

function httpsHost(value) {
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    return undefined;
  }
  if (
    parsed.protocol !== 'https:' ||
    parsed.username !== '' ||
    parsed.password !== '' ||
    (parsed.port !== '' && parsed.port !== '443')
  ) {
    return undefined;
  }
  return parsed;
}

export function artworkUrlAllowed(value) {
  const parsed = httpsHost(value);
  return parsed !== undefined && ARTWORK_HOSTS.has(parsed.hostname);
}

/** A Cover Art Archive image hop, not a page the lookup may open on its own. */
export function archiveImageUrl(value) {
  const parsed = httpsHost(value);
  if (parsed === undefined) {
    return false;
  }
  const host = parsed.hostname;
  // The suffix is compared label by label, so a host like
  // not-archive.org cannot ride the archive's suffix.
  const archive =
    host === 'archive.org' || host.split('.').slice(-2).join('.') === 'archive.org';
  if (!archive) {
    return false;
  }
  const segments = parsed.pathname.split('/');
  return segments[1] === 'download' || (segments[1] === '0' && segments[2] === 'items');
}

export function publicCoverUrl(albumId, ext) {
  if (!/^[a-f0-9]{16}$/.test(albumId) || (ext !== 'jpg' && ext !== 'png')) {
    return '';
  }
  return `/media/library/covers/${albumId}.${ext}`;
}

const jobs = new Map();

export function activityDocument() {
  return {
    jobs: [...jobs.values()].map((job) => ({
      id: job.id,
      label: job.label,
      done: job.done,
      total: job.total,
    })),
  };
}

function setJob(id, label, done, total) {
  jobs.set(id, { id, label, done, total });
}

function clearJob(id) {
  jobs.delete(id);
}

export function publicArtistUrl(artistKey, ext) {
  if (!/^[a-f0-9]{16}$/.test(artistKey) || (ext !== 'jpg' && ext !== 'png')) {
    return '';
  }
  return `/media/library/artists/${artistKey}.${ext}`;
}

export function artistHopAllowed(value) {
  const parsed = httpsHost(value);
  if (parsed === undefined) {
    return false;
  }
  const host = parsed.hostname;
  const path = parsed.pathname;
  if (host === 'musicbrainz.org') {
    return path === '/ws/2/artist' || /^\/ws\/2\/artist\/[0-9a-f-]{36}$/.test(path);
  }
  if (host === 'www.wikidata.org') {
    return /^\/wiki\/Special:EntityData\/Q\d+\.json$/.test(path);
  }
  if (host === 'commons.wikimedia.org') {
    return path.startsWith('/wiki/Special:FilePath/') || path === '/w/index.php';
  }
  if (host === 'upload.wikimedia.org' || host === 'thumb.wikimedia.org') {
    return path.includes('/wikipedia/commons/') && (path.endsWith('.jpg') || path.endsWith('.png'));
  }
  return false;
}

function foldTag(value) {
  return String(value)
    .slice(0, 200)
    .normalize('NFKD')
    .replace(/\p{M}+/gu, '')
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, ' ')
    .trim()
    .replace(/ {2,}/g, ' ');
}

function luceneQuoted(value) {
  return `"${String(value)
    .slice(0, 200)
    .replace(/[\\+\-!(){}[\]^"~*?:/|&]/g, (ch) => `\\${ch}`)}"`;
}

export function releaseSearchUrl(artist, album) {
  const url = new URL('https://musicbrainz.org/ws/2/release');
  url.searchParams.set('query', `release:${luceneQuoted(album)} AND artist:${luceneQuoted(artist)}`);
  url.searchParams.set('fmt', 'json');
  url.searchParams.set('limit', '5');
  return url.href;
}

function creditName(release) {
  const credit = release['artist-credit'];
  if (!Array.isArray(credit)) {
    return '';
  }
  let name = '';
  const parts = Math.min(credit.length, 8);
  for (let index = 0; index < parts; index += 1) {
    const part = credit[index];
    if (part === null || typeof part !== 'object') {
      return '';
    }
    const artist = part.artist;
    const credited =
      typeof part.name === 'string'
        ? part.name
        : artist !== null && typeof artist === 'object' && typeof artist.name === 'string'
          ? artist.name
          : '';
    const joined = typeof part.joinphrase === 'string' ? part.joinphrase : '';
    name += credited + joined;
    if (name.length > 200) {
      return name.slice(0, 200);
    }
  }
  return name;
}

export function pickRelease(artist, album, payload) {
  if (payload === null || typeof payload !== 'object' || !Array.isArray(payload.releases)) {
    return undefined;
  }
  const wantArtist = foldTag(artist);
  const wantAlbum = foldTag(album);
  if (wantArtist === '' || wantAlbum === '') {
    return undefined;
  }
  let best;
  const count = Math.min(payload.releases.length, 5);
  for (let index = 0; index < count; index += 1) {
    const release = payload.releases[index];
    if (release === null || typeof release !== 'object' || typeof release.id !== 'string' || typeof release.title !== 'string') {
      continue;
    }
    const id = release.id.toLowerCase();
    if (!ARTWORK_MBID.test(id) || foldTag(release.title) !== wantAlbum || foldTag(creditName(release)) !== wantArtist) {
      continue;
    }
    const raw = Number(release.score);
    const score = Number.isFinite(raw) ? raw : 0;
    const rank = (release.status === 'Official' ? 1000 : 0) + score;
    if (best !== undefined && rank <= best.rank) {
      continue;
    }
    const group = release['release-group'];
    let groupId;
    if (group !== null && typeof group === 'object' && typeof group.id === 'string') {
      const candidate = group.id.toLowerCase();
      if (ARTWORK_MBID.test(candidate)) {
        groupId = candidate;
      }
    }
    best = { id, groupId, rank };
  }
  if (best === undefined) {
    return undefined;
  }
  return { id: best.id, groupId: best.groupId };
}

function headerOf(response, name) {
  const headers = response?.headers;
  if (headers === undefined || headers === null || typeof headers.get !== 'function') {
    return '';
  }
  return headers.get(name) ?? '';
}

function isJsonType(type) {
  const clean = type.split(';')[0].trim().toLowerCase();
  return clean === 'application/json' || clean === 'application/ld+json';
}

function networkImageExt(mime, bytes) {
  if (bytes.length < ARTWORK_MIN_BYTES) {
    return undefined;
  }
  const magic = pictureExt('', bytes);
  if (magic !== 'jpg' && magic !== 'png') {
    return undefined;
  }
  const clean = mime.split(';')[0].trim().toLowerCase();
  if (clean === '' || clean === 'application/octet-stream' || clean === 'binary/octet-stream') {
    return magic;
  }
  if (magic === 'jpg' && (clean === 'image/jpeg' || clean === 'image/jpg' || clean === 'image/pjpeg')) {
    return 'jpg';
  }
  if (magic === 'png' && (clean === 'image/png' || clean === 'image/x-png')) {
    return 'png';
  }
  return undefined;
}

async function discard(response) {
  try {
    await response?.body?.cancel();
  } catch {
    // A discarded body must not escape the lookup.
  }
}

async function readCapped(response, max) {
  const declared = Number(headerOf(response, 'content-length'));
  if (Number.isFinite(declared) && (declared < 0 || declared > max)) {
    await discard(response);
    return undefined;
  }
  const body = response?.body;
  if (body === undefined || body === null || typeof body.getReader !== 'function') {
    return undefined;
  }
  const reader = body.getReader();
  const chunks = [];
  let total = 0;
  try {
    for (;;) {
      const step = await reader.read();
      if (step.done) {
        break;
      }
      const piece = Buffer.from(step.value);
      total += piece.length;
      if (total > max) {
        await reader.cancel();
        return undefined;
      }
      chunks.push(piece);
    }
  } catch {
    try {
      await reader.cancel();
    } catch {
      // ignore
    }
    return undefined;
  }
  return Buffer.concat(chunks, total);
}

function delay(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function createPacer(intervalMs, now, sleep) {
  let nextAt = 0;
  return async function pace() {
    const wait = nextAt - now();
    if (wait > 0) {
      await sleep(wait);
    }
    nextAt = now() + intervalMs;
  };
}

async function allowedFetch(url, ctx, depth) {
  const allowed = depth === 0 ? artworkUrlAllowed(url) : artworkUrlAllowed(url) || archiveImageUrl(url);
  if (depth > 2 || !allowed) {
    return undefined;
  }
  const parsed = new URL(url);
  if (parsed.hostname === 'musicbrainz.org') {
    await ctx.pace();
  }
  let response;
  try {
    response = await ctx.fetchImpl(url, {
      method: 'GET',
      redirect: 'manual',
      headers: {
        'User-Agent': ARTWORK_UA,
        Accept: parsed.hostname === 'musicbrainz.org' ? 'application/json' : 'image/jpeg, image/png',
      },
      signal: AbortSignal.timeout(ARTWORK_TIMEOUT_MS),
    });
  } catch {
    return undefined;
  }
  const status = response?.status;
  if (typeof status !== 'number') {
    return undefined;
  }
  if (status >= 300 && status < 400) {
    const location = headerOf(response, 'location');
    await discard(response);
    let next;
    try {
      next = new URL(location, url).href;
    } catch {
      return undefined;
    }
    if (!artworkUrlAllowed(next) && !archiveImageUrl(next)) {
      return { kind: 'blocked' };
    }
    return allowedFetch(next, ctx, depth + 1);
  }
  if (status !== 200) {
    await discard(response);
    return undefined;
  }
  const type = headerOf(response, 'content-type');
  const bytes = await readCapped(response, parsed.hostname === 'musicbrainz.org' ? ARTWORK_JSON_MAX : ARTWORK_IMAGE_MAX);
  if (bytes === undefined) {
    return undefined;
  }
  if (parsed.hostname === 'musicbrainz.org') {
    if (!isJsonType(type)) {
      return undefined;
    }
    try {
      const json = JSON.parse(bytes.toString('utf8'));
      if (json === null || typeof json !== 'object' || Array.isArray(json)) {
        return undefined;
      }
      return { kind: 'json', json };
    } catch {
      return undefined;
    }
  }
  const ext = networkImageExt(type, bytes);
  if (ext === undefined) {
    return undefined;
  }
  return { kind: 'image', ext, bytes };
}

function coverFileFor(cacheDir, albumId, ext) {
  if (publicCoverUrl(albumId, ext) === '') {
    return undefined;
  }
  const file = join(cacheDir, `${albumId}.${ext}`);
  if (relative(cacheDir, file) !== `${albumId}.${ext}`) {
    return undefined;
  }
  return file;
}

function publishCover(album, covers, cacheDir, ext, bytes) {
  const url = publicCoverUrl(album.id, ext);
  const file = coverFileFor(cacheDir, album.id, ext);
  if (url === '' || file === undefined) {
    return false;
  }
  mkdirSync(cacheDir, { recursive: true });
  writeFileSync(file, bytes);
  covers.set(album.id, { file, type: ext === 'png' ? 'image/png' : 'image/jpeg' });
  album.coverUrl = url;
  return true;
}

function cachedImage(file, ext) {
  let fd;
  try {
    const stat = statSync(file);
    if (!stat.isFile() || stat.size < ARTWORK_MIN_BYTES || stat.size > ARTWORK_IMAGE_MAX) {
      return false;
    }
    fd = openSync(file, 'r');
    return pictureExt('', readAt(fd, 8, 0)) === ext;
  } catch {
    return false;
  } finally {
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch {
        // ignore
      }
    }
  }
}

function adoptCached(album, covers, cacheDir) {
  if (typeof album.id !== 'string' || album.coverUrl !== '') {
    return false;
  }
  const existing = covers.get(album.id);
  if (existing !== undefined) {
    const ext = existing.type === 'image/png' ? 'png' : 'jpg';
    const url = publicCoverUrl(album.id, ext);
    if (url === '') {
      return false;
    }
    album.coverUrl = url;
    return true;
  }
  for (const ext of ['jpg', 'png']) {
    const file = coverFileFor(cacheDir, album.id, ext);
    if (file === undefined || !cachedImage(file, ext)) {
      continue;
    }
    covers.set(album.id, { file, type: ext === 'png' ? 'image/png' : 'image/jpeg' });
    album.coverUrl = publicCoverUrl(album.id, ext);
    return true;
  }
  return false;
}

function artworkQueryOk(artist, album) {
  if (typeof artist !== 'string' || typeof album !== 'string') {
    return false;
  }
  if (artist === 'Unknown artist' || album === 'Unknown album') {
    return false;
  }
  return foldTag(artist) !== '' && foldTag(album) !== '';
}

function sealCoverUrls(albums) {
  if (!Array.isArray(albums)) {
    return;
  }
  for (const album of albums) {
    if (album === null || typeof album !== 'object') {
      continue;
    }
    if (
      album.coverUrl === '' ||
      album.coverUrl === publicCoverUrl(album.id, 'jpg') ||
      album.coverUrl === publicCoverUrl(album.id, 'png')
    ) {
      continue;
    }
    album.coverUrl = '';
  }
}

async function lookupOne(album, covers, ctx) {
  const search = await allowedFetch(releaseSearchUrl(album.artistName, album.title), ctx, 0);
  if (search === undefined || search.kind !== 'json') {
    return false;
  }
  const picked = pickRelease(album.artistName, album.title, search.json);
  if (picked === undefined) {
    return false;
  }
  const urls = [`https://coverartarchive.org/release/${picked.id}/front-500`];
  if (picked.groupId !== undefined) {
    urls.push(`https://coverartarchive.org/release-group/${picked.groupId}/front-500`);
  }
  for (const url of urls) {
    const image = await allowedFetch(url, ctx, 0);
    if (image?.kind === 'blocked') {
      break;
    }
    if (image === undefined || image.kind !== 'image') {
      continue;
    }
    if (publishCover(album, covers, ctx.cacheDir, image.ext, image.bytes)) {
      return true;
    }
  }
  return false;
}

async function fillArtworkRun(albums, covers, options, stats) {
  const now = options.now ?? Date.now;
  const sleep = options.sleep ?? delay;
  const limit = options.limit ?? ARTWORK_CAP;
  const budgetMs = options.budgetMs ?? ARTWORK_BUDGET_MS;
  const cacheDir = options.cacheDir ?? join(cacheRoot, 'covers');
  const fetchImpl = options.fetchImpl ?? fetch;
  const ctx = {
    fetchImpl,
    cacheDir,
    pace: createPacer(options.intervalMs ?? ARTWORK_INTERVAL_MS, now, sleep),
  };
  const started = now();
  const waiting = albums.filter(
    (album) =>
      album !== null &&
      typeof album === 'object' &&
      album.coverUrl !== publicCoverUrl(album.id, 'jpg') &&
      album.coverUrl !== publicCoverUrl(album.id, 'png') &&
      artworkQueryOk(album.artistName, album.title),
  );
  const total = Math.min(waiting.length, limit);
  if (total > 0) {
    setJob('artwork', 'Fetching album art', 0, total);
  }
  for (const album of albums) {
    if (album === null || typeof album !== 'object' || typeof album.coverUrl !== 'string') {
      continue;
    }
    if (album.coverUrl === publicCoverUrl(album.id, 'jpg') || album.coverUrl === publicCoverUrl(album.id, 'png')) {
      continue;
    }
    album.coverUrl = '';
    if (adoptCached(album, covers, cacheDir)) {
      continue;
    }
    if (typeof album.artistName !== 'string' || typeof album.title !== 'string' || typeof album.id !== 'string') {
      continue;
    }
    if (!artworkQueryOk(album.artistName, album.title) || publicCoverUrl(album.id, 'jpg') === '') {
      continue;
    }
    if (stats.looked >= limit || now() - started >= budgetMs) {
      continue;
    }
    stats.looked += 1;
    let ok = false;
    try {
      ok = await lookupOne(album, covers, ctx);
    } catch {
      ok = album.coverUrl === publicCoverUrl(album.id, 'jpg') || album.coverUrl === publicCoverUrl(album.id, 'png');
    }
    if (ok) {
      stats.filled += 1;
    } else {
      album.coverUrl = '';
      stats.failed += 1;
    }
    setJob('artwork', 'Fetching album art', stats.looked, total);
    options.publish?.();
  }
  clearJob('artwork');
}

export async function fillMissingArtwork(albums, covers, options) {
  const stats = { looked: 0, filled: 0, failed: 0 };
  try {
    if (Array.isArray(albums) && covers !== undefined && covers !== null && typeof covers.get === 'function') {
      await fillArtworkRun(albums, covers, options ?? {}, stats);
    }
  } catch {
    // Lookup failures stay in the counts. They must not escape.
  }
  try {
    sealCoverUrls(albums);
  } catch {
    // ignore
  }
  return stats;
}

export function wikidataId(relations) {
  if (!Array.isArray(relations)) {
    return undefined;
  }
  for (const relation of relations) {
    const resource = relation?.url?.resource;
    const match = /^https:\/\/www\.wikidata\.org\/wiki\/(Q\d+)$/.exec(resource ?? '');
    if (match !== null) {
      return match[1];
    }
  }
  return undefined;
}

export function wikidataPortrait(entity, qid) {
  const claims = entity?.entities?.[qid]?.claims?.P18;
  const value = claims?.[0]?.mainsnak?.datavalue?.value;
  if (typeof value !== 'string' || value.length < 1 || value.length > 240 || value.includes('/') || value.includes('\0')) {
    return undefined;
  }
  if (!value.endsWith('.jpg') && !value.endsWith('.png')) {
    return undefined;
  }
  return value;
}

async function artistFetch(url, ctx, depth) {
  if (depth > 5 || !artistHopAllowed(url)) {
    return undefined;
  }
  const parsed = new URL(url);
  if (parsed.hostname === 'musicbrainz.org') {
    await ctx.pace();
  }
  let response;
  try {
    response = await ctx.fetchImpl(url, {
      method: 'GET',
      redirect: 'manual',
      headers: {
        'User-Agent': ARTWORK_UA,
        Accept: parsed.hostname === 'musicbrainz.org' || parsed.hostname === 'www.wikidata.org' ? 'application/json' : 'image/jpeg, image/png',
      },
      signal: AbortSignal.timeout(ARTWORK_TIMEOUT_MS),
    });
  } catch {
    return undefined;
  }
  const status = response?.status;
  if (typeof status !== 'number') {
    return undefined;
  }
  if (status >= 300 && status < 400) {
    const location = headerOf(response, 'location');
    await discard(response);
    let next;
    try {
      next = new URL(location, url).href;
    } catch {
      return undefined;
    }
    if (!artistHopAllowed(next)) {
      return undefined;
    }
    return artistFetch(next, ctx, depth + 1);
  }
  if (status !== 200) {
    await discard(response);
    return undefined;
  }
  const type = headerOf(response, 'content-type');
  const bytes = await readCapped(response, parsed.hostname.endsWith('wikimedia.org') && parsed.pathname.includes('/wikipedia/commons/') ? ARTWORK_IMAGE_MAX : ARTWORK_JSON_MAX);
  if (bytes === undefined) {
    return undefined;
  }
  if (parsed.hostname === 'upload.wikimedia.org' || parsed.hostname === 'thumb.wikimedia.org') {
    const ext = networkImageExt(type, bytes);
    return ext === undefined ? undefined : { kind: 'image', ext, bytes };
  }
  if (!isJsonType(type)) {
    return undefined;
  }
  try {
    const json = JSON.parse(bytes.toString('utf8'));
    if (json === null || typeof json !== 'object' || Array.isArray(json)) {
      return undefined;
    }
    return { kind: 'json', json };
  } catch {
    return undefined;
  }
}

async function lookupArtist(artist, portraits, ctx) {
  const searchUrl = new URL('https://musicbrainz.org/ws/2/artist');
  searchUrl.searchParams.set('query', `artist:${luceneQuoted(artist.name)}`);
  searchUrl.searchParams.set('fmt', 'json');
  searchUrl.searchParams.set('limit', '5');
  const search = await artistFetch(searchUrl.href, ctx, 0);
  const rows = search?.kind === 'json' ? search.json.artists : undefined;
  if (!Array.isArray(rows)) {
    return false;
  }
  const wanted = foldTag(artist.name);
  const picked = rows.find((row) => foldTag(row?.name ?? '') === wanted && ARTWORK_MBID.test(row?.id ?? ''));
  if (picked === undefined) {
    return false;
  }
  const rels = await artistFetch(`https://musicbrainz.org/ws/2/artist/${picked.id}?inc=url-rels&fmt=json`, ctx, 0);
  const qid = rels?.kind === 'json' ? wikidataId(rels.json.relations) : undefined;
  if (qid === undefined) {
    return false;
  }
  const entity = await artistFetch(`https://www.wikidata.org/wiki/Special:EntityData/${qid}.json`, ctx, 1);
  const filename = entity?.kind === 'json' ? wikidataPortrait(entity.json, qid) : undefined;
  if (filename === undefined) {
    return false;
  }
  const fileUrl = `https://commons.wikimedia.org/wiki/Special:FilePath/${encodeURIComponent(filename)}?width=500`;
  const image = await artistFetch(fileUrl, ctx, 1);
  if (image?.kind !== 'image') {
    return false;
  }
  const url = publicArtistUrl(artist.key, image.ext);
  if (url === '') {
    return false;
  }
  const file = join(ctx.artistDir, `${artist.key}.${image.ext}`);
  writeFileSync(file, image.bytes);
  portraits.set(artist.key, { file, type: image.ext === 'png' ? 'image/png' : 'image/jpeg' });
  artist.imageUrl = url;
  return true;
}

export async function fillArtistPhotos(artists, portraits, options) {
  const stats = { looked: 0, filled: 0, failed: 0 };
  try {
    if (!Array.isArray(artists) || portraits === undefined || portraits === null) {
      return stats;
    }
    const now = options?.now ?? Date.now;
    const sleep = options?.sleep ?? delay;
    const limit = options?.limit ?? ARTWORK_CAP;
    const fetchImpl = options?.fetchImpl ?? fetch;
    const artistDir = options?.artistDir ?? join(cacheRoot, 'artists');
    mkdirSync(artistDir, { recursive: true });
    for (const artist of artists) {
      if (artist === null || typeof artist !== 'object' || publicArtistUrl(artist.key, 'jpg') === '') {
        continue;
      }
      for (const ext of ['jpg', 'png']) {
        const file = join(artistDir, `${artist.key}.${ext}`);
        if (!existsSync(file)) {
          continue;
        }
        portraits.set(artist.key, { file, type: ext === 'png' ? 'image/png' : 'image/jpeg' });
        artist.imageUrl = publicArtistUrl(artist.key, ext);
        break;
      }
    }
    const ctx = {
      fetchImpl,
      artistDir,
      pace: createPacer(options?.intervalMs ?? ARTWORK_INTERVAL_MS, now, sleep),
    };
    const waiting = artists.filter((artist) => artworkQueryOk(artist?.name, 'known') && publicArtistUrl(artist.key, 'jpg') !== '' && artist.imageUrl === undefined);
    const total = Math.min(waiting.length, limit);
    if (total > 0) {
      setJob('artists', 'Fetching artist photos', 0, total);
    }
    for (const artist of waiting) {
      if (stats.looked >= limit) {
        break;
      }
      stats.looked += 1;
      setJob('artists', 'Fetching artist photos', stats.looked, total);
      let ok = false;
      try {
        ok = await lookupArtist(artist, portraits, ctx);
      } catch {
        ok = false;
      }
      if (ok) {
        stats.filled += 1;
        options?.publish?.();
      } else {
        stats.failed += 1;
      }
    }
    clearJob('artists');
  } catch {
    clearJob('artists');
  }
  return stats;
}

export function guardRequest(handler, req, res) {
  const fail = () => {
    try {
      if (!res.writableEnded) {
        if (!res.headersSent) {
          res.statusCode = 500;
        }
        res.end();
      }
    } catch {
      // The request handler must not throw.
    }
  };
  try {
    const result = handler(req, res);
    if (result instanceof Promise) {
      result.catch(fail);
    }
  } catch {
    fail();
  }
}

function contentType(file) {
  const ext = extname(file).toLowerCase();
  if (ext === '.html') return 'text/html; charset=utf-8';
  if (ext === '.js') return 'text/javascript; charset=utf-8';
  if (ext === '.css') return 'text/css; charset=utf-8';
  if (ext === '.svg') return 'image/svg+xml';
  if (ext === '.txt') return 'text/plain; charset=utf-8';
  if (ext === '.webmanifest') return 'application/manifest+json';
  if (ext === '.woff2') return 'font/woff2';
  if (ext === '.json') return 'application/json; charset=utf-8';
  if (ext === '.wav') return 'audio/wav';
  if (ext === '.png') return 'image/png';
  if (ext === '.jpg' || ext === '.jpeg') return 'image/jpeg';
  return 'application/octet-stream';
}

function safeStatic(urlPath) {
  if (staticRoot === '') {
    return undefined;
  }
  let decoded;
  try {
    decoded = decodeURIComponent(urlPath);
  } catch {
    return undefined;
  }
  if (decoded.includes('\0') || decoded.includes('..')) {
    return undefined;
  }
  const full = join(staticRoot, decoded);
  const rel = relative(staticRoot, full);
  if (rel.startsWith('..') || rel.startsWith(sep)) {
    return undefined;
  }
  return full;
}

function sendFile(req, res, file, type) {
  const stat = statSync(file);
  const size = stat.size;
  res.setHeader('Accept-Ranges', 'bytes');
  res.setHeader('Content-Type', type);
  res.setHeader('X-Content-Type-Options', 'nosniff');
  // HTML and CSS keep stable URLs, so a reload has to revalidate them.
  // Hashed scripts and the audio bytes stay cacheable.
  if (type.startsWith('text/html') || type.startsWith('text/css')) {
    res.setHeader('Cache-Control', 'no-cache');
  }
  const range = req.headers.range;
  if (range !== undefined) {
    const match = /^bytes=(\d*)-(\d*)$/.exec(range);
    if (match === null) {
      res.statusCode = 416;
      res.setHeader('Content-Range', `bytes */${size}`);
      res.end();
      return;
    }
    let start = match[1] === '' ? 0 : Number(match[1]);
    let end = match[2] === '' ? size - 1 : Number(match[2]);
    if (match[1] === '' && match[2] !== '') {
      start = Math.max(0, size - Number(match[2]));
      end = size - 1;
    }
    if (!Number.isInteger(start) || !Number.isInteger(end) || start > end || start >= size) {
      res.statusCode = 416;
      res.setHeader('Content-Range', `bytes */${size}`);
      res.end();
      return;
    }
    end = Math.min(end, size - 1);
    res.statusCode = 206;
    res.setHeader('Content-Range', `bytes ${start}-${end}/${size}`);
    res.setHeader('Content-Length', end - start + 1);
    if (req.method === 'HEAD') {
      res.end();
      return;
    }
    createReadStream(file, { start, end }).pipe(res);
    return;
  }
  res.statusCode = 200;
  res.setHeader('Content-Length', size);
  if (req.method === 'HEAD') {
    res.end();
    return;
  }
  createReadStream(file).pipe(res);
}

const STORE_CATALOG_URL = 'https://github.com/itz4blitz/gunmetal-extensions/releases/download/extensions/catalog.json';
const STORE_HOSTS = new Set([
  'github.com',
  'release-assets.githubusercontent.com',
  'objects.githubusercontent.com',
]);
const STORE_CACHE_MS = 60_000;
const STORE_MAX_BYTES = 256 * 1024;
let storeCatalog = { at: 0, body: '' };

function storeHostAllowed(url) {
  return url.protocol === 'https:' && STORE_HOSTS.has(url.hostname);
}

async function fetchPinned(url, hops = 0) {
  if (hops > 3) {
    throw new Error('redirects');
  }
  const parsed = new URL(url);
  if (!storeHostAllowed(parsed)) {
    throw new Error('host');
  }
  const response = await fetch(parsed, { redirect: 'manual' });
  if (response.status >= 300 && response.status < 400) {
    const next = response.headers.get('location');
    if (next === null) {
      throw new Error('redirect');
    }
    return fetchPinned(new URL(next, parsed).toString(), hops + 1);
  }
  if (!response.ok) {
    throw new Error('status');
  }
  const text = await response.text();
  if (text.length > STORE_MAX_BYTES) {
    throw new Error('size');
  }
  return text;
}

function storeCatalogValid(text) {
  let value;
  try {
    value = JSON.parse(text);
  } catch {
    return false;
  }
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }
  return value.id === 'gunmetal.extensions' && value.version === '1' && Array.isArray(value.extensions) && value.extensions.length > 0;
}

async function publishedStoreCatalog() {
  const now = Date.now();
  if (storeCatalog.body !== '' && now - storeCatalog.at < STORE_CACHE_MS) {
    return storeCatalog.body;
  }
  const text = await fetchPinned(STORE_CATALOG_URL);
  if (!storeCatalogValid(text)) {
    throw new Error('catalog');
  }
  storeCatalog = { at: now, body: text };
  return text;
}

async function serveStoreCatalog(req, res) {
  try {
    const body = await publishedStoreCatalog();
    res.statusCode = 200;
    res.setHeader('Content-Type', 'application/json; charset=utf-8');
    res.setHeader('Cache-Control', 'no-store');
    res.setHeader('X-Content-Type-Options', 'nosniff');
    res.end(req.method === 'HEAD' ? undefined : body);
  } catch {
    res.statusCode = 502;
    res.setHeader('Cache-Control', 'no-store');
    res.end();
  }
}

const video = {
  sources: [],
  titles: [],
  media: new Map(),
  body: '{"kind":"video","titles":[]}',
};

function sourcesDocument() {
  return {
    sources: video.sources.map((source) => ({
      id: source.id,
      name: source.name,
      path: source.path,
      titles: source.titles ?? 0,
    })),
  };
}

function videoDocument() {
  return { kind: 'video', titles: video.titles };
}

async function rescanVideo() {
  const titles = [];
  video.media = new Map();
  for (const source of video.sources) {
    try {
      const scanned = await scanVideoSource(source);
      source.titles = scanned.titles.length;
      titles.push(...scanned.titles);
      for (const [id, hit] of scanned.media) {
        video.media.set(id, hit);
      }
    } catch {
      source.titles = 0;
    }
  }
  titles.sort((a, b) => a.title.localeCompare(b.title) || a.id.localeCompare(b.id));
  video.titles = titles;
  video.body = JSON.stringify(videoDocument());
}

async function serveSourcesApi(req, res, url) {
  const json = (code, value) => {
    res.statusCode = code;
    res.setHeader('Content-Type', 'application/json; charset=utf-8');
    res.setHeader('Cache-Control', 'no-store');
    res.setHeader('X-Content-Type-Options', 'nosniff');
    res.end(req.method === 'HEAD' ? undefined : JSON.stringify(value));
  };
  if (url.pathname === '/api/sources') {
    if (req.method === 'GET' || req.method === 'HEAD') {
      return json(200, sourcesDocument());
    }
    if (req.method === 'POST') {
      let body;
      try {
        body = await readJsonBody(req);
      } catch {
        return json(400, { error: 'the body is not json' });
      }
      const name = typeof body?.name === 'string' ? body.name.trim().slice(0, 80) : '';
      const path = typeof body?.path === 'string' ? body.path.trim() : '';
      if (name === '' || !path.startsWith('/') || path.includes('..') || path.includes('\0')) {
        return json(400, { error: 'a name and an absolute path inside the container are required' });
      }
      if (!existsSync(path) || !statSync(path).isDirectory()) {
        return json(400, { error: 'the path is not a directory this container can see' });
      }
      if (video.sources.some((source) => source.path === path)) {
        return json(409, { error: 'that path is already a source' });
      }
      const source = { id: idOf(`video-source:${path}`), name, path, titles: 0 };
      video.sources.push(source);
      saveSources();
      await rescanVideo();
      return json(201, source);
    }
    return json(405, { error: 'use GET, POST or DELETE' });
  }
  const match = /^\/api\/sources\/([a-f0-9]{16})(\/rescan)?$/.exec(url.pathname);
  if (match === null) {
    return json(404, { error: 'no such source route' });
  }
  const source = video.sources.find((candidate) => candidate.id === match[1]);
  if (source === undefined) {
    return json(404, { error: 'no such source' });
  }
  if (match[2] === '/rescan' && req.method === 'POST') {
    await rescanVideo();
    return json(200, source);
  }
  if (match[2] === undefined && req.method === 'DELETE') {
    video.sources = video.sources.filter((candidate) => candidate.id !== source.id);
    saveSources();
    await rescanVideo();
    return json(200, sourcesDocument());
  }
  return json(405, { error: 'use DELETE, or POST with /rescan' });
}

async function main() {
  const files = [];
  await walk(musicRoot, files);
  files.sort();
  const state = buildLibrary(files);
  const portraits = new Map();
  let body = JSON.stringify(state.library);
  const publish = () => {
    body = JSON.stringify(state.library);
  };

  video.sources = loadSources();
  await rescanVideo();
  console.log(`video sources=${video.sources.length} titles=${video.titles.length}`);
  console.log(`library albums=${state.library.albums.length} tracks=${state.media.size} skipped=${state.skipped}`);
  void (async () => {
    const artwork = await fillMissingArtwork(state.library.albums, state.covers, { publish });
    publish();
    console.log(`artwork looked=${artwork.looked} filled=${artwork.filled} failed=${artwork.failed}`);
    const photos = await fillArtistPhotos(state.library.artists, portraits, { publish });
    publish();
    console.log(`artists looked=${photos.looked} filled=${photos.filled} failed=${photos.failed}`);
  })().catch((error) => {
    console.log(`artwork failed=${error instanceof Error ? error.message : 'lookup'}`);
  });

  const CSP = [
    "default-src 'none'",
    "script-src 'self' 'wasm-unsafe-eval'",
    "style-src 'self' 'sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU='",
    "img-src 'self' blob:",
    "media-src 'self' blob:",
    "font-src 'self'",
    "connect-src 'self'",
    "worker-src 'self'",
    "manifest-src 'self'",
    "base-uri 'none'",
    "form-action 'self'",
    "frame-ancestors 'none'",
    "object-src 'none'",
    "require-trusted-types-for 'script'",
    'trusted-types gunmetal-loader',
  ].join('; ');

  createServer((req, res) => {
    guardRequest(async () => {
      const url = new URL(req.url ?? '/', 'http://127.0.0.1');
      const sourcesApi = url.pathname === '/api/sources' || url.pathname.startsWith('/api/sources/');
      if (!sourcesApi && req.method !== 'GET' && req.method !== 'HEAD') {
        res.statusCode = 405;
        res.end();
        return;
      }
      if (sourcesApi) {
        await serveSourcesApi(req, res, url);
        return;
      }
      if (url.pathname === '/video-library.json') {
        res.statusCode = 200;
        res.setHeader('Content-Type', 'application/json; charset=utf-8');
        res.setHeader('Cache-Control', 'no-store');
        res.setHeader('X-Content-Type-Options', 'nosniff');
        res.end(req.method === 'HEAD' ? undefined : video.body);
        return;
      }
      const film = /^\/media\/video\/([a-f0-9]{16})$/.exec(url.pathname);
      if (film !== null) {
        const hit = video.media.get(film[1]);
        if (hit === undefined) {
          res.statusCode = 404;
          res.end();
          return;
        }
        sendFile(req, res, hit.file, hit.type);
        return;
      }
      if (url.pathname === '/extensions/catalog.json') {
        void serveStoreCatalog(req, res);
        return;
      }
      if (url.pathname === '/activity.json') {
        res.statusCode = 200;
        res.setHeader('Content-Type', 'application/json; charset=utf-8');
        res.setHeader('Cache-Control', 'no-store');
        res.setHeader('X-Content-Type-Options', 'nosniff');
        res.end(req.method === 'HEAD' ? undefined : JSON.stringify(activityDocument()));
        return;
      }
      if (url.pathname === '/library.json') {
        res.statusCode = 200;
        res.setHeader('Content-Type', 'application/json; charset=utf-8');
        res.setHeader('Cache-Control', 'no-store');
        res.setHeader('X-Content-Type-Options', 'nosniff');
        res.end(req.method === 'HEAD' ? undefined : body);
        return;
      }
      const portrait = /^\/media\/library\/artists\/([a-f0-9]{16})\.(jpg|png)$/.exec(url.pathname);
      if (portrait !== null) {
        const hit = portraits.get(portrait[1]);
        if (hit === undefined || (portrait[2] === 'png') !== (hit.type === 'image/png')) {
          res.statusCode = 404;
          res.end();
          return;
        }
        sendFile(req, res, hit.file, hit.type);
        return;
      }
      const cover = /^\/media\/library\/covers\/([a-f0-9]{16})\.(jpg|png)$/.exec(url.pathname);
      if (cover !== null) {
        const hit = state.covers.get(cover[1]);
        if (hit === undefined || (cover[2] === 'png') !== (hit.type === 'image/png')) {
          res.statusCode = 404;
          res.end();
          return;
        }
        sendFile(req, res, hit.file, hit.type);
        return;
      }
      const track = /^\/media\/library\/([a-f0-9]{16})$/.exec(url.pathname);
      if (track !== null) {
        const hit = state.media.get(track[1]);
        if (hit === undefined) {
          res.statusCode = 404;
          res.end();
          return;
        }
        sendFile(req, res, hit.file, hit.type);
        return;
      }
      if (staticRoot !== '') {
        const candidate = safeStatic(url.pathname === '/' ? '/index.html' : url.pathname);
        if (candidate !== undefined && existsSync(candidate) && statSync(candidate).isFile()) {
          const type = contentType(candidate);
          if (type.startsWith('text/html')) {
            res.setHeader('Content-Security-Policy', CSP);
          }
          sendFile(req, res, candidate, type);
          return;
        }
        const index = join(staticRoot, 'index.html');
        if (!url.pathname.includes('.') && existsSync(index)) {
          res.setHeader('Content-Security-Policy', CSP);
          sendFile(req, res, index, 'text/html; charset=utf-8');
          return;
        }
      }
      res.statusCode = 404;
      res.end();
    }, req, res);
  }).listen(port, host, () => {
    console.log(`listening ${host}:${port}`);
  });
}

// Set by the dry logic check so importing this file does not bind a port.
if (process.env.GUNMETAL_LISTEN_HOST_CHECK !== '1') {
  await main();
}
