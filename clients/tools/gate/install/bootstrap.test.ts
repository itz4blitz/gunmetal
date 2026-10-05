import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

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
