import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const client = fileURLToPath(new URL('../../../', import.meta.url));
function check(directory: string): unknown {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('licenses.ts', import.meta.url)), directory], { encoding: 'utf8', timeout: 120000 });
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

test('a malformed project licence allow-list fails closed', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-licence-policy-'));
  try {
    const project = join(directory, 'clients');
    await mkdir(project);
    await writeFile(join(directory, 'deny.toml'), '[licenses]\nallow = [\n');
    assert.deepEqual(check(project), { status: 1, signal: null, stderr: '', result: [{
      rule: 'SEC-SUP-029', path: 'licenses.policy', message: 'invalid project licence allow-list',
    }] });
  } finally { await rm(directory, { recursive: true, force: true }); }
});
