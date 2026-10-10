// @vitest-environment node
import { spawn, type ChildProcess } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, expect, test } from 'vitest';

/**
 * The listen host end to end: a real server process, real folders and a real
 * FLAC header, driven over HTTP the way the Libraries pane drives it.
 */

const SERVER = join(process.cwd(), 'tools/listen-host/server.mjs');

type Host = { url: string; stop: () => Promise<void> };

let work = '';
let running: Host[] = [];

beforeEach(() => {
  work = mkdtempSync(join(tmpdir(), 'gunmetal-libraries-'));
  mkdirSync(join(work, 'cache'));
  running = [];
});

afterEach(async () => {
  for (const host of running) {
    await host.stop();
  }
  rmSync(work, { recursive: true, force: true });
});

/** A FLAC file by the format's own layout: the marker, STREAMINFO, then the tags. */
function flac(tags: readonly string[], seconds: number): Buffer {
  const info = Buffer.alloc(34);
  info.writeUInt16BE(4096, 0);
  info.writeUInt16BE(4096, 2);
  const packed = (44_100n << 44n) | (1n << 41n) | (15n << 36n) | BigInt(44_100 * seconds);
  info.writeBigUInt64BE(packed, 10);
  const vendor = Buffer.from('gunmetal-test');
  const rows = tags.map((tag) => Buffer.from(tag));
  const comment = Buffer.concat([
    le32(vendor.length),
    vendor,
    le32(rows.length),
    ...rows.flatMap((row) => [le32(row.length), row]),
  ]);
  return Buffer.concat([
    Buffer.from('fLaC'),
    blockHeader(0, false, info.length),
    info,
    blockHeader(4, true, comment.length),
    comment,
  ]);
}

function le32(value: number): Buffer {
  const out = Buffer.alloc(4);
  out.writeUInt32LE(value, 0);
  return out;
}

function blockHeader(type: number, last: boolean, length: number): Buffer {
  const out = Buffer.alloc(4);
  out.writeUInt8((last ? 0x80 : 0) | type, 0);
  out.writeUIntBE(length, 1, 3);
  return out;
}

function musicFolder(name: string): string {
  const dir = join(work, name);
  mkdirSync(join(dir, 'Night Owner', 'Configured Night'), { recursive: true });
  writeFileSync(
    join(dir, 'Night Owner', 'Configured Night', '01 First Light.flac'),
    flac(['ARTIST=Night Owner', 'ALBUM=Configured Night', 'TITLE=First Light', 'TRACKNUMBER=1', 'DATE=2026'], 3),
  );
  return dir;
}

function filmFolder(name: string): string {
  const dir = join(work, name);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'Harbour.Lights.mp4'), Buffer.from('not really a film'));
  return dir;
}

function showFolder(name: string): string {
  const dir = join(work, name);
  mkdirSync(join(dir, 'The Dock'), { recursive: true });
  writeFileSync(join(dir, 'The Dock', 'The.Dock.S01E02.mkv'), Buffer.from('not really an episode'));
  return dir;
}

function start(music?: string): Promise<Host> {
  return new Promise((resolve, reject) => {
    const child: ChildProcess = spawn(process.execPath, [SERVER], {
      env: {
        PATH: process.env.PATH ?? '',
        HOST: '127.0.0.1',
        PORT: '0',
        CACHE: join(work, 'cache'),
        LOOKUPS: 'off',
        ...(music === undefined ? {} : { MUSIC: music }),
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let out = '';
    let err = '';
    let listening = false;
    const stop = () =>
      new Promise<void>((done) => {
        if (child.exitCode !== null || child.signalCode !== null) {
          done();
          return;
        }
        const timer = setTimeout(done, 1000);
        child.once('exit', () => {
          clearTimeout(timer);
          done();
        });
        child.kill('SIGKILL');
      });
    child.stderr?.on('data', (chunk: Buffer) => {
      err += chunk.toString('utf8');
    });
    child.stdout?.on('data', (chunk: Buffer) => {
      out += chunk.toString('utf8');
      const match = /listening 127\.0\.0\.1:(\d+)/.exec(out);
      // Later log lines arrive on the same stream: the host is registered once.
      if (match !== null && match[1] !== '0' && !listening) {
        listening = true;
        const host = { url: `http://127.0.0.1:${match[1]}`, stop };
        running.push(host);
        resolve(host);
      }
    });
    child.once('exit', (code) => {
      reject(new Error(`the listen host stopped with ${code}: ${err}`));
    });
  });
}

async function get(host: Host, path: string): Promise<{ status: number; body: unknown }> {
  const response = await fetch(`${host.url}${path}`);
  return { status: response.status, body: await response.json() };
}

async function send(
  host: Host,
  method: 'POST' | 'DELETE',
  path: string,
  body?: unknown,
): Promise<{ status: number; body: unknown }> {
  const response = await fetch(`${host.url}${path}`, {
    method,
    ...(body === undefined ? {} : { headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) }),
  });
  return { status: response.status, body: await response.json() };
}

type Row = { id: string; name: string; kind: string; path: string; items: number };

function rows(body: unknown): Row[] {
  return (body as { sources: Row[] }).sources;
}

function withoutIds(body: unknown): Omit<Row, 'id'>[] {
  return rows(body).map(({ name, kind, path, items }) => ({ name, kind, path, items }));
}

type Library = {
  kind: string;
  albums: {
    title: string;
    artistName: string;
    year: number;
    tracks: { title: string; durationMs: number; mediaUrl: string }[];
  }[];
  artists: { name: string }[];
};

const NO_MUSIC = '/gunmetal-test/no-such-music-folder';

test('an unset music folder is not a library', async () => {
  const host = await start();
  expect(await get(host, '/api/sources')).toStrictEqual({ status: 200, body: { sources: [] } });
  expect(await get(host, '/library.json')).toStrictEqual({
    status: 200,
    body: { kind: 'folder', albums: [], artists: [] },
  });
});

test('a server nobody has configured has no libraries, no music and no films', async () => {
  const host = await start(NO_MUSIC);
  expect(await get(host, '/api/sources')).toStrictEqual({ status: 200, body: { sources: [] } });
  expect(await get(host, '/library.json')).toStrictEqual({
    status: 200,
    body: { kind: 'folder', albums: [], artists: [] },
  });
  expect(await get(host, '/video-library.json')).toStrictEqual({ status: 200, body: { kind: 'video', titles: [] } });
});

test('a music library the owner adds is the music the player gets, and removing it takes the music away', async () => {
  const music = musicFolder('my-music');
  const host = await start(NO_MUSIC);

  const added = await send(host, 'POST', '/api/sources', { name: 'My Music', kind: 'music', path: music });
  expect(added.status).toStrictEqual(201);
  const row = added.body as Row;
  expect({ ...row, id: '' }).toStrictEqual({ id: '', name: 'My Music', kind: 'music', path: music, items: 1 });
  expect(/^[a-f0-9]{16}$/.test(row.id)).toStrictEqual(true);
  expect((await get(host, '/api/sources')).body).toStrictEqual({ sources: [row] });

  const library = (await get(host, '/library.json')).body as Library;
  expect(library.kind).toStrictEqual('folder');
  expect(library.artists.map((artist) => artist.name)).toStrictEqual(['Night Owner']);
  expect(
    library.albums.map((album) => ({
      title: album.title,
      artistName: album.artistName,
      year: album.year,
      tracks: album.tracks.map((track) => ({ title: track.title, durationMs: track.durationMs })),
    })),
  ).toStrictEqual([
    {
      title: 'Configured Night',
      artistName: 'Night Owner',
      year: 2026,
      tracks: [{ title: 'First Light', durationMs: 3000 }],
    },
  ]);
  const mediaUrl = library.albums[0]?.tracks[0]?.mediaUrl ?? '';
  const audio = await fetch(`${host.url}${mediaUrl}`);
  expect(audio.status).toStrictEqual(200);
  expect(audio.headers.get('content-type')).toStrictEqual('audio/flac');
  expect(Buffer.from(await audio.arrayBuffer())).toStrictEqual(
    readFileSync(join(music, 'Night Owner', 'Configured Night', '01 First Light.flac')),
  );
  // The music library is not a film source.
  expect((await get(host, '/video-library.json')).body).toStrictEqual({ kind: 'video', titles: [] });

  const removed = await send(host, 'DELETE', `/api/sources/${row.id}`);
  expect(removed).toStrictEqual({ status: 200, body: { sources: [] } });
  expect((await get(host, '/library.json')).body).toStrictEqual({ kind: 'folder', albums: [], artists: [] });
  expect((await fetch(`${host.url}${mediaUrl}`)).status).toStrictEqual(404);
});

test('films and shows are libraries of their own kind, and a rescan picks up a new file', async () => {
  const films = filmFolder('films');
  const shows = showFolder('shows');
  const music = musicFolder('music');
  const host = await start(NO_MUSIC);

  expect(
    (await send(host, 'POST', '/api/sources', { name: 'Movies', kind: 'movies', path: films })).status,
  ).toStrictEqual(201);
  expect((await send(host, 'POST', '/api/sources', { name: 'TV', kind: 'shows', path: shows })).status).toStrictEqual(
    201,
  );
  const musicRow = (await send(host, 'POST', '/api/sources', { name: 'Music', kind: 'music', path: music }))
    .body as Row;

  const listed = await get(host, '/api/sources');
  expect(withoutIds(listed.body)).toStrictEqual([
    { name: 'Movies', kind: 'movies', path: films, items: 1 },
    { name: 'TV', kind: 'shows', path: shows, items: 1 },
    { name: 'Music', kind: 'music', path: music, items: 1 },
  ]);
  const video = (await get(host, '/video-library.json')).body as {
    titles: { title: string; kind: string; show?: string; season?: number; episode?: number }[];
  };
  expect(
    video.titles.map(({ title, kind, show, season, episode }) => ({ title, kind, show, season, episode })),
  ).toStrictEqual([
    { title: 'Harbour Lights', kind: 'movie', show: undefined, season: undefined, episode: undefined },
    { title: 'The Dock S01E02', kind: 'episode', show: 'The Dock', season: 1, episode: 2 },
  ]);

  writeFileSync(
    join(music, 'Night Owner', 'Configured Night', '02 Second Light.flac'),
    flac(['ARTIST=Night Owner', 'ALBUM=Configured Night', 'TITLE=Second Light', 'TRACKNUMBER=2'], 2),
  );
  const rescanned = await send(host, 'POST', `/api/sources/${musicRow.id}/rescan`);
  expect(rescanned).toStrictEqual({ status: 200, body: { ...musicRow, items: 2 } });
  const library = (await get(host, '/library.json')).body as Library;
  expect(library.albums[0]?.tracks.map((track) => track.title)).toStrictEqual(['First Light', 'Second Light']);

  writeFileSync(join(films, 'Second.Film.webm'), Buffer.from('also not a film'));
  const filmRow = rows(listed.body)[0];
  expect((await send(host, 'POST', `/api/sources/${filmRow?.id}/rescan`)).body).toStrictEqual({ ...filmRow, items: 2 });
});

test('a library needs a name, a known kind and a folder the server can see, once', async () => {
  const music = musicFolder('music');
  const host = await start(NO_MUSIC);
  const refused = { error: 'a library needs a name, a kind (music, movies or shows) and an absolute path' };
  expect(await send(host, 'POST', '/api/sources', { name: 'Music', path: music })).toStrictEqual({
    status: 400,
    body: refused,
  });
  expect(await send(host, 'POST', '/api/sources', { name: 'Music', kind: 'photos', path: music })).toStrictEqual({
    status: 400,
    body: refused,
  });
  expect(await send(host, 'POST', '/api/sources', { name: '', kind: 'music', path: music })).toStrictEqual({
    status: 400,
    body: refused,
  });
  expect(await send(host, 'POST', '/api/sources', { name: 'Music', kind: 'music', path: 'music' })).toStrictEqual({
    status: 400,
    body: refused,
  });
  expect(
    await send(host, 'POST', '/api/sources', { name: 'Music', kind: 'music', path: join(work, 'not-there') }),
  ).toStrictEqual({ status: 400, body: { error: 'the path is not a directory this server can see' } });
  expect(
    (await send(host, 'POST', '/api/sources', { name: 'Music', kind: 'music', path: music })).status,
  ).toStrictEqual(201);
  expect(await send(host, 'POST', '/api/sources', { name: 'Again', kind: 'movies', path: music })).toStrictEqual({
    status: 409,
    body: { error: 'that path is already a library' },
  });
  expect(withoutIds((await get(host, '/api/sources')).body)).toStrictEqual([
    { name: 'Music', kind: 'music', path: music, items: 1 },
  ]);
});

test('the libraries the owner set are still there after a restart', async () => {
  const music = musicFolder('music');
  const films = filmFolder('films');
  const first = await start(NO_MUSIC);
  await send(first, 'POST', '/api/sources', { name: 'Music', kind: 'music', path: music });
  await send(first, 'POST', '/api/sources', { name: 'Movies', kind: 'movies', path: films });
  const before = (await get(first, '/api/sources')).body;
  await first.stop();

  const second = await start(NO_MUSIC);
  expect((await get(second, '/api/sources')).body).toStrictEqual(before);
  expect(withoutIds(before)).toStrictEqual([
    { name: 'Music', kind: 'music', path: music, items: 1 },
    { name: 'Movies', kind: 'movies', path: films, items: 1 },
  ]);
  const library = (await get(second, '/library.json')).body as Library;
  expect(library.albums.map((album) => album.title)).toStrictEqual(['Configured Night']);
});

test('a server from before libraries had kinds keeps its folders as visible libraries, once', async () => {
  const music = musicFolder('old-music');
  const films = filmFolder('old-films');
  const shows = showFolder('old-tvshows');
  writeFileSync(
    join(work, 'cache', 'video-sources.json'),
    JSON.stringify({
      sources: [
        { id: '1111111111111111', name: 'Movies', path: films },
        { id: '2222222222222222', name: 'TV Shows', path: shows },
      ],
    }),
  );
  const first = await start(music);
  const carried = await get(first, '/api/sources');
  expect(withoutIds(carried.body)).toStrictEqual([
    { name: 'Music', kind: 'music', path: music, items: 1 },
    { name: 'Movies', kind: 'movies', path: films, items: 1 },
    { name: 'TV Shows', kind: 'shows', path: shows, items: 1 },
  ]);
  const musicRow = rows(carried.body)[0];
  expect((await send(first, 'DELETE', `/api/sources/${musicRow?.id}`)).status).toStrictEqual(200);
  await first.stop();

  // The old music folder is still on disk and still named in the environment.
  // The owner removed that library, so it stays removed.
  const second = await start(music);
  expect(withoutIds((await get(second, '/api/sources')).body)).toStrictEqual([
    { name: 'Movies', kind: 'movies', path: films, items: 1 },
    { name: 'TV Shows', kind: 'shows', path: shows, items: 1 },
  ]);
  expect((await get(second, '/library.json')).body).toStrictEqual({ kind: 'folder', albums: [], artists: [] });
});
