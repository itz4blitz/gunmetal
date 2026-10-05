import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmod, mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { gzipSync } from 'node:zlib';
import { archiveFiles, nativePnpm } from './native-pnpm.ts';

type Entry = { name: string; data: Buffer; type?: string; size?: string };
// An independent reference writer for the ustar layout npm package archives use.
function tarball(entries: Entry[], end = Buffer.alloc(1024)): Buffer {
  const blocks: Buffer[] = [];
  for (const entry of entries) {
    const header = Buffer.alloc(512);
    header.write(entry.name, 0, 100, 'latin1');
    header.write('0000644\0', 100, 8, 'latin1');
    header.write('0000000\0', 108, 8, 'latin1');
    header.write('0000000\0', 116, 8, 'latin1');
    header.write(entry.size ?? `${entry.data.length.toString(8).padStart(11, '0')}\0`, 124, 12, 'latin1');
    header.write('00000000000\0', 136, 12, 'latin1');
    header.write('        ', 148, 8, 'latin1');
    header.write(entry.type ?? '0', 156, 1, 'latin1');
    header.write('ustar\0' + '00', 257, 8, 'latin1');
    let sum = 0;
    for (const byte of header) sum += byte;
    header.write(`${sum.toString(8).padStart(6, '0')}\0 `, 148, 8, 'latin1');
    blocks.push(header, entry.data, Buffer.alloc((512 - entry.data.length % 512) % 512));
  }
  return gzipSync(Buffer.concat([...blocks, end]));
}

const program = Buffer.from('#!/bin/sh\nexit 0\n');
const identity = Buffer.from(JSON.stringify({ name: '@pnpm/exe.linux-x64', version: '12.7.0' }));
const packed = [{ name: 'package/pnpm', data: program }, { name: 'package/package.json', data: identity }];
type Layout = { root: string; checksum: string; executable: string };
async function layout(archive = tarball(packed), files: Record<string, Buffer> = { pnpm: program, 'package.json': identity }): Promise<Layout> {
  const root = await mkdtemp(join(tmpdir(), 'gunmetal-native-pnpm-'));
  await writeFile(join(root, 'pnpm.tgz'), archive);
  const packageRoot = join(root, 'node_modules/@pnpm/exe.linux-x64');
  await mkdir(join(root, 'bin'), { recursive: true });
  await mkdir(packageRoot, { recursive: true });
  for (const [name, data] of Object.entries(files)) await writeFile(join(packageRoot, name), data);
  const executable = join(packageRoot, 'pnpm');
  await chmod(executable, 0o755);
  await symlink('../node_modules/@pnpm/exe.linux-x64/pnpm', join(root, 'bin/pnpm'));
  return { root, checksum: createHash('sha512').update(archive).digest('base64'), executable };
}
async function refuses(fixture: Layout, message: string): Promise<void> {
  try {
    await assert.rejects(nativePnpm(fixture.root, fixture.checksum), {
      findings: [{ rule: 'SEC-SUP-011', path: 'runtime.pnpm', message }],
    });
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
}
const unverifiable = 'native pnpm layout could not be verified';
const unmatched = 'native pnpm package files must match the verified archive';

// Verifies: SEC-SUP-011, SEC-SUP-033. TeamCity must use the checksum-bound native manager layout, never PATH discovery.
test('native pnpm accepts only the verified extracted package reached through its bin directory', async () => {
  const fixture = await layout();
  try {
    assert.deepEqual(await nativePnpm(fixture.root, fixture.checksum), fixture.executable);
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('native pnpm rejects a bin entry that does not resolve to the verified package executable', async () => {
  const fixture = await layout();
  await rm(join(fixture.root, 'bin/pnpm'));
  await writeFile(join(fixture.root, 'bin/pnpm'), '#!/bin/sh\nexit 0\n');
  await chmod(join(fixture.root, 'bin/pnpm'), 0o755);
  await refuses(fixture, 'native pnpm bin entry must resolve to the verified package executable');
});

test('native pnpm rejects an archive whose bytes do not match the provisioned checksum', async () => {
  const fixture = await layout();
  await writeFile(join(fixture.root, 'pnpm.tgz'), 'substituted archive');
  await refuses(fixture, 'native pnpm archive must match the pinned checksum');
});

test('native pnpm rejects an archive path that is not a file', async () => {
  const fixture = await layout();
  await rm(join(fixture.root, 'pnpm.tgz'));
  await mkdir(join(fixture.root, 'pnpm.tgz'));
  await refuses(fixture, 'native pnpm archive must match the pinned checksum');
});

test('native pnpm rejects a layout without its archive', async () => {
  const fixture = await layout();
  await rm(join(fixture.root, 'pnpm.tgz'));
  await refuses(fixture, unverifiable);
});

test('native pnpm requires a layout root', async () => {
  await assert.rejects(nativePnpm('', 'unused'), {
    findings: [{ rule: 'SEC-SUP-011', path: 'runtime.pnpm', message: 'native pnpm root is required' }],
  });
});

test('native pnpm rejects a package executable without an execute bit', async () => {
  const fixture = await layout();
  await chmod(fixture.executable, 0o644);
  await refuses(fixture, 'native pnpm bin entry must resolve to the verified package executable');
});

test('native pnpm rejects a package executable that is a directory', async () => {
  const fixture = await layout();
  await rm(fixture.executable);
  await mkdir(fixture.executable);
  await refuses(fixture, 'native pnpm bin entry must resolve to the verified package executable');
});

for (const manifest of [{ name: '@pnpm/exe.linux-arm64', version: '12.7.0' }, { name: '@pnpm/exe.linux-x64', version: '12.7.1' }]) {
  test(`native pnpm rejects the package identity ${manifest.name}@${manifest.version}`, async () => {
    const data = Buffer.from(JSON.stringify(manifest));
    await refuses(await layout(tarball([{ name: 'package/pnpm', data: program }, { name: 'package/package.json', data }]),
      { pnpm: program, 'package.json': data }), 'native pnpm package identity must match the pinned manager');
  });
}

test('native pnpm rejects package metadata that is not JSON', async () => {
  const data = Buffer.from('not json');
  await refuses(await layout(tarball([{ name: 'package/pnpm', data: program }, { name: 'package/package.json', data }]),
    { pnpm: program, 'package.json': data }), unverifiable);
});

// Verifies: SEC-SUP-011. The executable that runs must be the one inside the checksum-verified archive.
test('native pnpm rejects an executable that differs from the verified archive', async () => {
  await refuses(await layout(tarball(packed), { pnpm: Buffer.from('#!/bin/sh\nexit 1\n'), 'package.json': identity }), unmatched);
});

test('native pnpm rejects package metadata that differs from the verified archive', async () => {
  await refuses(await layout(tarball(packed), {
    pnpm: program, 'package.json': Buffer.from(JSON.stringify({ name: '@pnpm/exe.linux-x64', version: '12.7.0', license: 'MIT' })),
  }), unmatched);
});

for (const missing of ['package/pnpm', 'package/package.json']) {
  test(`native pnpm rejects a verified archive without ${missing}`, async () => {
    await refuses(await layout(tarball(packed.filter(entry => entry.name !== missing))), unmatched);
  });
}

test('native pnpm rejects a retained archive that is not a gzip archive', async () => {
  await refuses(await layout(Buffer.from('verified native pnpm fixture')), unverifiable);
});

// Verifies: SEC-SUP-011. An archive is read to its end marker or not trusted at all.
test('native pnpm rejects a verified archive that stops without its end marker', async () => {
  await refuses(await layout(tarball(packed, Buffer.alloc(0))), unverifiable);
});

test('native pnpm rejects a verified archive that holds nothing', async () => {
  await refuses(await layout(gzipSync(Buffer.alloc(0))), unverifiable);
});

for (const [name, entry] of [
  ['a directory entry', { name: 'package/extra', data: Buffer.alloc(0), type: '5' }],
  ['a symbolic link entry', { name: 'package/extra', data: Buffer.alloc(0), type: '2' }],
  ['an entry outside the package', { name: 'package/../pnpm', data: program }],
  ['a nested entry', { name: 'package/bin/pnpm', data: program }],
  ['an entry of another package root', { name: 'other/pnpm', data: program }],
  ['an entry with an unreadable size', { name: 'package/extra', data: program, size: 'zzzzzzzzzzz\0' }],
  ['an entry with a negative size', { name: 'package/extra', data: program, size: '-0000000001\0' }],
  ['an entry longer than the archive', { name: 'package/extra', data: program, size: '00000010000\0' }],
] as const) {
  test(`native pnpm rejects a verified archive with ${name}`, async () => {
    await refuses(await layout(tarball([...packed, entry])), unverifiable);
  });
}

// Verifies: SEC-SUP-011. Asked for nested files, the reader returns a package's directories and is as
// strict about every name: each stays under `package/`, with no empty, `.` or `..` part.
function nested(entries: Entry[]): unknown {
  try { return [...archiveFiles(tarball(entries), true)].map(([name, data]) => [name, data.toString()]); }
  catch { return 'refused'; }
}
test('the archive reader returns nested regular files when asked to', () => {
  assert.deepEqual(nested([
    { name: 'package/package.json', data: Buffer.from('{}') },
    { name: 'package/dist/index.js', data: Buffer.from('export {};') },
    { name: 'package/dist/compose/.keep', data: Buffer.alloc(0) },
    { name: 'package/dist/compose/a...b.d.ts', data: Buffer.from('export {};') },
  ]), [
    ['package/package.json', '{}'],
    ['package/dist/index.js', 'export {};'],
    ['package/dist/compose/.keep', ''],
    ['package/dist/compose/a...b.d.ts', 'export {};'],
  ]);
});
for (const name of [
  'package/../escape.js', 'package/dist/../../escape.js', 'package/dist/..', 'package/./index.js', 'package/dist/.',
  'package//index.js', 'package/dist/', '/package/index.js', 'other/index.js', 'package', 'package/dist/in dex.js', 'package/dist\\index.js',
]) {
  test(`the archive reader refuses the nested entry name ${JSON.stringify(name)}`, () => {
    assert.deepEqual(nested([{ name: 'package/package.json', data: Buffer.from('{}') }, { name, data: program }]), 'refused');
  });
}
for (const type of ['5', '2', '1', 'x']) {
  test(`the archive reader refuses a nested entry of type ${type}`, () => {
    assert.deepEqual(nested([{ name: 'package/package.json', data: Buffer.from('{}') }, { name: 'package/dist/index.js', data: Buffer.alloc(0), type }]), 'refused');
  });
}
