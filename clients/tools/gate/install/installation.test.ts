import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { parseAllDocuments, stringify } from 'yaml';

const client = fileURLToPath(new URL('../../../', import.meta.url));
function check(directory: string, env = process.env): unknown {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('installation.ts', import.meta.url)), directory], { env, encoding: 'utf8', timeout: 120000 });
  return { status: result.status, signal: result.signal, stderr: result.stderr, result: result.stdout.trim() === '' ? null : JSON.parse(result.stdout) };
}

// Verifies: SEC-SUP-033. A safe file must not conceal an unsafe effective install.
test('workspace check refuses an effective script-policy environment override', () => {
  const env = { ...process.env, pnpm_config_ignore_scripts: 'false' };
  const observed = spawnSync('pnpm', ['--dir', client, 'config', 'list', '--json'], { env, encoding: 'utf8', timeout: 10000 });
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

for (const scenario of ['scripts', 'peer', 'runtime']) {
  test(`actual workspace check refuses a changed ${scenario} boundary`, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'gunmetal-install-policy-'));
    try {
      for (const file of ['package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml']) await cp(join(client, file), join(directory, file));
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
    } finally { await rm(directory, { recursive: true, force: true }); }
  });
}

// Verifies: SEC-SUP-033. Transitive identities must be checked before installation.
for (const index of [0, 1]) {
  test(`initial registry routing refuses a transitive alternative registry in lock document ${index}`, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'gunmetal-before-install-route-'));
    try {
      for (const file of ['package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml']) await cp(join(client, file), join(directory, file));
      const path = join(directory, 'pnpm-lock.yaml');
      const documents = parseAllDocuments(await readFile(path, 'utf8')).map(document => document.toJS());
      assert.equal(documents.length, 2);
      documents[index].packages['@jsr/std@1.0.0'] = { resolution: { integrity: 'sha512-fixture' } };
      await writeFile(path, documents.map(document => stringify(document)).join('\n---\n'));
      assert.deepEqual(check(directory), { status: 1, signal: null, stderr: '', result: [{
        rule: 'SEC-SUP-033', path: 'effective.@jsr/std', message: 'only registry.npmjs.org is allowed',
      }] });
    } finally { await rm(directory, { recursive: true, force: true }); }
  });
}
