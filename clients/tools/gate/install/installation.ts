import { spawnSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { glob, readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inspect, parseYaml } from './policy.ts';
import { Refusal } from './verify.ts';
import { nativePnpm } from './native-pnpm.ts';

type ObjectValue = Record<string, unknown>;
function object(value: unknown): value is ObjectValue {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function refuse(rule: string, path: string, message: string): never {
  throw new Refusal([{ rule, path, message }]);
}
function requirePolicy(kind: string, input: unknown): void {
  const findings = inspect(kind, input);
  if (findings.length !== 0) throw new Refusal(findings);
}
// `seams` lets a test stand in for the registry request and for the manager's own answers, which the
// real registry and the verified manager cannot be made to get wrong. The command line passes none.
export async function check(directory: string, seams: { request?: typeof fetch; run?: typeof spawnSync } = {}): Promise<unknown> {
  const request = seams.request ?? fetch;
  const run = seams.run ?? spawnSync;
  let settings: unknown;
  try {
    const documents = parseYaml(await readFile(join(directory, 'pnpm-workspace.yaml'), 'utf8'));
    if (documents.length !== 1) throw new Error('invalid YAML');
    settings = documents[0];
  } catch { return refuse('SEC-SUP-033', 'workspace', 'invalid or ambiguous YAML'); }
  requirePolicy('settings', settings);
  if (!object(settings) || !Array.isArray(settings.packages) || settings.packages.some(value => typeof value !== 'string')) {
    refuse('SEC-SUP-033', 'workspace.packages', 'workspace members must be declared');
  }
  const lockText = await readFile(join(directory, 'pnpm-lock.yaml'), 'utf8');
  requirePolicy('lockfile', { text: lockText });
  // A document that locks no registry package has no `packages` at all.
  const lockedNames = parseYaml(lockText).flatMap(document =>
    Object.keys((document as { packages?: ObjectValue }).packages ?? {}).map(key => key.slice(0, key.lastIndexOf('@'))));
  const manifests: string[] = [];
  const values = new Map<string, ObjectValue>();
  const options = { cwd: directory, exclude: ['**/node_modules/**'], followSymlinks: false };
  for await (const path of glob('**/package.json', options)) {
    const value: unknown = JSON.parse(await readFile(join(directory, path), 'utf8'));
    if (!object(value)) refuse('SEC-SUP-035', 'manifest', 'invalid manifest dependency groups');
    manifests.push(path);
    values.set(path, value);
  }
  const members: unknown[] = [];
  for await (const path of glob(settings.packages.map(pattern => `${pattern}/package.json`), options)) {
    const name = values.get(path)?.name;
    if (typeof name === 'string') members.push(name);
  }
  for (const value of values.values()) requirePolicy('manifest', { manifest: value, members });
  let reasons: string;
  try { reasons = await readFile(join(dirname(directory), 'supply-chain/js-direct-deps.toml'), 'utf8'); }
  catch { return refuse('SEC-SUP-035', 'supply-chain/js-direct-deps.toml', 'dependency reason list is unavailable'); }
  requirePolicy('direct-dependencies', { manifests: [...values.entries()].map(([path, manifest]) => ({ path, manifest })), list: reasons });
  const root = values.get('package.json');
  const engines = root?.engines;
  const node = process.versions.node;
  if (!object(engines) || engines.node !== node) refuse('SEC-SUP-011', 'runtime.node', 'observed runtime must match the manifest pin');
  const manager = await nativePnpm();
  const result = run(manager, ['--version'], { encoding: 'utf8', timeout: 10000 });
  const pnpm = result.stdout.trim();
  if (result.status !== 0 || result.signal !== null || engines.pnpm !== pnpm || root?.packageManager !== `pnpm@${pnpm}`) {
    refuse('SEC-SUP-011', 'runtime.pnpm', 'observed runtime must match the manifest pin');
  }
  const observed = run(manager, ['--dir', directory, 'config', 'list', '--json'], { encoding: 'utf8', timeout: 10000 });
  let config: unknown;
  try { config = observed.status === 0 && observed.signal === null ? JSON.parse(observed.stdout) : null; }
  catch { config = null; }
  if (!object(config)) refuse('SEC-SUP-033', 'effective', 'effective settings could not be read');
  const findings = inspect('settings', { ...config, registries: { default: config.registry } })
    .map(entry => ({ ...entry, path: entry.path.replace(/^workspace\./, 'effective.') }));
  if (findings.length !== 0) throw new Refusal(findings);
  const names = [...new Set([...lockedNames, ...[...values.values()].flatMap(value =>
    ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies'].flatMap(field =>
      object(value[field]) ? Object.entries(value[field]).filter(([, version]) => version !== 'workspace:*').map(([name]) => name) : []))])];
  requirePolicy('effective-registries', { config, names });
  const response = await request('https://registry.npmjs.org/pnpm', { signal: AbortSignal.timeout(30000) });
  if (!response.ok) refuse('SEC-SUP-034', 'pnpm.publication', 'publication must be known and at least seven days old');
  const metadata: unknown = await response.json();
  const pnpmPublication = object(metadata) && object(metadata.time) ? metadata.time[pnpm] : undefined;
  requirePolicy('age', { published: pnpmPublication, now: new Date().toISOString() });
  return { runtime: { node, pnpm }, pnpmPublication, manifests: manifests.sort(), findings: [] };
}

// Run as a program, this checks the directory it is given. Imported, it only offers `check`.
if (process.argv[1] !== undefined && realpathSync(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const directory = process.argv[2];
    if (directory === undefined) refuse('SEC-SUP-033', 'workspace', 'client workspace directory is required');
    process.stdout.write(`${JSON.stringify(await check(directory))}\n`);
  } catch (error) {
    const findings = error instanceof Refusal ? error.findings : [{ rule: 'SEC-SUP-033', path: 'workspace', message: 'workspace check could not complete' }];
    process.stdout.write(`${JSON.stringify(findings)}\n`);
    process.exitCode = 1;
  }
}
