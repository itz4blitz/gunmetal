import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

const client = new URL('../../../', import.meta.url);
async function jsonFile(name: string): Promise<unknown> {
  try {
    return JSON.parse(await readFile(new URL(name, client), 'utf8'));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return null;
    throw error;
  }
}

// Verifies: SEC-SUP-033, SEC-SUP-034, SEC-CLI-018
// Removing or loosening any rule permits an install forbidden by the baseline.
test('the committed workspace requires frozen script-free mature trusted registry installs', async () => {
  assert.deepEqual(await jsonFile('pnpm-workspace.yaml'), {
    packages: ['packages/*', 'apps/*'],
    registries: { default: 'https://registry.npmjs.org/' },
    frozenLockfile: true,
    ignoreScripts: true,
    ignorePnpmfile: true,
    strictDepBuilds: true,
    sideEffectsCache: false,
    verifyStoreIntegrity: true,
    allowBuilds: {},
    dangerouslyAllowAllBuilds: false,
    blockExoticSubdeps: true,
    minimumReleaseAge: 10080,
    minimumReleaseAgeStrict: true,
    minimumReleaseAgeIgnoreMissingTime: false,
    minimumReleaseAgeExclude: [],
    trustPolicy: 'no-downgrade',
    trustPolicyExclude: [],
    trustLockfile: false,
    engineStrict: true,
    autoInstallPeers: false,
    strictPeerDependencies: true,
  });
});

// Verifies: SEC-SUP-034
// An unpinned runtime/package manager would allow unreviewed tool upgrades.
test('the root client manifest pins the reviewed runtime and package manager', async () => {
  const manifest = await jsonFile('package.json') as Record<string, unknown> | null;
  assert.deepEqual(manifest === null ? null : {
    name: manifest.name,
    private: manifest.private,
    type: manifest.type,
    engines: manifest.engines,
    packageManager: manifest.packageManager,
  }, {
    name: '@gunmetal/clients',
    private: true,
    type: 'module',
    engines: { node: '24.20.0', pnpm: '12.7.0' },
    packageManager: 'pnpm@12.7.0',
  });
});

test('the shared TypeScript settings reject unchecked indexed access and absent optional fields', async () => {
  const config = await jsonFile('tsconfig.base.json') as { compilerOptions: Record<string, unknown> } | null;
  assert.deepEqual(config === null ? null : {
    strict: config.compilerOptions.strict,
    noUncheckedIndexedAccess: config.compilerOptions.noUncheckedIndexedAccess,
    exactOptionalPropertyTypes: config.compilerOptions.exactOptionalPropertyTypes,
    noEmit: config.compilerOptions.noEmit,
  }, {
    strict: true,
    noUncheckedIndexedAccess: true,
    exactOptionalPropertyTypes: true,
    noEmit: true,
  });
});
