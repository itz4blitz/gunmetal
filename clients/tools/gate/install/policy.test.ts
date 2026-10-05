import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { inspect } from './policy.ts';

async function fixture(name: string): Promise<string> {
  return readFile(new URL(`fixtures/${name}.txt`, import.meta.url), 'utf8');
}
function refused(rule: string, path: string, message: string): unknown {
  return [{ rule, path, message }];
}

// Verifies: SEC-SUP-033, SEC-SUP-034, SEC-CLI-018
test('secure settings pass without exceptions', async () => {
  assert.deepEqual(inspect('settings', JSON.parse(await fixture('settings'))), []);
});

for (const [key, bad, rule, message] of [
  ['frozenLockfile', false, 'SEC-SUP-033', 'must be true'],
  ['ignoreScripts', false, 'SEC-SUP-033', 'must be true'],
  ['allowBuilds', { canary: true }, 'SEC-SUP-033', 'must be empty'],
  ['dangerouslyAllowAllBuilds', true, 'SEC-SUP-033', 'must be false'],
  ['blockExoticSubdeps', false, 'SEC-SUP-033', 'must be true'],
  ['minimumReleaseAge', 10079, 'SEC-SUP-034', 'must be at least 10080 minutes'],
  ['minimumReleaseAgeStrict', false, 'SEC-SUP-034', 'must be true'],
  ['minimumReleaseAgeIgnoreMissingTime', true, 'SEC-SUP-034', 'must be false'],
  ['minimumReleaseAgeExclude', ['yaml'], 'SEC-SUP-034', 'must be empty'],
  ['trustPolicy', 'off', 'SEC-SUP-034', 'must be no-downgrade'],
  ['trustPolicyExclude', ['yaml'], 'SEC-SUP-034', 'must be empty'],
  ['trustLockfile', true, 'SEC-SUP-034', 'must be false'],
  ['registries', { default: 'https://registry.npmjs.org/', '@other': 'https://evil.test/' }, 'SEC-SUP-033', 'must contain only registry.npmjs.org'],
] as const) {
  // Verifies: SEC-SUP-033, SEC-SUP-034, SEC-CLI-018
  test(`settings reject loosening ${key}`, async () => {
    const settings = JSON.parse(await fixture('settings')) as Record<string, unknown>;
    settings[key] = bad;
    assert.deepEqual(inspect('settings', settings), refused(rule, `workspace.${key}`, message));
  });
  // Verifies: SEC-SUP-033, SEC-SUP-034, SEC-CLI-018
  test(`settings reject removing ${key}`, async () => {
    const settings = JSON.parse(await fixture('settings')) as Record<string, unknown>;
    delete settings[key];
    assert.deepEqual(inspect('settings', settings), refused(rule, `workspace.${key}`, message));
  });
}

// Verifies: SEC-SUP-034
test('an age-based trust exception cannot bypass no-downgrade', async () => {
  assert.deepEqual(inspect('settings', { ...JSON.parse(await fixture('settings')), trustPolicyIgnoreAfter: 10080 }),
    refused('SEC-SUP-034', 'workspace.trustPolicyIgnoreAfter', 'must be absent'));
});

// Verifies: SEC-SUP-033, SEC-CLI-027
test('both normal pnpm documents are accepted and inventoried', async () => {
  assert.deepEqual(inspect('lockfile', { text: await fixture('lockfile') }), []);
});

for (const [source, resolution] of [
  ['git', '{repo: "git+https://evil.test/repo.git", commit: abc}'],
  ['tarball', '{tarball: "https://registry.npmjs.org/yaml/-/yaml-2.9.1.tgz"}'],
  ['second registry', '{registry: "https://evil.test/", integrity: sha512-yaml}'],
  ['escaped git', '{repo: "\\u0067it+https://evil.test/repo.git", commit: abc}'],
] as const) {
  for (const document of [0, 1]) {
    // Verifies: SEC-SUP-033
    test(`lockfile refuses ${source} in document ${document}`, async () => {
      const key = document === 0 ? 'pnpm@12.7.0' : 'yaml@2.9.1';
      const old = document === 0 ? '{integrity: sha512-manager}' : '{integrity: sha512-yaml}';
      const text = (await fixture('lockfile')).replace(old, resolution);
      assert.deepEqual(inspect('lockfile', { text }),
        refused('SEC-SUP-033', `lockfile[${document}].packages.${key}.resolution`, 'registry integrity only; exotic sources are forbidden'));
    });
  }
}

for (const [name, text] of [
  ['duplicate keys', 'lockfileVersion: "9.0"\npackages: {}\npackages: {}\n'],
  ['aliases', 'lockfileVersion: "9.0"\npackages: &packages {}\nsnapshots: *packages\n'],
  ['invalid YAML', 'packages: [\n'],
  ['empty lockfile', ''],
  ['non-mapping document', '---\n[]\n'],
] as const) {
  // Verifies: SEC-SUP-033
  test(`lockfile refuses ${name}`, () => {
    assert.deepEqual(inspect('lockfile', { text }), refused('SEC-SUP-033', 'lockfile', 'invalid or ambiguous YAML'));
  });
}

for (const doc of [0, 1]) {
  // Verifies: SEC-CLI-027
  test(`tracking package fails even in document ${doc}`, async () => {
    const text = (await fixture('lockfile')).replace(doc === 0 ? 'pnpm@12.7.0:' : 'yaml@2.9.1:', '"@sentry/browser@10.0.0":');
    assert.deepEqual(inspect('lockfile', { text }),
      refused('SEC-CLI-027', `lockfile[${doc}].packages.@sentry/browser@10.0.0`, 'tracking package @sentry/browser is forbidden'));
  });
}

for (const field of ['peerDependencies', 'optionalDependencies']) {
  // Verifies: SEC-SUP-035
  test(`${field} refuses a registry package`, () => {
    assert.deepEqual(inspect('manifest', { manifest: { [field]: { yaml: '2.9.1' } }, members: [] }),
      refused('SEC-SUP-035', `manifest.${field}.yaml`, 'only a known workspace member with workspace:* is allowed'));
  });
  // Verifies: SEC-SUP-035
  test(`${field} accepts an actual workspace member`, () => {
    assert.deepEqual(inspect('manifest', { manifest: { [field]: { '@gunmetal/ui': 'workspace:*' } }, members: ['@gunmetal/ui'] }), []);
  });
  // Verifies: SEC-SUP-035
  test(`${field} refuses a counterfeit workspace member`, () => {
    assert.deepEqual(inspect('manifest', { manifest: { [field]: { '@gunmetal/ghost': 'workspace:*' } }, members: ['@gunmetal/ui'] }),
      refused('SEC-SUP-035', `manifest.${field}.@gunmetal/ghost`, 'only a known workspace member with workspace:* is allowed'));
  });
}

// Verifies: SEC-SUP-034
test('the package manager is permitted exactly at seven days', () => {
  assert.deepEqual(inspect('age', { published: '2026-09-27T00:00:00.000Z', now: '2026-10-04T00:00:00.000Z' }), []);
});
for (const published of ['2026-09-27T00:00:00.001Z', '2026-10-05T00:00:00.000Z', 'invalid', null]) {
  // Verifies: SEC-SUP-034
  test(`the package manager refuses immature or missing publication time ${published}`, () => {
    assert.deepEqual(inspect('age', { published, now: '2026-10-04T00:00:00.000Z' }),
      refused('SEC-SUP-034', 'pnpm.publication', 'publication must be known and at least seven days old'));
  });
}

// Verifies: SEC-SUP-029
test('the licence check accepts a complete pnpm report against the project allow-list', () => {
  assert.deepEqual(inspect('licenses', { allowed: ['ISC'], report: { ISC: [{ name: 'yaml', versions: ['2.9.1'], paths: ['/installed/yaml'], license: 'ISC' }] } }), []);
});
for (const license of ['SSPL-1.0', 'UNKNOWN', '', 'MPL-2.0-no-copyleft-exception', 'OFL-1.1']) {
  // Verifies: SEC-SUP-029
  test(`the licence check refuses ${license || 'missing licence'}`, () => {
    assert.deepEqual(inspect('licenses', { allowed: ['MIT', 'MPL-2.0'], report: { [license]: [{ name: 'bad', versions: ['1.0.0'], paths: ['/installed/bad'], license }] } }),
      refused('SEC-SUP-029', 'licenses.bad', `licence ${license || 'UNKNOWN'} is not allowed`));
  });
}
// Test-only additions staged after the prior green; expected values are literals.
// Verifies: SEC-SUP-035
for (const manifest of [null, [], { peerDependencies: [] }, { optionalDependencies: 'yaml' }]) {
  test(`malformed manifest refuses ${JSON.stringify(manifest)}`, () => {
    assert.deepEqual(inspect('manifest', { manifest, members: [] }),
      refused('SEC-SUP-035', 'manifest', 'invalid manifest dependency groups'));
  });
}
// Verifies: SEC-SUP-033
for (const [field, source] of [
  ['dependencies', 'git+https://evil.test/repository.git'],
  ['devDependencies', 'https://registry.npmjs.org/yaml/-/yaml-2.9.1.tgz'],
  ['dependencies', 'file:../foreign'],
  ['devDependencies', 'gh:1.0.0'],
]) {
  test(`manifest refuses exotic source ${source}`, () => {
    assert.deepEqual(inspect('manifest', { manifest: { [field]: { bad: source } }, members: [] }),
      refused('SEC-SUP-033', `manifest.${field}.bad`, 'only exact registry versions or known workspace:* members are allowed'));
  });
}
// Verifies: SEC-SUP-033
for (const document of [0, 1]) {
  for (const source of ['git+https://evil.test/repository.git', 'https://evil.test/pkg.tgz', 'file:../foreign', 'gh:1.0.0']) {
    test(`lockfile importer refuses ${source} in document ${document}`, async () => {
      const original = document === 0 ? 'specifier: 12.7.0' : 'specifier: 2.9.1';
      const text = (await fixture('lockfile')).replace(original, `specifier: "${source}"`);
      assert.deepEqual(inspect('lockfile', { text }), refused('SEC-SUP-033', `lockfile[${document}].importers`, 'exotic dependency sources are forbidden'));
    });
  }
}
// Verifies: SEC-SUP-029
for (const report of [null, [], { MIT: 'bad' }, { MIT: [null] }, { MIT: [{ name: 'bad', license: 'MIT' }] }]) {
  test(`malformed licence report refuses ${JSON.stringify(report)}`, () => {
    assert.deepEqual(inspect('licenses', { allowed: ['MIT'], report }),
      refused('SEC-SUP-029', 'licenses', 'invalid licence report'));
  });
}
// Verifies: SEC-SUP-029
test('an incompatible MPL secondary-licence notice fails even when metadata says MPL-2.0', () => {
  assert.deepEqual(inspect('licenses', {
    allowed: ['MPL-2.0'],
    report: { 'MPL-2.0': [{ name: 'bad', versions: ['1.0.0'], paths: ['/installed/bad'], license: 'MPL-2.0', licenseText: 'This Source Code Form is Incompatible With Secondary Licenses, as defined by the Mozilla Public License, v. 2.0.' }] },
  }), refused('SEC-SUP-029', 'licenses.bad', 'licence MPL-2.0 marked incompatible with secondary licences is not allowed'));
});

// Verifies: SEC-SUP-033, SEC-CLI-027
test('the complete committed manager and project lockfile passes the source policy', async () => {
  const text = await readFile(new URL('../../../pnpm-lock.yaml', import.meta.url), 'utf8');
  assert.deepEqual(inspect('lockfile', { text }), []);
});
