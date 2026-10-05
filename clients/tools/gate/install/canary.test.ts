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
const names = ['gunmetal-implicit-gyp-canary', 'gunmetal-install-canary'];
// What this test itself finds on disk for one install: which canaries are really there, and what they left.
async function observed(project: string): Promise<unknown> {
  const installed: string[] = [];
  for (const name of names) {
    const manifest = await marker(join(project, 'node_modules', name, 'package.json'));
    if (manifest !== null && (JSON.parse(manifest) as { name?: unknown }).name === name) installed.push(name);
  }
  return { installed, preinstall: await marker(join(project, 'preinstall.marker')), implicitBuild: await marker(join(project, 'gyp.marker')) };
}
const committed = async (): Promise<Record<string, unknown>> =>
  JSON.parse(await readFile(new URL('../../../pnpm-workspace.yaml', import.meta.url), 'utf8')) as Record<string, unknown>;
// Runs the canary in place, or from a copy of the tools beside the given workspace settings.
async function canary(directory: string, settings?: string, env = process.env): Promise<{ status: number | null; signal: string | null; stderr: string; verdict: unknown; secure: unknown; control: unknown }> {
  let helper = fileURLToPath(new URL('canary.ts', import.meta.url));
  if (settings !== undefined) {
    const client = join(directory, 'clients');
    const tools = join(client, 'tools/gate/install');
    await mkdir(tools, { recursive: true });
    await mkdir(join(client, 'node_modules'));
    await symlink(await realpath(fileURLToPath(new URL('../../../node_modules/yaml', import.meta.url))), join(client, 'node_modules/yaml'));
    await writeFile(join(client, 'package.json'), JSON.stringify({ private: true, type: 'module' }));
    await writeFile(join(client, 'pnpm-workspace.yaml'), settings);
    for (const file of ['canary.ts', 'native-pnpm.ts', 'policy.ts', 'fixtures']) await cp(fileURLToPath(new URL(file, import.meta.url)), join(tools, file), { recursive: true });
    helper = join(tools, 'canary.ts');
  }
  const run = spawnSync(process.execPath, [helper, join(directory, 'run')], { env, encoding: 'utf8', timeout: 120000 });
  return {
    status: run.status, signal: run.signal, stderr: run.stderr, verdict: run.stdout.trim() === '' ? null : JSON.parse(run.stdout),
    secure: await observed(join(directory, 'run', 'secure')), control: await observed(join(directory, 'run', 'control')),
  };
}
const quiet = { installed: names, preinstall: null, implicitBuild: null };
const ran = { installed: names, preinstall: 'preinstall ran', implicitBuild: 'implicit node-gyp ran' };

// Verifies: SEC-SUP-033, SEC-CLI-018
// The canary reports its own verdict, and a canary whose fixtures cannot execute fails the positive control.
for (const format of ['committed', 'ordinary YAML']) {
  test(`frozen installs suppress both scripts with ${format} settings and prove both controls execute`, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'gunmetal-install-canary-'));
    try {
      assert.deepEqual(await canary(directory, format === 'committed' ? undefined : stringify(await committed())), {
        status: 0, signal: null, stderr: '', verdict: { secure: quiet, control: ran, findings: [] }, secure: quiet, control: ran,
      });
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
}

// Verifies: SEC-SUP-033. The secure install runs under the workspace settings as committed, so loosening them fails the canary.
test('workspace settings that let install scripts run turn the canary red', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-install-canary-'));
  try {
    const loosened = { ...await committed(), ignoreScripts: false, allowBuilds: {
      'gunmetal-install-canary@file:../gunmetal-install-canary-1.0.0.tgz': true,
      'gunmetal-implicit-gyp-canary@file:../gunmetal-implicit-gyp-canary-1.0.0.tgz': true,
    } };
    const message = 'the committed settings must install the canaries without running their scripts';
    assert.deepEqual(await canary(directory, JSON.stringify(loosened)), {
      status: 1, signal: null, stderr: '', verdict: { secure: ran, control: ran, findings: [
        { rule: 'SEC-SUP-033', path: 'canary.secure.preinstall', message },
        { rule: 'SEC-SUP-033', path: 'canary.secure.implicitBuild', message },
      ] }, secure: ran, control: ran,
    });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

// Verifies: SEC-SUP-033, SEC-CLI-018. A control that cannot run its scripts proves nothing, and the canary says so.
test('a positive control that is kept from running its scripts turns the canary red', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-install-canary-'));
  try {
    const message = 'the positive control must install the canaries and run their scripts';
    assert.deepEqual(await canary(directory, undefined, { ...process.env, pnpm_config_ignore_scripts: 'true' }), {
      status: 1, signal: null, stderr: '', verdict: { secure: quiet, control: quiet, findings: [
        { rule: 'SEC-SUP-033', path: 'canary.control.preinstall', message },
        { rule: 'SEC-SUP-033', path: 'canary.control.implicitBuild', message },
      ] }, secure: quiet, control: quiet,
    });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
