import assert from 'node:assert/strict';
import { cp, mkdir, mkdtemp, readFile, realpath, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { stringify } from 'yaml';

async function marker(path: string): Promise<string | null> {
  try { return await readFile(path, 'utf8'); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return null;
    throw error;
  }
}

// Verifies: SEC-SUP-033, SEC-CLI-018
// A canary whose fixtures cannot execute must fail the positive control.
for (const format of ['committed', 'ordinary YAML']) {
test(`frozen installs suppress both scripts with ${format} settings and prove both controls execute`, async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-install-canary-'));
  try {
    let helper = fileURLToPath(new URL('canary.ts', import.meta.url));
    if (format === 'ordinary YAML') {
      const client = join(directory, 'clients');
      const tools = join(client, 'tools/gate/install');
      await mkdir(tools, { recursive: true });
      await mkdir(join(client, 'node_modules'));
      await symlink(await realpath(fileURLToPath(new URL('../../../node_modules/yaml', import.meta.url))), join(client, 'node_modules/yaml'));
      await writeFile(join(client, 'package.json'), JSON.stringify({ private: true, type: 'module' }));
      await writeFile(join(client, 'pnpm-workspace.yaml'), stringify(JSON.parse(await readFile(new URL('../../../pnpm-workspace.yaml', import.meta.url), 'utf8'))));
      for (const file of ['canary.ts', 'policy.ts', 'fixtures']) await cp(fileURLToPath(new URL(file, import.meta.url)), join(tools, file), { recursive: true });
      helper = join(tools, 'canary.ts');
    }
    const run = spawnSync(process.execPath, [
      helper, join(directory, 'run'),
    ], { encoding: 'utf8', timeout: 120000 });
    assert.deepEqual({ status: run.status, signal: run.signal, error: run.error?.message }, {
      status: 0, signal: null, error: undefined,
    }, run.stderr);
    assert.deepEqual({
      securePreinstall: await marker(join(directory, 'run', 'secure', 'preinstall.marker')),
      secureImplicitBuild: await marker(join(directory, 'run', 'secure', 'gyp.marker')),
      controlPreinstall: await marker(join(directory, 'run', 'control', 'preinstall.marker')),
      controlImplicitBuild: await marker(join(directory, 'run', 'control', 'gyp.marker')),
    }, {
      securePreinstall: null,
      secureImplicitBuild: null,
      controlPreinstall: 'preinstall ran',
      controlImplicitBuild: 'implicit node-gyp ran',
    });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
}
