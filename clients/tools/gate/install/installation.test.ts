import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmod, cp, mkdir, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { parseAllDocuments, stringify } from 'yaml';
import { nativePnpm } from './native-pnpm.ts';

const client = fileURLToPath(new URL('../../../', import.meta.url));
async function workspace(root: string): Promise<string> {
  const directory = join(root, 'clients');
  await cp(client, directory, { recursive: true, filter: source => !source.includes('/node_modules') });
  await mkdir(join(root, 'supply-chain'), { recursive: true });
  await cp(fileURLToPath(new URL('../../../../supply-chain/js-direct-deps.toml', import.meta.url)), join(root, 'supply-chain/js-direct-deps.toml'));
  return directory;
}
function check(directory: string, env = process.env): unknown {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('installation.ts', import.meta.url)), directory], { env, encoding: 'utf8', timeout: 120000 });
  return { status: result.status, signal: result.signal, stderr: result.stderr, result: result.stdout.trim() === '' ? null : JSON.parse(result.stdout) };
}

// Verifies: SEC-SUP-033. A safe file must not conceal an unsafe effective install.
test('workspace check refuses an effective script-policy environment override', async () => {
  const env = { ...process.env, pnpm_config_ignore_scripts: 'false' };
  const observed = spawnSync(await nativePnpm(), ['--dir', client, 'config', 'list', '--json'], { env, encoding: 'utf8', timeout: 10000 });
  assert.deepEqual({ status: observed.status, signal: observed.signal, ignoreScripts: JSON.parse(observed.stdout).ignoreScripts },
    { status: 0, signal: null, ignoreScripts: false });
  assert.deepEqual(check(client, env), { status: 1, signal: null, stderr: '', result: [{
    rule: 'SEC-SUP-033', path: 'effective.ignoreScripts', message: 'must be true',
  }] });
});

// Verifies: SEC-SUP-033, SEC-SUP-034, SEC-SUP-035. The release workflow consumes observed workspace facts.
test('actual workspace check observes pinned runtimes, live pnpm age and all manifests', () => {
  assert.deepEqual(check(client), { status: 0, signal: null, stderr: '', result: {
    runtime: { node: '24.20.0', pnpm: '12.7.0' },
    pnpmPublication: '2026-09-25T10:38:41.952Z', manifests: ['package.json'], findings: [],
  } });
});

// A stand-in for whatever `pnpm` an attacker or a stale image leaves on PATH. It records being run.
async function decoy(directory: string): Promise<{ env: NodeJS.ProcessEnv; script: string; marker: string }> {
  const script = join(directory, 'pnpm');
  await writeFile(script, '#!/bin/sh\nprintf ran > "$0.ran"\n');
  await chmod(script, 0o755);
  return { env: { ...process.env, PATH: `${directory}${delimiter}${process.env.PATH ?? ''}` }, script, marker: `${script}.ran` };
}

// Verifies: SEC-SUP-011, SEC-SUP-033. Only the verified native manager runs; PATH is never searched for one.
test('a decoy pnpm first on PATH is never executed by a successful workspace check', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-decoy-pnpm-'));
  try {
    const { env, script, marker } = await decoy(directory);
    assert.deepEqual(check(client, env), { status: 0, signal: null, stderr: '', result: {
      runtime: { node: '24.20.0', pnpm: '12.7.0' },
      pnpmPublication: '2026-09-25T10:38:41.952Z', manifests: ['package.json'], findings: [],
    } });
    await assert.rejects(stat(marker), { code: 'ENOENT' });
    // Positive control: the decoy does leave its marker once something executes it.
    const control = spawnSync(script, [], { encoding: 'utf8', timeout: 10000 });
    assert.deepEqual({ status: control.status, signal: control.signal, marker: await readFile(marker, 'utf8') },
      { status: 0, signal: null, marker: 'ran' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

// Verifies: SEC-SUP-011. The manager refusal reaches the result as itself, and nothing falls back to PATH.
test('without a native manager root the workspace check reports that refusal and never runs a pnpm from PATH', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-decoy-pnpm-'));
  try {
    const { env, marker } = await decoy(directory);
    const { GUNMETAL_NATIVE_PNPM_ROOT: _root, ...withoutRoot } = env;
    assert.deepEqual(check(client, withoutRoot), { status: 1, signal: null, stderr: '', result: [{
      rule: 'SEC-SUP-011', path: 'runtime.pnpm', message: 'native pnpm root is required',
    }] });
    await assert.rejects(stat(marker), { code: 'ENOENT' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

// Verifies: SEC-SUP-035. A workspace whose reason list is absent cannot pass.
test('actual workspace check refuses a workspace without its dependency reason list', async () => {
  const root = await mkdtemp(join(tmpdir(), 'gunmetal-install-reasons-'));
  try {
    const directory = await workspace(root);
    await rm(join(root, 'supply-chain'), { recursive: true });
    assert.deepEqual(check(directory), { status: 1, signal: null, stderr: '', result: [{
      rule: 'SEC-SUP-035', path: 'supply-chain/js-direct-deps.toml', message: 'dependency reason list is unavailable',
    }] });
  } finally { await rm(root, { recursive: true, force: true }); }
});

for (const scenario of ['scripts', 'peer', 'runtime']) {
  test(`actual workspace check refuses a changed ${scenario} boundary`, async () => {
    const root = await mkdtemp(join(tmpdir(), 'gunmetal-install-policy-'));
    try {
      const directory = await workspace(root);
      let expected: unknown;
      if (scenario === 'scripts') {
        const path = join(directory, 'pnpm-workspace.yaml');
        const settings = JSON.parse(await readFile(path, 'utf8')) as Record<string, unknown>;
        await writeFile(path, JSON.stringify({ ...settings, ignoreScripts: false }));
        expected = [{ rule: 'SEC-SUP-033', path: 'workspace.ignoreScripts', message: 'must be true' }];
      } else {
        const path = join(directory, 'package.json');
        const manifest = JSON.parse(await readFile(path, 'utf8')) as Record<string, unknown>;
        if (scenario === 'peer') {
          await writeFile(path, JSON.stringify({ ...manifest, peerDependencies: { yaml: '2.9.1' } }));
          expected = [{ rule: 'SEC-SUP-035', path: 'manifest.peerDependencies.yaml', message: 'only a known workspace member with workspace:* is allowed' }];
        } else {
          await writeFile(path, JSON.stringify({ ...manifest, engines: { node: '24.19.0', pnpm: '12.7.0' } }));
          expected = [{ rule: 'SEC-SUP-011', path: 'runtime.node', message: 'observed runtime must match the manifest pin' }];
        }
      }
      assert.deepEqual(check(directory), { status: 1, signal: null, stderr: '', result: expected });
    } finally { await rm(root, { recursive: true, force: true }); }
  });
}

// Verifies: SEC-SUP-033. Transitive identities must be checked before installation.
for (const index of [0, 1]) {
  test(`initial registry routing refuses a transitive alternative registry in lock document ${index}`, async () => {
    const root = await mkdtemp(join(tmpdir(), 'gunmetal-before-install-route-'));
    try {
      const directory = await workspace(root);
      const path = join(directory, 'pnpm-lock.yaml');
      const documents = parseAllDocuments(await readFile(path, 'utf8')).map(document => document.toJS());
      assert.equal(documents.length, 2);
      documents[index].packages['@jsr/std@1.0.0'] = { resolution: { integrity: 'sha512-fixture' } };
      await writeFile(path, documents.map(document => stringify(document)).join('\n---\n'));
      assert.deepEqual(check(directory), { status: 1, signal: null, stderr: '', result: [{
        rule: 'SEC-SUP-033', path: 'effective.@jsr/std', message: 'only registry.npmjs.org is allowed',
      }] });
    } finally { await rm(root, { recursive: true, force: true }); }
  });
}
