import { mkdir, readFile, symlink, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { join, delimiter, dirname } from 'node:path';
import { parseYaml } from './policy.ts';
import { nativePnpm } from './native-pnpm.ts';

const directory = process.argv[2];
if (directory === undefined) throw new Error('a canary directory is required');
const canaryBin = join(directory, 'bin');
await mkdir(canaryBin, { recursive: true });
await symlink(join(dirname(process.execPath), '../lib/node_modules/npm/node_modules/node-gyp/bin/node-gyp.js'), join(canaryBin, 'node-gyp'));
async function command(args: string[], cwd: string, markerDirectory: string): Promise<string> {
  // npm's node-gyp ships inside the exact checksum-pinned Node distribution.
  const result = spawnSync(await nativePnpm(), args, {
    cwd,
    env: {
      ...process.env,
      GUNMETAL_CANARY_MARKERS: markerDirectory,
      npm_config_nodedir: join(dirname(process.execPath), '..'),
      PATH: `${canaryBin}${delimiter}${process.env.PATH ?? ''}`,
    },
    encoding: 'utf8',
    timeout: 60000,
  });
  if (result.status !== 0) throw new Error(`canary command failed in ${cwd} (${args.join(' ')}): ${result.stderr}`);
  return result.stdout;
}
async function read(path: string): Promise<string | null> {
  try { return await readFile(path, 'utf8'); }
  catch { return null; }
}

// Sorted, as the verdict lists them.
const names = ['gunmetal-implicit-gyp-canary', 'gunmetal-install-canary'];
for (const [name, manifest] of [
  ['gunmetal-install-canary', 'canary-package'],
  ['gunmetal-implicit-gyp-canary', 'canary-gyp-package'],
]) {
  const packageDirectory = join(directory, name);
  await mkdir(packageDirectory, { recursive: true });
  await writeFile(join(packageDirectory, 'package.json'), await readFile(new URL(`fixtures/${manifest}.txt`, import.meta.url)));
  await writeFile(join(packageDirectory, 'binding.gyp'), await readFile(new URL('fixtures/canary-binding.txt', import.meta.url)));
  await command(['pack', '--ignore-scripts', '--pack-destination', directory], packageDirectory, directory);
}
const documents = parseYaml(await readFile(new URL('../../../pnpm-workspace.yaml', import.meta.url), 'utf8'));
if (documents.length !== 1) throw new Error('one workspace document is required');
const settings = documents[0] as Record<string, unknown>;
const observed: Record<string, Record<string, unknown>> = {};
for (const mode of ['secure', 'control']) {
  const project = join(directory, mode);
  await mkdir(project, { recursive: true });
  await writeFile(join(project, 'package.json'), JSON.stringify({
    name: `gunmetal-install-${mode}`, private: true,
    dependencies: {
      'gunmetal-install-canary': `file:${join(directory, 'gunmetal-install-canary-1.0.0.tgz')}`,
      'gunmetal-implicit-gyp-canary': `file:${join(directory, 'gunmetal-implicit-gyp-canary-1.0.0.tgz')}`,
    },
  }));
  // The secure install runs under the workspace settings exactly as committed: only the member list and a
  // private store are replaced, and no flag is added, so a committed file that lets scripts run turns this
  // canary red. The control then switches scripts on for these two packages, to prove they do leave their
  // markers when they are allowed to.
  await writeFile(join(project, 'pnpm-workspace.yaml'), JSON.stringify({
    ...settings,
    packages: [],
    storeDir: join(project, 'store'),
    ...(mode === 'secure' ? {} : {
      ignoreScripts: false,
      allowBuilds: {
        'gunmetal-install-canary@file:../gunmetal-install-canary-1.0.0.tgz': true,
        'gunmetal-implicit-gyp-canary@file:../gunmetal-implicit-gyp-canary-1.0.0.tgz': true,
      },
    }),
  }));
  await command(['install', '--lockfile-only', '--no-frozen-lockfile', '--ignore-scripts'], project, project);
  await command(['install', '--frozen-lockfile'], project, project);
  const installed: string[] = [];
  for (const name of names) {
    const manifest = await read(join(project, 'node_modules', name, 'package.json'));
    if (manifest !== null && (JSON.parse(manifest) as { name?: unknown }).name === name) installed.push(name);
  }
  observed[mode] = { installed, preinstall: await read(join(project, 'preinstall.marker')), implicitBuild: await read(join(project, 'gyp.marker')) };
}

// The verdict is this program's own: what each install left behind, compared with what it must leave.
const required: Record<string, [Record<string, unknown>, string]> = {
  secure: [{ installed: names, preinstall: null, implicitBuild: null }, 'the committed settings must install the canaries without running their scripts'],
  control: [{ installed: names, preinstall: 'preinstall ran', implicitBuild: 'implicit node-gyp ran' }, 'the positive control must install the canaries and run their scripts'],
};
const findings = Object.entries(required).flatMap(([mode, [facts, message]]) => Object.entries(facts)
  .filter(([fact, value]) => JSON.stringify(observed[mode]?.[fact]) !== JSON.stringify(value))
  .map(([fact]) => ({ rule: 'SEC-SUP-033', path: `canary.${mode}.${fact}`, message })));
process.stdout.write(`${JSON.stringify({ ...observed, findings })}\n`);
process.exitCode = findings.length === 0 ? 0 : 1;
