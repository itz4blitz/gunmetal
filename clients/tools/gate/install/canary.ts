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
  await writeFile(join(project, 'pnpm-workspace.yaml'), JSON.stringify({
    ...settings,
    packages: [],
    storeDir: join(project, 'store'),
    sideEffectsCache: false,
    ignoreScripts: mode === 'secure',
    allowBuilds: mode === 'secure' ? {} : {
      'gunmetal-install-canary@file:../gunmetal-install-canary-1.0.0.tgz': true,
      'gunmetal-implicit-gyp-canary@file:../gunmetal-implicit-gyp-canary-1.0.0.tgz': true,
    },
  }));
  await command(['install', '--lockfile-only', '--no-frozen-lockfile', '--ignore-scripts'], project, project);
  await command(['install', '--frozen-lockfile', ...(mode === 'secure' ? ['--ignore-scripts'] : [])], project, project);
}
