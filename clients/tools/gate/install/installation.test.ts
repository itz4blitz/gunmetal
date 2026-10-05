import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { parseAllDocuments, stringify } from 'yaml';
import { decoy } from './decoy.ts';
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
const passed = { status: 0, signal: null, stderr: '', result: {
  runtime: { node: '24.20.0', pnpm: '12.7.0' },
  pnpmPublication: '2026-09-25T10:38:41.952Z', manifests: ['package.json'], findings: [],
} };
function refusal(rule: string, path: string, message: string): unknown {
  return { status: 1, signal: null, stderr: '', result: [{ rule, path, message }] };
}
// Runs `change` on a private copy of the workspace and returns what the check then reports.
async function changed(change: (directory: string, root: string) => Promise<void>): Promise<unknown> {
  const root = await mkdtemp(join(tmpdir(), 'gunmetal-install-policy-'));
  try {
    const directory = await workspace(root);
    await change(directory, root);
    return check(directory);
  } finally { await rm(root, { recursive: true, force: true }); }
}
async function rewrite(path: string, change: (value: Record<string, unknown>) => unknown): Promise<void> {
  const value = change(JSON.parse(await readFile(path, 'utf8')) as Record<string, unknown>);
  await writeFile(path, typeof value === 'string' ? value : JSON.stringify(value));
}

// Verifies: SEC-SUP-033, SEC-SUP-034. A safe file must not conceal an unsafe effective install: each
// override is first shown to change what the pinned pnpm itself reports, then shown to be refused.
for (const [variable, value, key, effective, rule, path, message] of [
  ['pnpm_config_ignore_scripts', 'false', 'ignoreScripts', false, 'SEC-SUP-033', 'effective.ignoreScripts', 'must be true'],
  ['pnpm_config_frozen_lockfile', 'false', 'frozenLockfile', false, 'SEC-SUP-033', 'effective.frozenLockfile', 'must be true'],
  ['pnpm_config_ignore_pnpmfile', 'false', 'ignorePnpmfile', false, 'SEC-SUP-033', 'effective.ignorePnpmfile', 'must be true'],
  ['pnpm_config_strict_dep_builds', 'false', 'strictDepBuilds', false, 'SEC-SUP-033', 'effective.strictDepBuilds', 'must be true'],
  ['pnpm_config_side_effects_cache', 'true', 'sideEffectsCache', true, 'SEC-SUP-033', 'effective.sideEffectsCache', 'must be false'],
  ['pnpm_config_verify_store_integrity', 'false', 'verifyStoreIntegrity', false, 'SEC-SUP-033', 'effective.verifyStoreIntegrity', 'must be true'],
  ['pnpm_config_dangerously_allow_all_builds', 'true', 'dangerouslyAllowAllBuilds', true, 'SEC-SUP-033', 'effective.dangerouslyAllowAllBuilds', 'must be false'],
  ['pnpm_config_block_exotic_subdeps', 'false', 'blockExoticSubdeps', false, 'SEC-SUP-033', 'effective.blockExoticSubdeps', 'must be true'],
  ['pnpm_config_engine_strict', 'false', 'engineStrict', false, 'SEC-SUP-033', 'effective.engineStrict', 'must be true'],
  ['pnpm_config_auto_install_peers', 'true', 'autoInstallPeers', true, 'SEC-SUP-033', 'effective.autoInstallPeers', 'must be false'],
  ['pnpm_config_strict_peer_dependencies', 'false', 'strictPeerDependencies', false, 'SEC-SUP-033', 'effective.strictPeerDependencies', 'must be true'],
  ['pnpm_config_minimum_release_age', '0', 'minimumReleaseAge', 0, 'SEC-SUP-034', 'effective.minimumReleaseAge', 'must be at least 10080 minutes'],
  ['pnpm_config_minimum_release_age_strict', 'false', 'minimumReleaseAgeStrict', false, 'SEC-SUP-034', 'effective.minimumReleaseAgeStrict', 'must be true'],
  ['pnpm_config_trust_policy', 'off', 'trustPolicy', 'off', 'SEC-SUP-034', 'effective.trustPolicy', 'must be no-downgrade'],
  ['pnpm_config_trust_lockfile', 'true', 'trustLockfile', true, 'SEC-SUP-034', 'effective.trustLockfile', 'must be false'],
  // A real redirect of the default registry, as pnpm's own configuration output reports it.
  ['pnpm_config_registry', 'https://evil.test/', 'registry', 'https://evil.test/', 'SEC-SUP-033', 'effective.registries', 'must contain only registry.npmjs.org'],
] as const) {
  test(`workspace check refuses the effective override ${variable}=${value}`, async () => {
    const env = { ...process.env, [variable]: value };
    const observed = spawnSync(await nativePnpm(), ['--dir', client, 'config', 'list', '--json'], { env, encoding: 'utf8', timeout: 10000 });
    assert.deepEqual({ status: observed.status, signal: observed.signal, [key]: observed.status === 0 ? (JSON.parse(observed.stdout) as Record<string, unknown>)[key] : observed.stderr },
      { status: 0, signal: null, [key]: effective });
    assert.deepEqual(check(client, env), refusal(rule, path, message));
  });
}

// Verifies: SEC-SUP-033, SEC-SUP-034, SEC-SUP-035. The release workflow consumes observed workspace facts.
test('actual workspace check observes pinned runtimes, live pnpm age and all manifests', () => {
  assert.deepEqual(check(client), passed);
});

// Verifies: SEC-SUP-011, SEC-SUP-033. Only the verified native manager runs; PATH is never searched for one.
test('a decoy pnpm first on PATH is never executed by a successful workspace check', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-decoy-pnpm-'));
  try {
    const { env, script, marker } = await decoy(directory, 'pnpm');
    assert.deepEqual(check(client, env), passed);
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
    const { env, marker } = await decoy(directory, 'pnpm');
    const { GUNMETAL_NATIVE_PNPM_ROOT: _root, ...withoutRoot } = env;
    assert.deepEqual(check(client, withoutRoot), refusal('SEC-SUP-011', 'runtime.pnpm', 'native pnpm root is required'));
    await assert.rejects(stat(marker), { code: 'ENOENT' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

// Verifies: SEC-SUP-035. A workspace whose reason list is absent cannot pass.
test('actual workspace check refuses a workspace without its dependency reason list', async () => {
  assert.deepEqual(await changed((_directory, root) => rm(join(root, 'supply-chain'), { recursive: true })),
    refusal('SEC-SUP-035', 'supply-chain/js-direct-deps.toml', 'dependency reason list is unavailable'));
});

// Verifies: SEC-SUP-033. The workspace file is read strictly, as one document that names its members.
for (const [name, write, path, message] of [
  ['workspace settings that are not YAML', () => 'packages: [\n', 'workspace', 'invalid or ambiguous YAML'],
  ['two workspace documents', (settings: Record<string, unknown>) => `${JSON.stringify(settings)}\n---\n${JSON.stringify(settings)}\n`, 'workspace', 'invalid or ambiguous YAML'],
  ['a merge key in the workspace settings', (settings: Record<string, unknown>) => `${JSON.stringify(settings).slice(0, -1)}, <<: {"patchedDependencies": {"yaml@2.9.1": "./patch.diff"}}}\n`, 'workspace', 'invalid or ambiguous YAML'],
  ['a directive before the workspace settings', (settings: Record<string, unknown>) => `%YAML 1.1\n---\n${JSON.stringify(settings)}\n`, 'workspace', 'invalid or ambiguous YAML'],
  ['workspace settings without members', ({ packages: _packages, ...settings }: Record<string, unknown>) => settings, 'workspace.packages', 'workspace members must be declared'],
  ['a workspace member that is not a string', (settings: Record<string, unknown>) => ({ ...settings, packages: ['packages/*', 5] }), 'workspace.packages', 'workspace members must be declared'],
] as const) {
  test(`actual workspace check refuses ${name}`, async () => {
    assert.deepEqual(await changed(directory => rewrite(join(directory, 'pnpm-workspace.yaml'), write)), refusal('SEC-SUP-033', path, message));
  });
}

for (const [scenario, file, change, rule, path, message] of [
  ['scripts', 'pnpm-workspace.yaml', (settings: Record<string, unknown>) => ({ ...settings, ignoreScripts: false }), 'SEC-SUP-033', 'workspace.ignoreScripts', 'must be true'],
  ['peer', 'package.json', (manifest: Record<string, unknown>) => ({ ...manifest, peerDependencies: { yaml: '2.9.1' } }), 'SEC-SUP-035', 'manifest.peerDependencies.yaml', 'only a known workspace member with workspace:* is allowed'],
  ['runtime', 'package.json', (manifest: Record<string, unknown>) => ({ ...manifest, engines: { node: '24.19.0', pnpm: '12.7.0' } }), 'SEC-SUP-011', 'runtime.node', 'observed runtime must match the manifest pin'],
  // Verifies: SEC-SUP-011. The verified manager must be the one both manifest fields pin.
  ['manager engine', 'package.json', (manifest: Record<string, unknown>) => ({ ...manifest, engines: { node: '24.20.0', pnpm: '12.6.0' } }), 'SEC-SUP-011', 'runtime.pnpm', 'observed runtime must match the manifest pin'],
  ['packageManager', 'package.json', (manifest: Record<string, unknown>) => ({ ...manifest, packageManager: 'pnpm@12.6.0' }), 'SEC-SUP-011', 'runtime.pnpm', 'observed runtime must match the manifest pin'],
] as const) {
  test(`actual workspace check refuses a changed ${scenario} boundary`, async () => {
    assert.deepEqual(await changed(directory => rewrite(join(directory, file), change)), refusal(rule, path, message));
  });
}

// Verifies: SEC-SUP-033. Transitive identities must be checked before installation.
for (const index of [0, 1]) {
  test(`initial registry routing refuses a transitive alternative registry in lock document ${index}`, async () => {
    assert.deepEqual(await changed(async directory => {
      const path = join(directory, 'pnpm-lock.yaml');
      const documents = parseAllDocuments(await readFile(path, 'utf8')).map(document => document.toJS());
      assert.equal(documents.length, 2);
      documents[index].packages['@jsr/std@1.0.0'] = { resolution: { integrity: `sha512-${'A'.repeat(86)}==` } };
      await writeFile(path, documents.map(document => stringify(document)).join('\n---\n'));
    }), refusal('SEC-SUP-033', 'effective.@jsr/std', 'only registry.npmjs.org is allowed'));
  });
}

// Verifies: SEC-SUP-033. With no registry dependency, pnpm writes a project document that has
// importers only; the check reads that document as it is.
test('actual workspace check accepts a project document with importers only', async () => {
  assert.deepEqual(await changed(async (directory, root) => {
    await rewrite(join(directory, 'package.json'), ({ devDependencies: _dependencies, ...manifest }) => manifest);
    await writeFile(join(root, 'supply-chain/js-direct-deps.toml'), '# No direct dependency.\n');
    const path = join(directory, 'pnpm-lock.yaml');
    const lock = await readFile(path, 'utf8');
    await writeFile(path, `${lock.slice(0, lock.indexOf('\n---\n', 4))}\n---\nlockfileVersion: '9.0'\n\nsettings:\n  autoInstallPeers: false\n  excludeLinksFromLockfile: false\n\nimporters:\n\n  .: {}\n`);
  }), passed);
});

// Runs the exported check in a child process with stand-ins for the registry request and for the manager's
// own `--version` and `config list`, answers the real registry and the verified manager cannot be made to give.
// The child is the fixed program fixtures/seamed-check.ts. Each test's stand-ins are data, handed to it as
// one JSON argument; no program text is assembled here.
type Answer = { status: number | null; signal: string | null; stdout: string; stderr: string };
function seamed(standIns: { request?: { status: number; body: unknown }; run?: { argument: string; answer: Answer } } = {}): unknown {
  const harness = fileURLToPath(new URL('fixtures/seamed-check.ts', import.meta.url));
  const result = spawnSync(process.execPath, [harness, JSON.stringify({ directory: client, ...standIns })], { encoding: 'utf8', timeout: 120000 });
  let value: unknown = result.stdout;
  try { value = JSON.parse(result.stdout); } catch { /* Left as text, so a failure shows what was printed. */ }
  return { status: result.status, signal: result.signal, stderr: result.stderr, result: value };
}
function reported(rule: string, path: string, message: string): unknown {
  return { status: 0, signal: null, stderr: '', result: [{ rule, path, message }] };
}
// Positive control: with nothing replaced, the same harness reports the real workspace as passing.
test('the exported workspace check passes the real workspace when nothing is replaced', () => {
  assert.deepEqual(seamed(), { status: 0, signal: null, stderr: '', result: passed.result });
});
// Verifies: SEC-SUP-034. The manager's publication time must be known from the registry and old enough.
for (const [name, request] of [
  ['a registry that fails', { status: 503, body: null }],
  ['registry metadata without publication times', { status: 200, body: { time: {} } }],
  ['registry metadata that is not an object', { status: 200, body: [] }],
  ['a publication time in the future', { status: 200, body: { time: { '12.7.0': '2999-01-01T00:00:00.000Z' } } }],
] as const) {
  test(`workspace check refuses ${name}`, () => {
    assert.deepEqual(seamed({ request }), reported('SEC-SUP-034', 'pnpm.publication', 'publication must be known and at least seven days old'));
  });
}
// Verifies: SEC-SUP-011, SEC-SUP-033. What the manager cannot report is never assumed.
for (const [name, argument, answer, rule, path, message] of [
  ['a manager whose version command fails', '--version', { status: 1, signal: null, stdout: '12.7.0\n', stderr: '' }, 'SEC-SUP-011', 'runtime.pnpm', 'observed runtime must match the manifest pin'],
  ['a manager whose version command reports a signal', '--version', { status: 0, signal: 'SIGKILL', stdout: '12.7.0\n', stderr: '' }, 'SEC-SUP-011', 'runtime.pnpm', 'observed runtime must match the manifest pin'],
  ['a manager that reports another version', '--version', { status: 0, signal: null, stdout: '12.7.1\n', stderr: '' }, 'SEC-SUP-011', 'runtime.pnpm', 'observed runtime must match the manifest pin'],
  ['effective settings the manager fails to list', 'config', { status: 1, signal: null, stdout: '{}', stderr: '' }, 'SEC-SUP-033', 'effective', 'effective settings could not be read'],
  ['effective settings from a manager that reports a signal', 'config', { status: 0, signal: 'SIGKILL', stdout: '{}', stderr: '' }, 'SEC-SUP-033', 'effective', 'effective settings could not be read'],
  ['effective settings that are not JSON', 'config', { status: 0, signal: null, stdout: 'not json', stderr: '' }, 'SEC-SUP-033', 'effective', 'effective settings could not be read'],
  ['effective settings that are not an object', 'config', { status: 0, signal: null, stdout: '[]', stderr: '' }, 'SEC-SUP-033', 'effective', 'effective settings could not be read'],
] as const) {
  test(`workspace check refuses ${name}`, () => {
    assert.deepEqual(seamed({ run: { argument, answer } }), reported(rule, path, message));
  });
}
