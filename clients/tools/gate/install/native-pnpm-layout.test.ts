import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, readdir, readlink, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { nativePnpm } from './native-pnpm.ts';

function build(...args: string[]): unknown {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('native-pnpm-layout.ts', import.meta.url)), ...args], { encoding: 'utf8', timeout: 120000 });
  return { status: result.status, signal: result.signal, stderr: result.stderr, result: result.stdout.trim() === '' ? null : JSON.parse(result.stdout) };
}
function refusal(message: string): unknown {
  return { status: 1, signal: null, stderr: '', result: [{ rule: 'SEC-SUP-011', path: 'runtime.pnpm', message }] };
}
// The archive TeamCity downloaded and checked; the builder under test re-checks it against the pin.
const pinned = join(String(process.env.GUNMETAL_NATIVE_PNPM_ROOT), 'pnpm.tgz');

// Verifies: SEC-SUP-011, SEC-SUP-033. The committed builder makes the layout the checks run against.
test('the layout builder turns the pinned archive into the layout the native manager check accepts', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-native-layout-'));
  try {
    const root = join(directory, 'native-pnpm');
    const executable = join(root, 'node_modules/@pnpm/exe.linux-x64/pnpm');
    assert.deepEqual(build(pinned, root), { status: 0, signal: null, stderr: '', result: { root, executable } });
    assert.deepEqual(await nativePnpm(root), executable);
    const version = spawnSync(executable, ['--version'], { encoding: 'utf8', timeout: 10000 });
    assert.deepEqual({
      top: (await readdir(root)).sort(),
      files: (await readdir(join(root, 'node_modules/@pnpm/exe.linux-x64'))).sort(),
      bin: await readlink(join(root, 'bin/pnpm')),
      mode: (await stat(executable)).mode & 0o777,
      archive: (await readFile(join(root, 'pnpm.tgz'))).equals(await readFile(pinned)),
      version: { status: version.status, signal: version.signal, stdout: version.stdout.trim() },
    }, {
      top: ['bin', 'node_modules', 'pnpm.tgz'],
      files: ['LICENSE', 'THIRD-PARTY-NOTICES.md', 'package.json', 'pnpm'],
      bin: '../node_modules/@pnpm/exe.linux-x64/pnpm',
      mode: 0o755,
      archive: true,
      version: { status: 0, signal: null, stdout: '12.7.0' },
    });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('the layout builder refuses an archive that is not the pinned one and creates nothing', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-native-layout-'));
  try {
    const archive = join(directory, 'pnpm.tgz');
    await writeFile(archive, 'not the pinned archive');
    const root = join(directory, 'native-pnpm');
    assert.deepEqual(build(archive, root), refusal('native pnpm archive must match the pinned checksum'));
    await assert.rejects(stat(root), { code: 'ENOENT' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('the layout builder refuses an archive it cannot read and creates nothing', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-native-layout-'));
  try {
    const root = join(directory, 'native-pnpm');
    assert.deepEqual(build(join(directory, 'absent.tgz'), root), refusal('native pnpm layout could not be built'));
    await assert.rejects(stat(root), { code: 'ENOENT' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('the layout builder never builds into an existing root', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-native-layout-'));
  try {
    const root = join(directory, 'native-pnpm');
    await mkdir(root);
    await writeFile(join(root, 'kept'), 'present before the build');
    assert.deepEqual(build(pinned, root), refusal('native pnpm layout could not be built'));
    assert.deepEqual(await readdir(root), ['kept']);
  } finally { await rm(directory, { recursive: true, force: true }); }
});

for (const args of [[], [pinned]]) {
  test(`the layout builder requires an archive and a root, given ${args.length} argument(s)`, () => {
    assert.deepEqual(build(...args), refusal('native pnpm archive and layout root are required'));
  });
}
