import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, stat, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { decoy } from './decoy.ts';

const client = fileURLToPath(new URL('../../../', import.meta.url));
function check(directory: string, env = process.env): unknown {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('licenses.ts', import.meta.url)), directory], { env, encoding: 'utf8', timeout: 120000 });
  return { status: result.status, signal: result.signal, stderr: result.stderr, result: result.stdout.trim() === '' ? null : JSON.parse(result.stdout) };
}
const allowed = ['0BSD', 'AGPL-3.0-or-later', 'Apache-2.0', 'Apache-2.0 WITH LLVM-exception', 'BSD-2-Clause', 'BSD-3-Clause', 'BSL-1.0', 'CC0-1.0', 'GPL-3.0-or-later', 'ISC', 'LGPL-2.1-or-later', 'LGPL-3.0-or-later', 'MIT', 'MPL-2.0', 'Unicode-3.0', 'Unlicense', 'Zlib'];

// Verifies: SEC-SUP-029. The actual project allow-list and all installed tooling are checked together.
test('real licence collection covers the project and physically installed manager against deny.toml', () => {
  assert.deepEqual(check(client), { status: 0, signal: null, stderr: '', result: {
    allowed,
    packages: [
      { name: '@pnpm/exe.linux-x64', version: '12.7.0', license: 'MIT' },
      { name: 'yaml', version: '2.9.1', license: 'ISC' },
    ],
  } });
});

// Verifies: SEC-SUP-029. Where the manager root lives must not change the inventory it is matched against.
test('real licence collection gives the same result when the native manager root is reached through a symlink', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-linked-manager-'));
  try {
    const linked = join(directory, 'native-pnpm');
    await symlink(String(process.env.GUNMETAL_NATIVE_PNPM_ROOT), linked);
    assert.deepEqual(check(client, { ...process.env, GUNMETAL_NATIVE_PNPM_ROOT: linked }), { status: 0, signal: null, stderr: '', result: {
      allowed,
      packages: [
        { name: '@pnpm/exe.linux-x64', version: '12.7.0', license: 'MIT' },
        { name: 'yaml', version: '2.9.1', license: 'ISC' },
      ],
    } });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

// Verifies: SEC-SUP-011. The manager refusal reaches the licence collector's result as itself.
test('without a native manager root the licence collector reports that refusal', () => {
  const { GUNMETAL_NATIVE_PNPM_ROOT: _root, ...withoutRoot } = process.env;
  assert.deepEqual(check(client, withoutRoot), { status: 1, signal: null, stderr: '', result: [{
    rule: 'SEC-SUP-011', path: 'runtime.pnpm', message: 'native pnpm root is required',
  }] });
});

for (const license of ['SSPL-1.0', undefined]) {
  test(`actual installed licence metadata refuses ${license ?? 'missing licence'}`, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'gunmetal-installed-licence-'));
    try {
      const project = join(directory, 'clients');
      await mkdir(project);
      await cp(join(client, '../deny.toml'), join(directory, 'deny.toml'));
      for (const file of ['package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml']) await cp(join(client, file), join(project, file));
      await cp(join(client, 'node_modules'), join(project, 'node_modules'), { recursive: true });
      const path = join(project, 'node_modules/.pnpm/yaml@2.9.1/node_modules/yaml/package.json');
      const manifest = JSON.parse(await readFile(path, 'utf8')) as Record<string, unknown>;
      await writeFile(path, JSON.stringify({ ...manifest, license }));
      assert.deepEqual(check(project), { status: 1, signal: null, stderr: '', result: [{
        rule: 'SEC-SUP-029', path: 'licenses.yaml', message: `licence ${license ?? 'UNKNOWN'} is not allowed`,
      }] });
    } finally { await rm(directory, { recursive: true, force: true }); }
  });
}

// Verifies: SEC-SUP-029. The allow-list is read in-process and only in the one form deny.toml uses:
// a `[licenses]` table whose `allow` lists one double-quoted licence per line. Anything else fails closed.
for (const [name, policy] of [
  ['no policy file', null],
  ['an unterminated list', '[licenses]\nallow = [\n'],
  ['no licence table', '[bans]\nallow = [\n    "MIT",\n]\n'],
  ['two licence tables', '[licenses]\nallow = [\n    "MIT",\n]\n[licenses]\nallow = [\n    "ISC",\n]\n'],
  ['two allow lists', '[licenses]\nallow = [\n    "MIT",\n]\nallow = [\n    "ISC",\n]\n'],
  ['an allow list only in another table', '[licenses]\nconfidence-threshold = 0.9\n[bans]\nallow = [\n    "MIT",\n]\n'],
  ['a dotted key instead of the table', 'licenses.allow = [\n    "MIT",\n]\n'],
  ['a one-line list', '[licenses]\nallow = ["MIT"]\n'],
  ['a key without spaces', '[licenses]\nallow=[\n    "MIT",\n]\n'],
  ['a quoted key', '[licenses]\n"allow" = [\n    "MIT",\n]\n'],
  ['a literal string', "[licenses]\nallow = [\n    'MIT',\n]\n"],
  ['an escape in a licence', '[licenses]\nallow = [\n    "MI\\u0054",\n]\n'],
  ['a comment after a licence', '[licenses]\nallow = [\n    "MIT", # permissive\n]\n'],
  ['a comment line inside the list', '[licenses]\nallow = [\n    # permissive\n    "MIT",\n]\n'],
  ['a licence without its comma', '[licenses]\nallow = [\n    "MIT"\n]\n'],
  ['two licences on one line', '[licenses]\nallow = [\n    "MIT", "ISC",\n]\n'],
  ['a multi-line string anywhere in the file', 'note = """\n[licenses]\n"""\n[licenses]\nallow = [\n    "MIT",\n]\n'],
  ['a literal multi-line string anywhere in the file', "note = '''\ntext\n'''\n[licenses]\nallow = [\n    \"MIT\",\n]\n"],
  ['an empty list', '[licenses]\nallow = [\n]\n'],
  ['an empty licence', '[licenses]\nallow = [\n    "",\n]\n'],
  ['a repeated licence', '[licenses]\nallow = [\n    "MIT",\n    "MIT",\n]\n'],
] as const) {
  test(`a project licence policy with ${name} fails closed`, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'gunmetal-licence-policy-'));
    try {
      const project = join(directory, 'clients');
      await mkdir(project);
      if (policy !== null) await writeFile(join(directory, 'deny.toml'), policy);
      assert.deepEqual(check(project), { status: 1, signal: null, stderr: '', result: [{
        rule: 'SEC-SUP-029', path: 'licenses.policy', message: 'invalid project licence allow-list',
      }] });
    } finally { await rm(directory, { recursive: true, force: true }); }
  });
}

// Verifies: SEC-SUP-029. No program found on PATH decides which licences are allowed.
test('a decoy python3 first on PATH is never executed by the licence collector', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-decoy-python-'));
  try {
    const { env, script, marker } = await decoy(directory, 'python3');
    assert.deepEqual(check(client, env), { status: 0, signal: null, stderr: '', result: {
      allowed,
      packages: [
        { name: '@pnpm/exe.linux-x64', version: '12.7.0', license: 'MIT' },
        { name: 'yaml', version: '2.9.1', license: 'ISC' },
      ],
    } });
    await assert.rejects(stat(marker), { code: 'ENOENT' });
    // Positive control: the decoy does leave its marker once something executes it.
    const control = spawnSync(script, [], { encoding: 'utf8', timeout: 10000 });
    assert.deepEqual({ status: control.status, signal: control.signal, marker: await readFile(marker, 'utf8') },
      { status: 0, signal: null, marker: 'ran' });
  } finally { await rm(directory, { recursive: true, force: true }); }
});
