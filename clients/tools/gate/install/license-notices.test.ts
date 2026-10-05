import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { inspect } from './policy.ts';

const notice = 'This Source Code Form is “Incompatible With Secondary Licenses”';
const refusal = [{ rule: 'SEC-SUP-029', path: 'licenses.yaml', message: 'licence MPL-2.0 marked incompatible with secondary licences is not allowed' }];
function report(licenseText: string): unknown {
  return { allowed: ['MPL-2.0'], report: { 'MPL-2.0': [{ name: 'yaml', versions: ['2.9.1'], paths: ['/installed/yaml'], license: 'MPL-2.0', licenseText }] } };
}
// Verifies: SEC-SUP-029. A template Exhibit B is distinct from an applied declaration.
test('the standard full MPL licence template does not itself mark software incompatible', async () => {
  assert.deepEqual(inspect('licenses', report(await readFile(new URL('fixtures/mpl-2.0.txt', import.meta.url), 'utf8'))), []);
});
test('an applied typographic MPL declaration is refused despite line wrapping', () => {
  assert.deepEqual(inspect('licenses', report(notice.replace('Code Form', 'Code\nForm'))), refusal);
});
test('a canonical template plus an appended declaration still refuses the declaration', async () => {
  const template = await readFile(new URL('fixtures/mpl-2.0.txt', import.meta.url), 'utf8');
  assert.deepEqual(inspect('licenses', report(`${template}\n${notice}\n`)), refusal);
});
// The applied declaration as a source comment: fixed text, written out as it stands.
const declared = '/* This Source Code Form is \u201cIncompatible With Secondary Licenses\u201d */\n';
for (const file of ['NOTICE.txt', 'src/module.js', 'linked-source']) {
  test(`actual installed MPL declaration in ${file} is collected and refused`, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'gunmetal-mpl-notice-'));
    try {
      const client = fileURLToPath(new URL('../../../', import.meta.url));
      const project = join(directory, 'clients');
      await mkdir(project);
      await cp(join(client, '../deny.toml'), join(directory, 'deny.toml'));
      for (const input of ['package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml']) await cp(join(client, input), join(project, input));
      await cp(join(client, 'node_modules'), join(project, 'node_modules'), { recursive: true });
      const packageDirectory = join(project, 'node_modules/.pnpm/yaml@2.9.1/node_modules/yaml');
      const manifestPath = join(packageDirectory, 'package.json');
      const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as Record<string, unknown>;
      await writeFile(manifestPath, JSON.stringify({ ...manifest, license: 'MPL-2.0' }));
      await cp(fileURLToPath(new URL('fixtures/mpl-2.0.txt', import.meta.url)), join(packageDirectory, 'LICENSE'));
      await mkdir(join(packageDirectory, 'src'), { recursive: true });
      if (file === 'linked-source') {
        const source = join(directory, 'source');
        await mkdir(source);
        await writeFile(join(source, 'module.js'), declared);
        await symlink(source, join(packageDirectory, file));
      } else await writeFile(join(packageDirectory, file), declared);
      const result = spawnSync(process.execPath, [fileURLToPath(new URL('licenses.ts', import.meta.url)), project], { encoding: 'utf8', timeout: 120000 });
      assert.deepEqual({ status: result.status, signal: result.signal, stderr: result.stderr,
        result: result.stdout.trim() === '' ? null : JSON.parse(result.stdout) }, { status: 1, signal: null, stderr: '', result:
          file === 'linked-source' ? [{ rule: 'SEC-SUP-029', path: 'licenses.yaml', message: 'MPL source symlinks require review' }] : refusal });
    } finally { await rm(directory, { recursive: true, force: true }); }
  });
}
