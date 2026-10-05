import { spawnSync } from 'node:child_process';
import { cp, mkdtemp, readFile, readdir, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseAllDocuments } from 'yaml';
import { inspect, type Finding } from './policy.ts';
import { nativePnpm } from './native-pnpm.ts';

type Identity = { name: string; version: string; integrity: string };
export type Installed = Identity & { path: string };
type ObjectValue = Record<string, unknown>;
export class Refusal extends Error {
  findings: Finding[];
  constructor(findings: Finding[]) { super('installation verification refused'); this.findings = findings; }
}
function object(value: unknown): value is ObjectValue {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function refuse(path: string, message: string): never {
  throw new Refusal([{ rule: 'SEC-SUP-036', path, message }]);
}
function requirePolicy(kind: string, input: unknown): void {
  const findings = inspect(kind, input);
  if (findings.length !== 0) throw new Refusal(findings);
}
function command(executable: string, args: string[], cwd: string, env = process.env): unknown {
  const result = spawnSync(executable, args, { cwd, env, encoding: 'utf8', timeout: 120000 });
  if (result.status !== 0 || result.signal !== null) refuse('verification.execution', 'registry verification command failed');
  try { return JSON.parse(result.stdout) as unknown; }
  catch { return refuse('verification.execution', 'registry verification command returned invalid JSON'); }
}
async function manifest(directory: string): Promise<ObjectValue> {
  const value: unknown = JSON.parse(await readFile(join(directory, 'package.json'), 'utf8'));
  if (!object(value)) refuse('verification.installation', 'installed package identity must match its locked registry identity');
  return value;
}
async function installedPaths(directory: string): Promise<string[]> {
  const paths: string[] = [];
  const store = join(directory, 'node_modules/.pnpm');
  for (const entry of await readdir(store, { withFileTypes: true })) {
    if (!entry.isDirectory() || entry.name === 'node_modules') continue;
    const modules = join(store, entry.name, 'node_modules');
    for (const child of await readdir(modules, { withFileTypes: true })) {
      if (!child.isDirectory() || child.name.startsWith('.')) continue;
      if (child.name.startsWith('@')) {
        for (const scoped of await readdir(join(modules, child.name), { withFileTypes: true })) {
          if (scoped.isDirectory()) paths.push(join(modules, child.name, scoped.name));
        }
      } else paths.push(join(modules, child.name));
    }
  }
  return paths;
}

export async function collectInstalled(directory: string): Promise<{ installed: Installed[]; manager: string; locked: Map<string, Identity> }> {
  const text = await readFile(join(directory, 'pnpm-lock.yaml'), 'utf8');
  requirePolicy('lockfile', { text });
  const locked = new Map<string, Identity>();
  for (const document of parseAllDocuments(text)) {
    const value = document.toJS() as { packages: Record<string, { resolution: { integrity: string } }> };
    for (const [key, entry] of Object.entries(value.packages)) {
      const boundary = key.lastIndexOf('@');
      locked.set(key, { name: key.slice(0, boundary), version: key.slice(boundary + 1), integrity: entry.resolution.integrity });
    }
  }
  const manager = await nativePnpm();
  const paths = [...await installedPaths(directory), dirname(manager)];
  const packages = new Map<string, Installed>();
  for (const path of paths) {
    const value = await manifest(path);
    const identity = locked.get(`${String(value.name)}@${String(value.version)}`);
    if (identity === undefined) refuse('verification.installation', 'installed package identity must match its locked registry identity');
    packages.set(`${identity.name}@${identity.version}`, { ...identity, path: await realpath(path) });
  }
  const installed = [...packages.values()].sort((left, right) => left.name < right.name ? -1 : left.name > right.name ? 1 : left.version.localeCompare(right.version));
  const config = command(manager, ['--dir', directory, 'config', 'list', '--json'], directory);
  requirePolicy('effective-registries', { config, names: installed.map(entry => entry.name) });
  return { installed, manager, locked };
}

async function verify(directory: string): Promise<unknown> {
  const { installed, locked } = await collectInstalled(directory);
  const present: { name: string; version: string }[] = [];
  for (const entry of installed) {
    const response = await fetch(`https://registry.npmjs.org/${encodeURIComponent(entry.name)}/${encodeURIComponent(entry.version)}`, {
      signal: AbortSignal.timeout(30000),
    });
    if (!response.ok) refuse('verification.registry', 'registry metadata is unavailable');
    const metadata: unknown = await response.json();
    if (!object(metadata) || metadata.name !== entry.name || metadata.version !== entry.version ||
        !object(metadata.dist) || metadata.dist.integrity !== entry.integrity) {
      refuse('verification.inventory', 'verifier inventory must exactly match installed package identities and integrity');
    }
    if (metadata.dist.attestations !== undefined && metadata.dist.attestations !== null) present.push({ name: entry.name, version: entry.version });
  }
  const projection = await mkdtemp(join(tmpdir(), 'gunmetal-signature-verifier-'));
  try {
    const { mkdir } = await import('node:fs/promises');
    await mkdir(join(projection, 'node_modules'));
    const dependencies: Record<string, string> = {};
    for (const [index, entry] of installed.entries()) {
      const alias = `verified-${index}`;
      dependencies[alias] = `npm:${entry.name}@${entry.version}`;
      // External symlinks make npm treat a package's author devDependencies as a development root.
      // A byte copy preserves the observed installed manifest and normal registry-package semantics.
      await cp(entry.path, join(projection, 'node_modules', alias), { recursive: true });
    }
    await writeFile(join(projection, 'package.json'), JSON.stringify({ name: 'gunmetal-signature-verifier', version: '1.0.0', private: true, dependencies }));
    const scopes = [...new Set(installed.filter(entry => entry.name.startsWith('@')).map(entry => entry.name.slice(0, entry.name.indexOf('/'))))];
    await writeFile(join(projection, '.npmrc'), ['registry=https://registry.npmjs.org/', ...scopes.map(scope => `${scope}:registry=https://registry.npmjs.org/`)].join('\n'));
    await writeFile(join(projection, 'empty.npmrc'), '');
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !/^(?:npm_config_|pnpm_config_|node_options$|node_path$)/i.test(key)));
    const npm = join(dirname(process.execPath), '../lib/node_modules/npm/bin/npm-cli.js');
    const options = ['--registry=https://registry.npmjs.org/', `--userconfig=${join(projection, 'empty.npmrc')}`, `--cache=${join(projection, 'cache')}`, '--ignore-scripts'];
    const listing = command(process.execPath, [npm, ...options, 'ls', '--all', '--json', '--long'], projection, env);
    const verified: Identity[] = [];
    if (!object(listing) || !object(listing.dependencies)) refuse('verification.inventory', 'verifier inventory must exactly match installed package identities and integrity');
    for (const value of Object.values(listing.dependencies)) {
      if (!object(value)) refuse('verification.inventory', 'verifier inventory must exactly match installed package identities and integrity');
      const identity = locked.get(`${String(value.name)}@${String(value.version)}`);
      if (identity === undefined) refuse('verification.inventory', 'verifier inventory must exactly match installed package identities and integrity');
      verified.push(identity);
    }
    requirePolicy('verification-inventory', { installed, verified });
    const report = command(process.execPath, [npm, ...options, 'audit', 'signatures', '--json', '--include-attestations'], projection, env);
    requirePolicy('signature-report', { present, report });
    return {
      installed: installed.map(({ name, version, integrity }) => ({ name, version, integrity })),
      verified: verified.sort((left, right) => left.name < right.name ? -1 : left.name > right.name ? 1 : left.version.localeCompare(right.version)),
      signatures: { invalid: [], missing: [], provenance: present },
    };
  } finally {
    await rm(projection, { recursive: true, force: true });
  }
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
try {
  const directory = process.argv[2];
  if (directory === undefined) refuse('verification.execution', 'client workspace directory is required');
  process.stdout.write(`${JSON.stringify(await verify(directory))}\n`);
} catch (error) {
  const findings = error instanceof Refusal ? error.findings : [{ rule: 'SEC-SUP-036', path: 'verification.execution', message: 'installation verification could not complete' }];
  process.stdout.write(`${JSON.stringify(findings)}\n`);
  process.exitCode = 1;
}
}
