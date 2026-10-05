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

// The integrity of each fixture document's package, as the real lockfile records them.
const integrity = [
  'sha512-nFZHfjYAaNbp3KapLvtORrPcWlL86/PYTXi5c2wN7ViaVhYhpS9oIZp6OfrMY83AecMibvnJ1fArefdOaagHtg==',
  'sha512-3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==',
] as const;
for (const [source, resolution] of [
  ['git', '{repo: "git+https://evil.test/repo.git", commit: abc}'],
  ['tarball', '{tarball: "https://registry.npmjs.org/yaml/-/yaml-2.9.1.tgz"}'],
  ['second registry', `{registry: "https://evil.test/", integrity: ${integrity[1]}}`],
  ['escaped git', '{repo: "\\u0067it+https://evil.test/repo.git", commit: abc}'],
  ['a resolution without integrity', '{}'],
  ['an empty integrity', '{integrity: ""}'],
  ['a sha1 integrity', '{integrity: sha1-2jmj7l5rSw0yVb/vlWAYkK/YBwk=}'],
  ['a sha256 integrity', '{integrity: sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=}'],
  ['a shortened sha512 integrity', `{integrity: ${integrity[1].slice(0, -3)}==}`],
  ['a second hash after the sha512 integrity', `{integrity: "${integrity[1]} sha1-2jmj7l5rSw0yVb/vlWAYkK/YBwk="}`],
  ['a revision beside the integrity', `{integrity: ${integrity[1]}, revision: abc}`],
] as const) {
  for (const document of [0, 1] as const) {
    // Verifies: SEC-SUP-033
    test(`lockfile refuses ${source} in document ${document}`, async () => {
      const key = document === 0 ? 'pnpm@12.7.0' : 'yaml@2.9.1';
      const old = `{integrity: ${integrity[document]}}`;
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
  ['dependencies', 'npm:yaml@2.9.1'],
  ['devDependencies', 'catalog:yaml'],
]) {
  test(`manifest refuses exotic source ${source}`, () => {
    assert.deepEqual(inspect('manifest', { manifest: { [field]: { bad: source } }, members: [] }),
      refused('SEC-SUP-033', `manifest.${field}.bad`, 'only exact registry versions or known workspace:* members are allowed'));
  });
}
// Verifies: SEC-SUP-033. The only non-registry protocol is a known workspace member.
for (const source of ['npm:yaml@2.9.1', 'catalog:yaml']) {
  test(`lockfile importer refuses ${source} in every document`, async () => {
    for (const document of [0, 1]) {
      const original = document === 0 ? 'specifier: 12.7.0' : 'specifier: 2.9.1';
      const text = (await fixture('lockfile')).replace(original, `specifier: "${source}"`);
      assert.deepEqual(inspect('lockfile', { text }),
        refused('SEC-SUP-033', `lockfile[${document}].importers`, 'unsupported dependency protocol is forbidden'));
    }
  });
}
for (const source of ['workspace:^', 'workspace:~', 'workspace:../member']) {
  test(`lockfile importer refuses the workspace range ${source}`, async () => {
    const text = (await fixture('lockfile')).replace('specifier: 2.9.1', `specifier: "${source}"`);
    assert.deepEqual(inspect('lockfile', { text }),
      refused('SEC-SUP-033', 'lockfile[1].importers', 'unsupported dependency protocol is forbidden'));
  });
}
for (const document of [0, 1]) {
  for (const source of ['npm:other@1.0.0', 'catalog:default', 'workspace:*']) {
    test(`lockfile snapshot refuses ${source} in document ${document}`, async () => {
      const key = document === 0 ? 'pnpm@12.7.0' : 'yaml@2.9.1';
      const text = (await fixture('lockfile')).replace(`${key}: {}`, `${key}: {dependencies: {other: "${source}"}}`);
      assert.deepEqual(inspect('lockfile', { text }),
        refused('SEC-SUP-033', `lockfile[${document}].snapshots`, 'unsupported dependency protocol is forbidden'));
    });
  }
}
for (const [name, importers] of [
  ['a project that is a source', { '.': 'npm:evil@1.0.0' }],
  ['a dependency group that is a source', { '.': { dependencies: 'npm:evil@1.0.0' } }],
  ['a dependency entry that is a source', { '.': { dependencies: { evil: 'npm:evil@1.0.0' } } }],
] as const) {
  test(`lockfile refuses ${name}`, () => {
    const text = `lockfileVersion: "9.0"\nimporters: ${JSON.stringify(importers)}\npackages: {}\nsnapshots: {}\n`;
    assert.deepEqual(inspect('lockfile', { text }),
      refused('SEC-SUP-033', 'lockfile[0].importers', 'unsupported dependency protocol is forbidden'));
  });
}

// Verifies: SEC-SUP-033. A document is read only in the shape the pinned manager writes: `importers`
// always, `packages` and `snapshots` together or not at all, and no other top-level key. Merge keys and
// directives, which another YAML reader could resolve differently, are refused wherever they appear.
const empty = 'lockfileVersion: "9.0"\nimporters: {".": {}}\npackages: {}\nsnapshots: {}\n';
test('lockfile accepts a document that locks nothing', () => {
  assert.deepEqual(inspect('lockfile', { text: empty }), []);
});
test('lockfile accepts a document with importers only, as pnpm writes for a workspace without registry packages', () => {
  const text = "lockfileVersion: '9.0'\n\nsettings:\n  autoInstallPeers: false\n  excludeLinksFromLockfile: false\n\nimporters:\n\n  .: {}\n";
  assert.deepEqual(inspect('lockfile', { text }), []);
});
for (const [name, text] of [
  ['a document without importers', 'lockfileVersion: "9.0"\npackages: {}\nsnapshots: {}\n'],
  ['importers that are a source', 'lockfileVersion: "9.0"\nimporters: "npm:evil@1.0.0"\npackages: {}\nsnapshots: {}\n'],
  ['importers that are a list', 'lockfileVersion: "9.0"\nimporters: []\npackages: {}\nsnapshots: {}\n'],
  ['packages without snapshots', 'lockfileVersion: "9.0"\nimporters: {".": {}}\npackages: {}\n'],
  ['snapshots without packages', 'lockfileVersion: "9.0"\nimporters: {".": {}}\nsnapshots: {}\n'],
  ['packages and snapshots with no value', 'lockfileVersion: "9.0"\nimporters: {".": {}}\npackages:\nsnapshots:\n'],
  ['packages that are a list', 'lockfileVersion: "9.0"\nimporters: {".": {}}\npackages: []\nsnapshots: {}\n'],
  ['snapshots that are a list', 'lockfileVersion: "9.0"\nimporters: {".": {}}\npackages: {}\nsnapshots: []\n'],
  ['an unknown top-level key', `${empty}overrides: {yaml: 2.9.0}\n`],
  ['a merge key that carries the importers', "lockfileVersion: '9.0'\npackages: {}\n<<: {importers: {.: {devDependencies: {yaml: {specifier: 2.9.1, version: 'link:../evil'}}}}}\n"],
  ['a merge key at the top level', `${empty}<<: {}\n`],
  ['a quoted merge key at the top level', `${empty}"<<": {}\n`],
  ['a merge key inside an importer', 'lockfileVersion: "9.0"\nimporters: {".": {<<: {devDependencies: {yaml: {specifier: 2.9.1, version: "link:../evil"}}}}}\npackages: {}\nsnapshots: {}\n'],
  ['a merge key inside a snapshot', 'lockfileVersion: "9.0"\nimporters: {".": {}}\npackages: {}\nsnapshots: {"yaml@2.9.1": {<<: {dependencies: {}}}}\n'],
  ['a merge key inside a list', 'lockfileVersion: "9.0"\nimporters: {".": {}}\npackages: {}\nsnapshots: {"yaml@2.9.1": {cpu: [{<<: {}}]}}\n'],
  ['a YAML 1.1 directive', `%YAML 1.1\n---\n${empty}`],
  ['a YAML 1.2 directive', `%YAML 1.2\n---\n${empty}`],
  ['a tag directive', `%TAG ! tag:example.test,2026:\n---\n${empty}`],
  ['a directive before a later document', `${empty}...\n%YAML 1.1\n---\n${empty}`],
] as const) {
  test(`lockfile refuses ${name}`, () => {
    assert.deepEqual(inspect('lockfile', { text }), refused('SEC-SUP-033', 'lockfile', 'invalid or ambiguous YAML'));
  });
}

// Verifies: SEC-SUP-033. A workspace member is accepted only in the shape the pinned manager records:
// `workspace:*` linked to another project of the same lockfile document.
test('lockfile accepts workspace members as the pinned manager records them', async () => {
  assert.deepEqual(inspect('lockfile', { text: await fixture('lockfile-workspace') }), []);
});
for (const [name, original, replacement, message] of [
  ['a workspace link to a project the lockfile does not record', 'link:../member', 'link:../outside', 'unsupported dependency protocol is forbidden'],
  ['a workspace link that leaves the workspace', 'link:../member', 'link:../../../member', 'unsupported dependency protocol is forbidden'],
  ['a workspace link from a project to itself', 'link:../member', 'link:.', 'unsupported dependency protocol is forbidden'],
  ['a workspace specifier recorded with a registry version', 'version: link:../member', 'version: 1.0.0', 'unsupported dependency protocol is forbidden'],
  ['a workspace specifier recorded without a version', '        version: link:../member\n', '', 'unsupported dependency protocol is forbidden'],
  ['a workspace range recorded with a link', 'specifier: workspace:*', 'specifier: workspace:^', 'unsupported dependency protocol is forbidden'],
  ['a link recorded for a registry specifier', 'specifier: workspace:*', 'specifier: 1.0.0', 'exotic dependency sources are forbidden'],
] as const) {
  test(`lockfile refuses ${name}`, async () => {
    const text = (await fixture('lockfile-workspace')).replace(original, replacement);
    assert.deepEqual(inspect('lockfile', { text }), refused('SEC-SUP-033', 'lockfile[1].importers', message));
  });
}
test('lockfile refuses a workspace link inside a snapshot', async () => {
  const text = (await fixture('lockfile-workspace')).replace('yaml@2.9.1: {}', 'yaml@2.9.1: {dependencies: {member: "link:../member"}}');
  assert.deepEqual(inspect('lockfile', { text }), refused('SEC-SUP-033', 'lockfile[1].snapshots', 'exotic dependency sources are forbidden'));
});

// Verifies: SEC-SUP-035. The reason list is read strictly: one quoted reason per named package.
const reviewed = [{ path: 'clients/package.json', manifest: { devDependencies: { yaml: '2.9.1' } } }];
for (const [name, list] of [
  ['a duplicate row', 'yaml = "first"\nyaml = "second"\n'],
  ['a row without a separator', 'yaml "strict parser"\n'],
  ['an unquoted reason', 'yaml = strict parser\n'],
  ['a reason without its closing quote', 'yaml = "strict parser\n'],
  ['a reason without its opening quote', 'yaml = strict parser"\n'],
  ['a package name with a space', 'ya ml = "strict parser"\n'],
  ['a table header', '[dependencies]\nyaml = "strict parser"\n'],
  ['an empty reason followed by a quoted comment', 'yaml = "" # "why"\n'],
  ['a reason followed by a comment', 'yaml = "strict parser" # reviewed\n'],
  ['a reason with a quotation mark inside', 'yaml = "a "strict" parser"\n'],
  ['a reason with an escape', 'yaml = "strict\\u0020parser"\n'],
  ['a name with only its opening quote', '"yaml = "strict parser"\n'],
] as const) {
  test(`dependency reason list refuses ${name}`, () => {
    assert.deepEqual(inspect('direct-dependencies', { manifests: reviewed, list }),
      refused('SEC-SUP-035', 'supply-chain/js-direct-deps.toml', 'dependency reason list is invalid'));
  });
}
for (const input of [
  null,
  { manifests: 'clients/package.json', list: '' },
  { manifests: [], list: 5 },
  { manifests: [null], list: '' },
  { manifests: [{ path: 5, manifest: {} }], list: '' },
  { manifests: [{ path: 'clients/package.json', manifest: null }], list: '' },
  { manifests: [{ path: 'clients/package.json', manifest: { dependencies: [] } }], list: '' },
]) {
  test(`dependency reason check refuses the malformed input ${JSON.stringify(input)}`, () => {
    assert.deepEqual(inspect('direct-dependencies', input),
      refused('SEC-SUP-035', 'supply-chain/js-direct-deps.toml', 'dependency reason list is invalid'));
  });
}
test('dependency reason list accepts comments, blank lines, CRLF endings and a quoted scoped name', () => {
  assert.deepEqual(inspect('direct-dependencies', {
    manifests: [{ path: 'clients/package.json', manifest: { dependencies: { '@gunmetal/kit': '1.2.3' }, devDependencies: { yaml: '2.9.1' } } }],
    list: '# Reasons.\r\n\r\n  # An indented comment.\r\n"@gunmetal/kit" = "shared kit"\r\nyaml = "strict parser"\r\n',
  }), []);
});
test('one reason covers a package used by several manifests, and workspace members need none', () => {
  assert.deepEqual(inspect('direct-dependencies', {
    manifests: [
      { path: 'clients/package.json', manifest: { devDependencies: { yaml: '2.9.1' } } },
      { path: 'clients/packages/ui/package.json', manifest: { dependencies: { yaml: '2.9.1', '@gunmetal/kit': 'workspace:*' } } },
    ],
    list: 'yaml = "strict parser"\n',
  }), []);
});
test('every unexplained direct dependency is reported once, in name order', () => {
  assert.deepEqual(inspect('direct-dependencies', {
    manifests: [
      { path: 'clients/package.json', manifest: { dependencies: { zod: '4.0.0', ajv: '8.0.0' } } },
      { path: 'clients/packages/ui/package.json', manifest: { devDependencies: { ajv: '8.0.0' } } },
    ],
    list: '',
  }), [
    { rule: 'SEC-SUP-035', path: 'supply-chain/js-direct-deps.toml.ajv', message: 'direct registry dependency is missing a written reason' },
    { rule: 'SEC-SUP-035', path: 'supply-chain/js-direct-deps.toml.zod', message: 'direct registry dependency is missing a written reason' },
  ]);
});
test('a reason of only spaces is not a written reason', () => {
  assert.deepEqual(inspect('direct-dependencies', { manifests: reviewed, list: 'yaml = "   "\n' }),
    refused('SEC-SUP-035', 'supply-chain/js-direct-deps.toml.yaml', 'direct registry dependency must have a written reason'));
});
// Verifies: SEC-SUP-035. The reason check fails closed on its own: every entry that is not a
// workspace member is a direct dependency, however its version is written and whichever group holds it.
for (const version of ['^2.9.1', 5]) {
  test(`a direct dependency that is not pinned exactly (${JSON.stringify(version)}) is covered by its reason and needs one`, () => {
    const manifests = [{ path: 'clients/package.json', manifest: { devDependencies: { yaml: version } } }];
    assert.deepEqual(inspect('direct-dependencies', { manifests, list: 'yaml = "strict parser"\n' }), []);
    assert.deepEqual(inspect('direct-dependencies', { manifests, list: '' }),
      refused('SEC-SUP-035', 'supply-chain/js-direct-deps.toml.yaml', 'direct registry dependency is missing a written reason'));
  });
}
for (const field of ['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies']) {
  test(`an unexplained entry under ${field} fails the reason check on its own`, () => {
    assert.deepEqual(inspect('direct-dependencies', {
      manifests: [{ path: 'clients/package.json', manifest: { [field]: { evil: '^1.0.0', '@gunmetal/kit': 'workspace:*' } } }], list: '',
    }), refused('SEC-SUP-035', 'supply-chain/js-direct-deps.toml.evil', 'direct registry dependency is missing a written reason'));
  });
  test(`${field} that is not a mapping makes the reason check refuse its input`, () => {
    assert.deepEqual(inspect('direct-dependencies', {
      manifests: [{ path: 'clients/package.json', manifest: { [field]: ['evil'] } }], list: '',
    }), refused('SEC-SUP-035', 'supply-chain/js-direct-deps.toml', 'dependency reason list is invalid'));
  });
}

// Verifies: SEC-SUP-035. Every direct registry package has one non-empty reason.
test('direct registry dependencies require their exact reviewed mapping', () => {
  assert.deepEqual(inspect('direct-dependencies', {
    manifests: [{ path: 'clients/package.json', manifest: { devDependencies: { yaml: '2.9.1' } } }],
    list: 'yaml = "strict parser"\n',
  }), []);
});
test('the committed direct dependency manifest and review list map exactly', async () => {
  const [manifest, list] = await Promise.all([
    readFile(new URL('../../../package.json', import.meta.url), 'utf8'),
    readFile(new URL('../../../../supply-chain/js-direct-deps.toml', import.meta.url), 'utf8'),
  ]);
  assert.deepEqual(inspect('direct-dependencies', {
    manifests: [{ path: 'clients/package.json', manifest: JSON.parse(manifest) }], list,
  }), []);
});
test('direct registry dependencies fail when their reviewed mapping is missing', () => {
  assert.deepEqual(inspect('direct-dependencies', {
    manifests: [{ path: 'clients/package.json', manifest: { devDependencies: { yaml: '2.9.1' } } }], list: '',
  }), refused('SEC-SUP-035', 'supply-chain/js-direct-deps.toml.yaml', 'direct registry dependency is missing a written reason'));
});
test('direct registry dependencies fail when the list maps a different package', () => {
  assert.deepEqual(inspect('direct-dependencies', {
    manifests: [{ path: 'clients/package.json', manifest: { devDependencies: { yaml: '2.9.1' } } }], list: 'other = "wrong mapping"\n',
  }), [
    { rule: 'SEC-SUP-035', path: 'supply-chain/js-direct-deps.toml.yaml', message: 'direct registry dependency is missing a written reason' },
    { rule: 'SEC-SUP-035', path: 'supply-chain/js-direct-deps.toml.other', message: 'reviewed dependency is not used by any manifest' },
  ]);
});
test('direct registry dependencies fail when the listed reason is empty', () => {
  assert.deepEqual(inspect('direct-dependencies', {
    manifests: [{ path: 'clients/package.json', manifest: { devDependencies: { yaml: '2.9.1' } } }], list: 'yaml = ""\n',
  }), refused('SEC-SUP-035', 'supply-chain/js-direct-deps.toml.yaml', 'direct registry dependency must have a written reason'));
});
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
