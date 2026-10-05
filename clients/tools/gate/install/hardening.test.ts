import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { nativePnpm } from './native-pnpm.ts';
import { inspect } from './policy.ts';

const deny = (rule: string, path: string, message: string) => [{ rule, path, message }];
async function fixture(name: string): Promise<string> {
  return readFile(new URL(`fixtures/${name}.txt`, import.meta.url), 'utf8');
}

// Verifies: SEC-SUP-033, SEC-CLI-018. Policy logic is proved at the pure boundary.
for (const [key, unsafe] of [
  ['ignorePnpmfile', false],
  ['strictDepBuilds', false],
  ['sideEffectsCache', true],
  ['verifyStoreIntegrity', false],
  ['engineStrict', false],
  ['autoInstallPeers', true],
  ['strictPeerDependencies', false],
] as const) {
  test(`settings refuse unsafe ${key}`, async () => {
    const value = { ...JSON.parse(await fixture('settings')), [key]: unsafe };
    assert.deepEqual(inspect('settings', value), deny('SEC-SUP-033', `workspace.${key}`, key === 'sideEffectsCache' || key === 'autoInstallPeers' ? 'must be false' : 'must be true'));
  });
  test(`settings refuse removing ${key}`, async () => {
    const value = JSON.parse(await fixture('settings')) as Record<string, unknown>;
    delete value[key];
    assert.deepEqual(inspect('settings', value), deny('SEC-SUP-033', `workspace.${key}`, key === 'sideEffectsCache' || key === 'autoInstallPeers' ? 'must be false' : 'must be true'));
  });
}
for (const [key, value] of [
  ['namedRegistries', { custom: 'https://evil.test/' }],
  ['packageConfigs', { member: { ignoreScripts: false } }],
  ['configDependencies', { injected: '1.0.0' }],
  ['pnpmfile', './hook.mjs'],
  ['globalPnpmfile', '/tmp/hook.mjs'],
  ['patchedDependencies', { 'yaml@2.9.1': './patch.diff' }],
] as const) {
  test(`settings refuse resolution or execution bypass ${key}`, async () => {
    assert.deepEqual(inspect('settings', { ...JSON.parse(await fixture('settings')), [key]: value }),
      deny('SEC-SUP-033', `workspace.${key}`, 'must be absent'));
  });
}

// Verifies: SEC-SUP-033. Sources are decoded before policy, in each lockfile document.
for (const document of [0, 1]) {
  for (const source of ['git+https://evil.test/pkg.git', 'file:../foreign', 'gh:1.0.0', '\\u0068ttps://evil.test/pkg.tgz']) {
    test(`snapshot sources refuse ${source} in document ${document}`, async () => {
      const key = document === 0 ? 'pnpm@12.7.0' : 'yaml@2.9.1';
      const text = (await fixture('lockfile')).replace(`${key}: {}`, `${key}:\n    dependencies: {bad: "${source}"}`);
      assert.deepEqual(inspect('lockfile', { text }), deny('SEC-SUP-033', `lockfile[${document}].snapshots`, 'exotic dependency sources are forbidden'));
    });
  }
  for (const key of ['bad@https://evil.test/pkg.tgz', 'bad@gh:1.0.0', 'bad@file:../foreign']) {
    test(`package identities refuse ${key} in document ${document}`, async () => {
      const original = document === 0 ? 'pnpm@12.7.0:' : 'yaml@2.9.1:';
      const text = (await fixture('lockfile')).replace(original, `"${key}":`);
      assert.deepEqual(inspect('lockfile', { text }), deny('SEC-SUP-033', `lockfile[${document}].packages.${key}`, 'only canonical registry package identities are allowed'));
    });
  }
}

// Verifies: SEC-CLI-027. Exact names and scoped SDK families, not substrings.
for (const name of [
  '@amplitude/analytics-browser', '@bugsnag/js', '@datadog/browser-rum',
  '@firebase/analytics', '@firebase/analytics-compat', '@honeybadger-io/js',
  '@opentelemetry/api', '@segment/analytics-next', '@sentry/core',
  'analytics', 'bugsnag-js', 'mixpanel-browser', 'newrelic', 'posthog-js', 'rollbar',
]) {
  test(`tracking deny-list refuses ${name}`, async () => {
    const text = (await fixture('lockfile')).replace('yaml@2.9.1:', `"${name}@2.9.1":`);
    assert.deepEqual(inspect('lockfile', { text }), deny('SEC-CLI-027', `lockfile[1].packages.${name}@2.9.1`, `tracking package ${name} is forbidden`));
  });
}
for (const name of ['analytics-helper', '@sentry-like/helper', '@firebase/util']) {
  test(`tracking deny-list accepts unrelated package ${name}`, async () => {
    const text = (await fixture('lockfile')).replace('yaml@2.9.1:', `"${name}@2.9.1":`);
    assert.deepEqual(inspect('lockfile', { text }), []);
  });
}

// Verifies: SEC-SUP-033. Built-in pnpm routes are harmless only when unused.
// The route keys exactly as pnpm 12.7.0 prints them in `config list --json` for the committed workspace
// (TeamCity Personal Build 52's report). `@` is the scope of the default registry.
const routes = {
  '@jsr:registry': 'https://npm.jsr.io/',
  registries: {
    'https://npm.jsr.io/': { scopes: ['@jsr'] },
    'https://npm.pkg.github.com/': { prefix: 'gh' },
    'https://registry.npmjs.org/': { scopes: ['@'], prefix: 'npmjs' },
  },
  registry: 'https://registry.npmjs.org/',
};
test('effective registry check accepts unused built-in alternative routes', () => {
  assert.deepEqual(inspect('effective-registries', { config: routes, names: ['yaml', '@pnpm/exe.linux-x64'] }), []);
});
// The routes above are not only written by hand: they are what the pinned manager reports today, and its
// whole report passes or fails the check as they do.
test('the pinned manager reports those routes, which pass for the used packages and fail for a jsr one', async () => {
  const client = fileURLToPath(new URL('../../../', import.meta.url));
  const listed = spawnSync(await nativePnpm(), ['--dir', client, 'config', 'list', '--json'], { encoding: 'utf8', timeout: 10000 });
  const config = (listed.status === 0 ? JSON.parse(listed.stdout) : {}) as Record<string, unknown>;
  assert.deepEqual({ '@jsr:registry': config['@jsr:registry'], registries: config.registries, registry: config.registry }, routes);
  assert.deepEqual(inspect('effective-registries', { config, names: ['yaml', '@pnpm/exe.linux-x64'] }), []);
  assert.deepEqual(inspect('effective-registries', { config, names: ['@jsr/std'] }), deny('SEC-SUP-033', 'effective.@jsr/std', 'only registry.npmjs.org is allowed'));
});
for (const [config, names, path] of [
  [{ ...routes, registry: 'https://evil.test/' }, ['yaml'], 'effective.registry'],
  [routes, ['@jsr/std'], 'effective.@jsr/std'],
  [{ ...routes, '@jsr:registry': 'https://registry.npmjs.org/' }, ['@jsr/std'], 'effective.@jsr/std'],
  [{ ...routes, '@pnpm:registry': 'https://evil.test/' }, ['@pnpm/exe.linux-x64'], 'effective.@pnpm/exe.linux-x64'],
  [{ ...routes, registries: { ...routes.registries, 'https://evil.test/': { scopes: ['@pnpm'] } } }, ['@pnpm/exe.linux-x64'], 'effective.@pnpm/exe.linux-x64'],
  // Another registry claiming the default scope takes every package, scoped or not.
  [{ ...routes, registries: { ...routes.registries, 'https://evil.test/': { scopes: ['@'] } } }, ['yaml'], 'effective.registry'],
  [{ ...routes, registries: { ...routes.registries, 'https://evil.test/': { scopes: ['@other', '@'] } } }, ['@pnpm/exe.linux-x64'], 'effective.registry'],
] as const) {
  test(`effective registry check refuses redirected used package ${path}`, () => {
    assert.deepEqual(inspect('effective-registries', { config, names }), deny('SEC-SUP-033', path, 'only registry.npmjs.org is allowed'));
  });
}

// Verifies: SEC-SUP-036. Verification must cover exact installed identities and tarball integrity.
const installed = [{ name: 'yaml', version: '2.9.1', integrity: 'sha512-installed' }];
test('signature inventory accepts the exact installed package set regardless of order', () => {
  assert.deepEqual(inspect('verification-inventory', { installed, verified: [...installed] }), []);
});
for (const verified of [[], [{ name: 'fake', version: '2.9.1', integrity: 'sha512-installed' }],
  [{ name: 'yaml', version: '2.9.0', integrity: 'sha512-installed' }],
  [{ name: 'yaml', version: '2.9.1', integrity: 'sha512-counterfeit' }],
  [...installed, { name: 'extra', version: '1.0.0', integrity: 'sha512-extra' }],
  [...installed, ...installed], null]) {
  test(`signature inventory refuses mismatch ${JSON.stringify(verified)}`, () => {
    assert.deepEqual(inspect('verification-inventory', { installed, verified }), deny('SEC-SUP-036', 'verification.inventory', 'verifier inventory must exactly match installed package identities and integrity'));
  });
}

// Verifies: SEC-SUP-029. Empty, inconsistent or truncated licence inventories cannot pass.
for (const entry of [
  { name: 'yaml', versions: [], paths: ['/installed/yaml'], license: 'ISC' },
  { name: 'yaml', versions: ['2.9.1'], paths: [], license: 'ISC' },
  { name: 'yaml', versions: [null], paths: ['/installed/yaml'], license: 'ISC' },
  { name: 'yaml', versions: ['2.9.1'], paths: [null], license: 'ISC' },
  { name: 'yaml', versions: ['2.9.1'], paths: ['/installed/yaml'], license: 'MIT' },
]) {
  test(`licence report refuses incomplete or inconsistent entry ${JSON.stringify(entry)}`, () => {
    assert.deepEqual(inspect('licenses', { allowed: ['MIT', 'ISC'], report: { ISC: [entry] } }), deny('SEC-SUP-029', 'licenses', 'invalid licence report'));
  });
}

// Verifies: SEC-SUP-036. Present provenance must be verified for precisely its installed identity.
const attested = [{ name: 'yaml', version: '2.9.1' }];
const verifiedAttestation = { name: 'yaml', version: '2.9.1', registry: 'https://registry.npmjs.org/', attestations: { provenance: {} } };
test('signature report accepts verified present provenance and clean signature results', () => {
  assert.deepEqual(inspect('signature-report', { present: attested, report: { invalid: [], missing: [], verified: [verifiedAttestation] } }), []);
});
test('signature report accepts no attestations only when none were present', () => {
  assert.deepEqual(inspect('signature-report', { present: [], report: { invalid: [], missing: [], verified: [] } }), []);
});
for (const report of [
  { invalid: [{ name: 'yaml', code: 'EINTEGRITYSIGNATURE' }], missing: [], verified: [verifiedAttestation] },
  { invalid: [{ name: 'yaml', code: 'EATTESTATIONVERIFY' }], missing: [], verified: [verifiedAttestation] },
  { invalid: [], missing: [{ name: 'yaml' }], verified: [verifiedAttestation] },
  { invalid: [], missing: [], verified: [] },
  { invalid: [], missing: [], verified: [{ ...verifiedAttestation, name: 'counterfeit' }] },
  { invalid: [], missing: [], verified: [{ ...verifiedAttestation, version: '2.9.0' }] },
  { invalid: [], missing: [], verified: [{ ...verifiedAttestation, registry: 'https://evil.test/' }] },
  { invalid: [], missing: [], verified: [verifiedAttestation, verifiedAttestation] },
  { invalid: [], verified: [verifiedAttestation] },
  null,
]) {
  test(`signature report refuses incomplete or invalid cryptographic evidence ${JSON.stringify(report)}`, () => {
    assert.deepEqual(inspect('signature-report', { present: attested, report }),
      deny('SEC-SUP-036', 'verification.signatures', 'all signatures and present provenance must verify for the exact installed inventory'));
  });
}

// Verifies: SEC-SUP-029. A successful report must cover every installed package version.
const completeReport = { ISC: [{ name: 'yaml', versions: ['2.9.1'], paths: ['/installed/yaml'], license: 'ISC' }] };
test('licence inventory accepts exact installed version coverage', () => {
  assert.deepEqual(inspect('license-inventory', { installed, report: completeReport }), []);
});
for (const report of [{}, { ISC: [] }, { ISC: [{ name: 'yaml', versions: ['2.9.0'], paths: ['/installed/yaml'], license: 'ISC' }] },
  { ISC: [...completeReport.ISC, { name: 'extra', versions: ['1.0.0'], paths: ['/installed/extra'], license: 'ISC' }] }]) {
  test(`licence inventory refuses missing or extraneous versions ${JSON.stringify(report)}`, () => {
    assert.deepEqual(inspect('license-inventory', { installed, report }), deny('SEC-SUP-029', 'licenses.inventory', 'licence report must exactly cover every installed package version'));
  });
}
