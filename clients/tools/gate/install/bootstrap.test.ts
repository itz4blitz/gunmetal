import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cp, mkdir, mkdtemp, readFile, readdir, realpath, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { decoy } from './decoy.ts';
import { archiveFiles } from './native-pnpm.ts';

const client = fileURLToPath(new URL('../../../', import.meta.url));
async function workspace(root: string): Promise<string> {
  const directory = join(root, 'clients');
  await cp(client, directory, { recursive: true, filter: source => !source.includes('/node_modules') });
  await mkdir(join(root, 'supply-chain'), { recursive: true });
  await cp(fileURLToPath(new URL('../../../../supply-chain/js-direct-deps.toml', import.meta.url)), join(root, 'supply-chain/js-direct-deps.toml'));
  return directory;
}
// Verifies: SEC-SUP-011, SEC-SUP-033. Bootstrap must validate before any project install.
for (const unsafe of [false, true]) {
  test(`checksum-pinned parser bootstrap checks a dependency-free workspace, unsafe=${unsafe}`, async () => {
    const root = await mkdtemp(join(tmpdir(), 'gunmetal-before-install-'));
    try {
      const directory = await workspace(root);
      if (unsafe) {
        const path = join(directory, 'pnpm-workspace.yaml');
        const settings = JSON.parse(await readFile(path, 'utf8')) as Record<string, unknown>;
        await writeFile(path, JSON.stringify({ ...settings, ignoreScripts: false }));
      }
      await assert.rejects(stat(join(directory, 'node_modules')), { code: 'ENOENT' });
      const result = spawnSync(process.execPath, [fileURLToPath(new URL('bootstrap.ts', import.meta.url)), directory], { encoding: 'utf8', timeout: 120000 });
      assert.deepEqual({ status: result.status, signal: result.signal, stderr: result.stderr,
        result: result.stdout.trim() === '' ? null : JSON.parse(result.stdout) }, unsafe ? {
        status: 1, signal: null, stderr: '', result: [{ rule: 'SEC-SUP-033', path: 'workspace.ignoreScripts', message: 'must be true' }],
      } : {
        status: 0, signal: null, stderr: '', result: { runtime: { node: '24.20.0', pnpm: '12.7.0' },
          pnpmPublication: '2026-09-25T10:38:41.952Z', manifests: ['package.json'], findings: [] },
      });
      await assert.rejects(stat(join(directory, 'node_modules')), { code: 'ENOENT' });
    } finally { await rm(root, { recursive: true, force: true }); }
  });
}

test('parser bootstrap refuses a modified cached archive without extracting or installing', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-parser-integrity-'));
  try {
    const archive = join(directory, 'yaml.tgz');
    await writeFile(archive, 'modified package archive');
    const result = spawnSync(process.execPath, [fileURLToPath(new URL('bootstrap.ts', import.meta.url)), directory, archive], { encoding: 'utf8', timeout: 120000 });
    assert.deepEqual({ status: result.status, signal: result.signal, stderr: result.stderr,
      result: result.stdout.trim() === '' ? null : JSON.parse(result.stdout) }, {
      status: 1, signal: null, stderr: '', result: [{ rule: 'SEC-SUP-011', path: 'bootstrap.parser', message: 'parser archive must match the pinned checksum' }],
    });
    await assert.rejects(stat(join(directory, 'node_modules')), { code: 'ENOENT' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

// Verifies: SEC-SUP-011. The parser is unpacked in-process, so no `tar` found on PATH can supply it.
test('a decoy tar first on PATH is never executed by the parser bootstrap', async () => {
  const root = await mkdtemp(join(tmpdir(), 'gunmetal-decoy-tar-'));
  try {
    const directory = await workspace(root);
    const { env, script, marker } = await decoy(root, 'tar');
    const result = spawnSync(process.execPath, [fileURLToPath(new URL('bootstrap.ts', import.meta.url)), directory], { env, encoding: 'utf8', timeout: 120000 });
    assert.deepEqual({ status: result.status, signal: result.signal, stderr: result.stderr,
      result: result.stdout.trim() === '' ? null : JSON.parse(result.stdout) }, {
      status: 0, signal: null, stderr: '', result: { runtime: { node: '24.20.0', pnpm: '12.7.0' },
        pnpmPublication: '2026-09-25T10:38:41.952Z', manifests: ['package.json'], findings: [] },
    });
    await assert.rejects(stat(marker), { code: 'ENOENT' });
    // Positive control: the decoy does leave its marker once something executes it.
    const control = spawnSync(script, [], { encoding: 'utf8', timeout: 10000 });
    assert.deepEqual({ status: control.status, signal: control.signal, marker: await readFile(marker, 'utf8') },
      { status: 0, signal: null, marker: 'ran' });
  } finally { await rm(root, { recursive: true, force: true }); }
});

// Verifies: SEC-SUP-011. The in-process reader yields exactly the files pnpm itself installs from the same archive.
// pnpm also writes one file of its own inside the installed package, the bin link `node_modules/.bin/yaml`;
// that directory is the installer's, not the archive's, and is left out of the comparison.
test('the archive reader unpacks the pinned parser archive to exactly the files pnpm installed', async () => {
  const response = await fetch('https://registry.npmjs.org/yaml/-/yaml-2.9.1.tgz', { signal: AbortSignal.timeout(30000) });
  const archive = Buffer.from(await response.arrayBuffer());
  assert.equal(createHash('sha512').update(archive).digest('base64'), '3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==');
  const digest = (data: Uint8Array): string => createHash('sha256').update(data).digest('hex');
  let unpacked: unknown;
  try { unpacked = [...archiveFiles(archive, true)].map(([name, data]) => `${name.slice('package/'.length)} ${digest(data)}`).sort(); }
  catch (error) { unpacked = String(error); }
  const installed: string[] = [];
  const base = await realpath(fileURLToPath(new URL('../../../node_modules/yaml', import.meta.url)));
  for (const entry of await readdir(base, { recursive: true, withFileTypes: true })) {
    const path = join(entry.parentPath, entry.name);
    if (!entry.isFile() || relative(base, path).startsWith('node_modules/')) continue;
    installed.push(`${relative(base, path)} ${digest(await readFile(path))}`);
  }
  assert.deepEqual(unpacked, installed.sort());
});
