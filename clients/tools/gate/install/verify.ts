import { spawnSync } from 'node:child_process';
import { cp, mkdtemp, readFile, readdir, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseAllDocuments } from 'yaml';
import { inspect } from './policy.ts';
import { nativePnpm, nativePnpmChecksum, Refusal } from './native-pnpm.ts';

// What this module establishes, and what it does not.
//
// Before any registry request (`collectInstalled`):
// - the lockfile passes the source policy;
// - the native manager is the pinned archive and its executable (`native-pnpm.ts`), and the lockfile's
//   entry for it carries that same pinned checksum;
// - every package directory under `node_modules/.pnpm`, and the manager, names an identity the lockfile holds;
// - nothing locked is missing: every dependency of every importer that is not a workspace link, and every
//   package of every project document, is among those installed identities.
// With the registry (`verify`):
// - the registry's metadata for each installed identity carries the lockfile's integrity;
// - the bundled npm verifies the registry signatures, and the provenance that metadata announces, for
//   exactly those identities.
//
// Not established: that the files of an installed project package are the bytes of its locked archive.
// No installed file is hashed here except the manager's. A package's identity is read from its own
// installed `package.json`, and that the files on disk are the locked archive's rests on pnpm's frozen
// install with `verifyStoreIntegrity`.
//
// Two shapes of an ordinary lockfile are read, and nothing wider:
// - a version followed by its peer context, `1.2.3(peer@4.5.6)`: the importer must name a snapshot the
//   same document holds, and every snapshot must be of a package under `packages`, which pins it;
// - a package for another platform, which the manager does not install here: it may be absent only when
//   every snapshot of it is `optional: true` and its `os`, `cpu` or `libc` list leaves out linux, x64 or
//   glibc. Such a package is locked but not installed, so nothing here verifies it on this platform.
export { Refusal };
type Identity = { name: string; version: string; integrity: string };
export type Installed = Identity & { path: string };
type ObjectValue = Record<string, unknown>;
type Platforms = { os?: unknown; cpu?: unknown; libc?: unknown };
type Locked = {
  importers: Record<string, Record<string, Record<string, { version?: unknown }> | undefined>>;
  packages?: Record<string, { resolution: { integrity: string } } & Platforms>;
  snapshots?: Record<string, { optional?: unknown } | null>;
};
// `name@1.2.3(peer@4.5.6)(other@7.8.9(nested@1.0.0))` is the package `name@1.2.3` in one peer context.
// Returns that package key, or null unless everything after it is groups in balanced parentheses up to the end.
function peerless(key: string): string | null {
  const open = key.indexOf('(');
  if (open === -1) return key.includes(')') ? null : key;
  let depth = 0;
  for (const character of key.slice(open)) {
    if (depth === 0 && character !== '(') return null;
    if (character === '(') depth += 1;
    if (character === ')') depth -= 1;
  }
  return depth === 0 ? key.slice(0, open) : null;
}
// Whether a locked package is for another platform than the one the pinned manager runs on, which
// `native-pnpm.ts` fixes as linux, x64 and glibc. It is only when one of its lists is a plain list of
// names that leaves this platform's out. A missing list, an empty one, a negation and anything that
// is not a list of names all read as "for this platform", so such a package still has to be installed.
function elsewhere(entry: Platforms): boolean {
  return ([['os', 'linux'], ['cpu', 'x64'], ['libc', 'glibc']] as const).some(([field, own]) => {
    const named = entry[field];
    return Array.isArray(named) && named.length !== 0 && named.every(name => typeof name === 'string' && !name.startsWith('!')) && !named.includes(own);
  });
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
  const documents = parseAllDocuments(text).map(document => document.toJS() as Locked);
  const locked = new Map<string, Identity>();
  for (const document of documents) {
    for (const [key, entry] of Object.entries(document.packages ?? {})) {
      const boundary = key.lastIndexOf('@');
      locked.set(key, { name: key.slice(0, boundary), version: key.slice(boundary + 1), integrity: entry.resolution.integrity });
    }
  }
  const manager = await nativePnpm();
  const managerRoot = dirname(manager);
  const packages = new Map<string, Installed>();
  for (const path of [...await installedPaths(directory), managerRoot]) {
    const value = await manifest(path);
    const identity = locked.get(`${String(value.name)}@${String(value.version)}`);
    if (identity === undefined) refuse('verification.installation', 'installed package identity must match its locked registry identity');
    // `nativePnpm` has just compared the manager's archive with the pinned checksum; the lockfile must name that same archive.
    if (path === managerRoot && identity.integrity !== `sha512-${nativePnpmChecksum}`) {
      refuse('verification.installation', 'locked native manager integrity must be the pinned checksum');
    }
    packages.set(`${identity.name}@${identity.version}`, { ...identity, path: await realpath(path) });
  }
  for (const document of documents) {
    const projects = Object.values(document.importers);
    const pinned = document.packages ?? {};
    const snapshots = document.snapshots ?? {};
    // A snapshot is a locked package in one peer context. One whose package has no entry under `packages`
    // has no integrity, so nothing pins it.
    const contexts = Object.keys(snapshots).map(key => [key, peerless(key)] as const);
    if (contexts.some(([, key]) => key === null || !Object.hasOwn(pinned, key))) {
      refuse('verification.installation', 'every snapshot must name a locked package');
    }
    // The one kind of locked package that may be absent: optional in every snapshot it has, and for another platform.
    const skipped = (key: string, entry: Platforms): boolean => {
      const own = contexts.filter(([, context]) => context === key);
      return own.length !== 0 && own.every(([snapshot]) => snapshots[snapshot]?.optional === true) && elsewhere(entry);
    };
    // The manager's own document locks one executable for each platform. Only this platform's exists here,
    // and it was checked above, so that document's packages are not all expected on disk.
    const required = projects.some(project => project.packageManagerDependencies !== undefined) ? [] :
      Object.entries(pinned).filter(([key, entry]) => !skipped(key, entry)).map(([key]) => key);
    for (const project of projects) {
      for (const group of ['dependencies', 'devDependencies', 'optionalDependencies']) {
        for (const [name, entry] of Object.entries(project[group] ?? {})) {
          // A workspace member is a link to another project, not a package to find installed.
          const version = String(entry.version);
          if (version.startsWith('link:')) continue;
          // An importer names one snapshot exactly, peer context included, and so the package that snapshot is of.
          // Whatever an importer names must be installed, optional or not.
          const key = peerless(`${name}@${version}`);
          if (key === null || !Object.hasOwn(snapshots, `${name}@${version}`)) refuse('verification.installation', 'every locked package must be installed');
          required.push(key);
        }
      }
    }
    if (required.some(key => !packages.has(key))) refuse('verification.installation', 'every locked package must be installed');
  }
  const installed = [...packages.values()].sort((left, right) => left.name < right.name ? -1 : left.name > right.name ? 1 : left.version.localeCompare(right.version));
  const config = command(manager, ['--dir', directory, 'config', 'list', '--json'], directory);
  requirePolicy('effective-registries', { config, names: installed.map(entry => entry.name) });
  return { installed, manager, locked };
}

// `request` stands in for the registry in tests; the command line always uses the real `fetch`.
export async function verify(directory: string, request: typeof fetch = fetch): Promise<unknown> {
  const { installed, locked } = await collectInstalled(directory);
  const present: { name: string; version: string }[] = [];
  for (const entry of installed) {
    const response = await request(`https://registry.npmjs.org/${encodeURIComponent(entry.name)}/${encodeURIComponent(entry.version)}`, {
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
