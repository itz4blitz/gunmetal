import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { parseAllDocuments } from 'yaml';
import { nativePnpm } from './native-pnpm.ts';
import { inspect } from './policy.ts';

// What the pinned manager writes for one project that depends on another with `workspace:*`.
const members = {
  'packages/consumer': { dependencies: { '@gunmetal/probe-member': { specifier: 'workspace:*', version: 'link:../member' } } },
  'packages/member': {},
};

// Verifies: SEC-SUP-033. The workspace-member shape the lockfile policy accepts is the one pnpm really records.
test('the pinned manager records a workspace member the way the lockfile fixture does', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-workspace-lock-'));
  try {
    const settings = JSON.parse(await readFile(new URL('../../../pnpm-workspace.yaml', import.meta.url), 'utf8')) as Record<string, unknown>;
    await writeFile(join(directory, 'pnpm-workspace.yaml'), JSON.stringify({ ...settings, packages: ['packages/*'], storeDir: join(directory, 'store') }));
    await writeFile(join(directory, 'package.json'), JSON.stringify({ name: 'gunmetal-workspace-probe', private: true }));
    for (const [name, manifest] of [
      ['member', { name: '@gunmetal/probe-member', version: '1.0.0', private: true }],
      ['consumer', { name: '@gunmetal/probe-consumer', version: '1.0.0', private: true, dependencies: { '@gunmetal/probe-member': 'workspace:*' } }],
    ] as const) {
      await mkdir(join(directory, 'packages', name), { recursive: true });
      await writeFile(join(directory, 'packages', name, 'package.json'), JSON.stringify(manifest));
    }
    const run = spawnSync(await nativePnpm(), ['install', '--lockfile-only', '--no-frozen-lockfile', '--ignore-scripts'], { cwd: directory, encoding: 'utf8', timeout: 60000 });
    assert.deepEqual({ status: run.status, signal: run.signal }, { status: 0, signal: null }, `${run.stdout}\n${run.stderr}`);
    const text = await readFile(join(directory, 'pnpm-lock.yaml'), 'utf8');
    const recorded = parseAllDocuments(text).map(document => document.toJS() as unknown);
    // With no registry package to lock, the manager writes neither `packages` nor `snapshots`.
    assert.deepEqual(recorded, [{
      lockfileVersion: '9.0',
      settings: { autoInstallPeers: false, excludeLinksFromLockfile: false },
      importers: { '.': {}, ...members },
    }]);
    assert.deepEqual(inspect('lockfile', { text }), []);
    const fixture = parseAllDocuments(await readFile(new URL('fixtures/lockfile-workspace.txt', import.meta.url), 'utf8')).map(document => document.toJS() as { importers: Record<string, unknown> });
    assert.deepEqual({ 'packages/consumer': fixture[1]?.importers['packages/consumer'], 'packages/member': fixture[1]?.importers['packages/member'] }, members);
  } finally { await rm(directory, { recursive: true, force: true }); }
});
