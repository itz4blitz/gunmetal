import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import * as verifier from './verify.ts';

const client = fileURLToPath(new URL('../../../', import.meta.url));
function verify(directory: string, env = process.env): { status: number | null; signal: string | null; findings: unknown; stderr: string } {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('verify.ts', import.meta.url)), directory], {
    env, encoding: 'utf8', timeout: 180000,
  });
  return { status: result.status, signal: result.signal, findings: result.stdout.trim() === '' ? null : JSON.parse(result.stdout), stderr: result.stderr };
}
function refusal(path: string, message: string): unknown {
  return { status: 1, signal: null, stderr: '', findings: [{ rule: 'SEC-SUP-036', path, message }] };
}
// Runs the verifier on a private copy of the installed workspace, after `change` has edited the copy.
async function copied(change: (directory: string) => Promise<void>): Promise<unknown> {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-verify-'));
  try {
    for (const file of ['package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml']) await cp(join(client, file), join(directory, file));
    await cp(join(client, 'node_modules'), join(directory, 'node_modules'), { recursive: true });
    await change(directory);
    return verify(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}
// Rewrites the copy's lockfile and insists the edit really changed it, so no refusal test can pass vacuously.
async function relock(directory: string, change: (lock: string) => string): Promise<void> {
  const path = join(directory, 'pnpm-lock.yaml');
  const lock = await readFile(path, 'utf8');
  const edited = change(lock);
  assert.notEqual(edited, lock);
  await writeFile(path, edited);
}
const manager = { name: '@pnpm/exe.linux-x64', version: '12.7.0', integrity: 'sha512-gGW7NJFmr33IJ6KZu+1w90KBtFxMVf/+AUG8aKJpy4v6dRnUd5v84PcH2ejUuxp/fm3zM0IY6h9LwcYPu9dxdg==' };
const parser = { name: 'yaml', version: '2.9.1', integrity: 'sha512-3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==' };
// A well-formed integrity that belongs to another package of the same lockfile.
const other = 'sha512-vwPQU+Bt3qXMhzjcmMa25W3yP8OyF9yFxHoojr10QJm9lRUmz/SwtM2oHbHKZKkyid/ngmMARXsChFyuRQig5w==';
const uninstalled = 'every locked package must be installed';

// Verifies: SEC-SUP-036. Real registry verification must cover both the installed manager and project graph.
const verified = {
  status: 0, signal: null, stderr: '', findings: {
    installed: [manager, parser], verified: [manager, parser],
    signatures: { invalid: [], missing: [], provenance: [{ name: '@pnpm/exe.linux-x64', version: '12.7.0' }] },
  },
};
test('real signature and provenance audit covers the exact installed project and manager inventory', () => {
  assert.deepEqual(verify(client), verified);
});

// Verifies: SEC-SUP-011. The manager refusal reaches the verifier's result as itself.
test('without a native manager root the verifier reports that refusal', () => {
  const { GUNMETAL_NATIVE_PNPM_ROOT: _root, ...withoutRoot } = process.env;
  assert.deepEqual(verify(client, withoutRoot), {
    status: 1, signal: null, stderr: '', findings: [{
      rule: 'SEC-SUP-011', path: 'runtime.pnpm', message: 'native pnpm root is required',
    }],
  });
});

// Verifies: SEC-SUP-036. The collector observes physical manifests, rather than trusting a supplied inventory.
test('a counterfeit physically installed identity fails before signature verification', async () => {
  assert.deepEqual(await copied(async directory => {
    const path = join(directory, 'node_modules/.pnpm/yaml@2.9.1/node_modules/yaml/package.json');
    const manifest = JSON.parse(await readFile(path, 'utf8')) as Record<string, unknown>;
    await writeFile(path, JSON.stringify({ ...manifest, name: 'counterfeit' }));
  }), refusal('verification.installation', 'installed package identity must match its locked registry identity'));
});

// Verifies: SEC-SUP-036. Nothing the lockfile promises may be absent from what is verified.
test('an installation that lacks a locked package fails, even though the manager is present', async () => {
  assert.deepEqual(await copied(directory => rm(join(directory, 'node_modules/.pnpm/yaml@2.9.1'), { recursive: true })),
    refusal('verification.installation', uninstalled));
});

test('a package installed under the identity of another locked package leaves its own missing', async () => {
  assert.deepEqual(await copied(async directory => {
    const path = join(directory, 'node_modules/.pnpm/yaml@2.9.1/node_modules/yaml/package.json');
    const manifest = JSON.parse(await readFile(path, 'utf8')) as Record<string, unknown>;
    await writeFile(path, JSON.stringify({ ...manifest, name: manager.name, version: manager.version }));
  }), refusal('verification.installation', uninstalled));
});

test('an importer dependency that is neither locked nor installed fails', async () => {
  assert.deepEqual(await copied(directory => relock(directory, lock => lock.replace('    devDependencies:\n      yaml:',
    '    dependencies:\n      ghost:\n        specifier: 1.0.0\n        version: 1.0.0\n    devDependencies:\n      yaml:'))),
  refusal('verification.installation', uninstalled));
});

test('a locked package that no importer names and nothing installed fails', async () => {
  assert.deepEqual(await copied(directory => relock(directory, lock => lock
    .replace('packages:\n\n  yaml@2.9.1:', `packages:\n\n  ghost@1.0.0:\n    resolution: {integrity: ${other}}\n\n  yaml@2.9.1:`)
    .replace('snapshots:\n\n  yaml@2.9.1: {}', 'snapshots:\n\n  ghost@1.0.0: {}\n\n  yaml@2.9.1: {}'))),
  refusal('verification.installation', uninstalled));
});

test('an importer dependency recorded without a version fails', async () => {
  assert.deepEqual(await copied(directory => relock(directory, lock => lock.replace('        specifier: 2.9.1\n        version: 2.9.1\n', '        specifier: 2.9.1\n'))),
    refusal('verification.installation', uninstalled));
});

// A workspace member is recorded as a link to another project, not as a package to install.
test('workspace members recorded as links need no installed package of their own', async () => {
  assert.deepEqual(await copied(directory => relock(directory, lock => lock.replace('        specifier: 2.9.1\n        version: 2.9.1\n',
    "        specifier: 2.9.1\n        version: 2.9.1\n\n  packages/consumer:\n    dependencies:\n      '@gunmetal/member':\n        specifier: workspace:*\n        version: link:../member\n\n  packages/member: {}\n"))),
  verified);
});

// Verifies: SEC-SUP-011, SEC-SUP-036. The lockfile's manager entry must be the archive the pin names.
test('a lockfile whose native manager integrity is not the pinned checksum fails before any registry request', async () => {
  assert.deepEqual(await copied(directory => relock(directory, lock => lock.replace(manager.integrity, other))),
    refusal('verification.installation', 'locked native manager integrity must be the pinned checksum'));
});

// Verifies: SEC-SUP-036. With nothing locked for the project, only the manager is installed and verified.
test('a workspace that locks no registry package verifies with the manager alone', async () => {
  assert.deepEqual(await copied(async directory => {
    await rm(join(directory, 'node_modules/.pnpm/yaml@2.9.1'), { recursive: true });
    await relock(directory, lock => `${lock.slice(0, lock.indexOf('\n---\n', 4))}\n---\nlockfileVersion: '9.0'\n\nsettings:\n  autoInstallPeers: false\n  excludeLinksFromLockfile: false\n\nimporters:\n\n  .: {}\n`);
  }), {
    status: 0, signal: null, stderr: '', findings: {
      installed: [manager], verified: [manager],
      signatures: { invalid: [], missing: [], provenance: [{ name: '@pnpm/exe.linux-x64', version: '12.7.0' }] },
    },
  });
});

// The exported verifier with a stand-in for the registry request, so the registry's answers can be made wrong.
type Request = (input: string | URL, init?: RequestInit) => Promise<Response>;
async function requested(request: Request): Promise<unknown> {
  const run = (verifier as { verify?: (directory: string, request: Request) => Promise<unknown> }).verify;
  try { return await run?.(client, request); }
  catch (error) { return (error as { findings?: unknown }).findings ?? String(error); }
}
// Fetches the real metadata and hands it on after `change`, so only the named field differs from the registry's.
function altered(change: (metadata: { dist: Record<string, unknown> }) => unknown): Request {
  return async (input, init) => Response.json(change(await (await fetch(input, init)).json() as { dist: Record<string, unknown> }));
}
function finding(path: string, message: string): unknown {
  return [{ rule: 'SEC-SUP-036', path, message }];
}

// Verifies: SEC-SUP-036. Each answer of the registry is checked, not assumed.
test('a registry that does not answer for an installed package fails verification', async () => {
  assert.deepEqual(await requested(async () => new Response('', { status: 503 })), finding('verification.registry', 'registry metadata is unavailable'));
});

test('registry metadata with another integrity than the lockfile fails verification', async () => {
  assert.deepEqual(await requested(altered(metadata => ({ ...metadata, dist: { ...metadata.dist, integrity: other } }))),
    finding('verification.inventory', 'verifier inventory must exactly match installed package identities and integrity'));
});

test('registry metadata for another version than the installed one fails verification', async () => {
  assert.deepEqual(await requested(altered(metadata => ({ ...metadata, version: '0.0.0' }))),
    finding('verification.inventory', 'verifier inventory must exactly match installed package identities and integrity'));
});

test('a signature audit that proves provenance the registry metadata did not announce fails verification', async () => {
  assert.deepEqual(await requested(altered(({ dist: { attestations: _attestations, ...dist }, ...metadata }) => ({ ...metadata, dist }))),
    finding('verification.signatures', 'all signatures and present provenance must verify for the exact installed inventory'));
});

test('a signature audit that lacks provenance the registry metadata announced fails verification', async () => {
  assert.deepEqual(await requested(altered(metadata => ({ ...metadata, dist: { ...metadata.dist, attestations: { url: 'https://registry.npmjs.org/-/npm/v1/attestations/announced' } } }))),
    finding('verification.signatures', 'all signatures and present provenance must verify for the exact installed inventory'));
});
