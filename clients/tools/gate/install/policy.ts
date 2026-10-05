import { parseAllDocuments, visit } from 'yaml';
import { createHash } from 'node:crypto';
import { posix } from 'node:path';

type ObjectValue = Record<string, unknown>;
export type Finding = { rule: string; path: string; message: string };

function object(value: unknown): value is ObjectValue {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function finding(rule: string, path: string, message: string): Finding[] {
  return [{ rule, path, message }];
}

export function settings(input: unknown): Finding[] {
  const value = object(input) ? input : {};
  const rules: [string, string, string, (candidate: unknown) => boolean][] = [
    ['frozenLockfile', 'SEC-SUP-033', 'must be true', candidate => candidate === true],
    ['ignoreScripts', 'SEC-SUP-033', 'must be true', candidate => candidate === true],
    ['ignorePnpmfile', 'SEC-SUP-033', 'must be true', candidate => candidate === true],
    ['strictDepBuilds', 'SEC-SUP-033', 'must be true', candidate => candidate === true],
    ['sideEffectsCache', 'SEC-SUP-033', 'must be false', candidate => candidate === false],
    ['verifyStoreIntegrity', 'SEC-SUP-033', 'must be true', candidate => candidate === true],
    ['allowBuilds', 'SEC-SUP-033', 'must be empty', candidate => object(candidate) && Object.keys(candidate).length === 0],
    ['dangerouslyAllowAllBuilds', 'SEC-SUP-033', 'must be false', candidate => candidate === false],
    ['blockExoticSubdeps', 'SEC-SUP-033', 'must be true', candidate => candidate === true],
    ['engineStrict', 'SEC-SUP-033', 'must be true', candidate => candidate === true],
    ['autoInstallPeers', 'SEC-SUP-033', 'must be false', candidate => candidate === false],
    ['strictPeerDependencies', 'SEC-SUP-033', 'must be true', candidate => candidate === true],
    ['minimumReleaseAge', 'SEC-SUP-034', 'must be at least 10080 minutes', candidate => typeof candidate === 'number' && Number.isFinite(candidate) && candidate >= 10080],
    ['minimumReleaseAgeStrict', 'SEC-SUP-034', 'must be true', candidate => candidate === true],
    ['minimumReleaseAgeIgnoreMissingTime', 'SEC-SUP-034', 'must be false', candidate => candidate === false],
    ['minimumReleaseAgeExclude', 'SEC-SUP-034', 'must be empty', candidate => Array.isArray(candidate) && candidate.length === 0],
    ['trustPolicy', 'SEC-SUP-034', 'must be no-downgrade', candidate => candidate === 'no-downgrade'],
    ['trustPolicyExclude', 'SEC-SUP-034', 'must be empty', candidate => Array.isArray(candidate) && candidate.length === 0],
    ['trustLockfile', 'SEC-SUP-034', 'must be false', candidate => candidate === false],
    ['registries', 'SEC-SUP-033', 'must contain only registry.npmjs.org', candidate => object(candidate) && Object.keys(candidate).length === 1 && candidate.default === 'https://registry.npmjs.org/'],
  ];
  const findings = rules.flatMap(([key, rule, message, accepts]) =>
    accepts(value[key]) ? [] : finding(rule, `workspace.${key}`, message));
  if ('trustPolicyIgnoreAfter' in value) {
    findings.push(...finding('SEC-SUP-034', 'workspace.trustPolicyIgnoreAfter', 'must be absent'));
  }
  for (const key of ['namedRegistries', 'packageConfigs', 'configDependencies', 'pnpmfile', 'globalPnpmfile', 'patchedDependencies']) {
    if (key in value) findings.push(...finding('SEC-SUP-033', `workspace.${key}`, 'must be absent'));
  }
  return findings;
}

// This reads YAML 1.2, where `<<` is an ordinary key and a directive changes nothing it relies on.
// The manager reads the same files with its own parser, which may merge `<<` or obey a directive, so a
// document carrying either could mean one thing here and another there. Both are refused outright.
export function parseYaml(text: string): unknown[] {
    if (/^%/m.test(text)) throw new Error('directives are forbidden');
    const documents = parseAllDocuments(text, {
      strict: true, uniqueKeys: true, stringKeys: true, version: '1.2',
    });
    if (documents.length === 0) throw new Error('invalid YAML');
    return documents.map(document => {
      if (document.errors.length !== 0 || document.warnings.length !== 0) throw new Error('invalid YAML');
      visit(document, { Alias: () => { throw new Error('aliases are forbidden'); } });
      const value = document.toJS({ maxAliasCount: 0 }) as unknown;
      if (mergeKey(value)) throw new Error('merge keys are forbidden');
      return value;
    });
}

function mergeKey(value: unknown): boolean {
  if (Array.isArray(value)) return value.some(mergeKey);
  return object(value) && Object.entries(value).some(([key, entry]) => key === '<<' || mergeKey(entry));
}

export function lockfile(input: unknown): Finding[] {
  const invalid = (): Finding[] => finding('SEC-SUP-033', 'lockfile', 'invalid or ambiguous YAML');
  if (!object(input) || typeof input.text !== 'string') return invalid();
  let values: unknown[];
  try {
    values = parseYaml(input.text);
  } catch {
    return invalid();
  }
  const findings: Finding[] = [];
  for (const [index, value] of values.entries()) {
    // The shape pnpm 12.7.0 writes, and nothing else: `importers` always; `packages` and `snapshots`
    // together, or neither when the document locks no registry package (workspace-lock.test.ts has the
    // pinned manager write such a document); and no other top-level key, so nothing sits where it is not read.
    if (!object(value) || value.lockfileVersion !== '9.0' || !object(value.importers) ||
        Object.keys(value).some(key => !['lockfileVersion', 'settings', 'importers', 'packages', 'snapshots'].includes(key)) ||
        ('packages' in value) !== ('snapshots' in value)) return invalid();
    const packages = 'packages' in value ? value.packages : {};
    const snapshots = 'snapshots' in value ? value.snapshots : {};
    if (!object(packages) || !object(snapshots)) return invalid();
    const importers = withoutWorkspaceMembers(value.importers);
    if (unsupportedProtocol(importers)) {
      findings.push(...finding('SEC-SUP-033', `lockfile[${index}].importers`, 'unsupported dependency protocol is forbidden'));
    } else if (exoticSource(importers)) {
      findings.push(...finding('SEC-SUP-033', `lockfile[${index}].importers`, 'exotic dependency sources are forbidden'));
    }
    if (unsupportedProtocol(snapshots)) {
      findings.push(...finding('SEC-SUP-033', `lockfile[${index}].snapshots`, 'unsupported dependency protocol is forbidden'));
    } else if (exoticSource(snapshots)) {
      findings.push(...finding('SEC-SUP-033', `lockfile[${index}].snapshots`, 'exotic dependency sources are forbidden'));
    }
    for (const [key, entry] of Object.entries(packages)) {
      const path = `lockfile[${index}].packages.${key}`;
      const identity = /^(?:(@[a-z0-9._-]+\/[a-z0-9._-]+)|([a-z0-9._-]+))@(\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?)$/.exec(key);
      if (identity === null) {
        findings.push(...finding('SEC-SUP-033', path, 'only canonical registry package identities are allowed'));
      }
      const name = identity?.[1] ?? identity?.[2] ?? '';
      if (tracking(name)) {
        findings.push(...finding('SEC-CLI-027', path, `tracking package ${name} is forbidden`));
      }
      // A registry package is locked by one SHA-512 integrity and nothing else: 64 bytes, so 86 base64 characters and `==`.
      const resolution = object(entry) ? entry.resolution : null;
      if (!object(resolution) || Object.keys(resolution).length !== 1 || typeof resolution.integrity !== 'string' ||
          !/^sha512-[A-Za-z0-9+/]{86}==$/.test(resolution.integrity)) {
        findings.push(...finding('SEC-SUP-033', `${path}.resolution`, 'registry integrity only; exotic sources are forbidden'));
      }
    }
  }
  return findings;
}

function tracking(name: string): boolean {
  const scopes = ['@amplitude/', '@bugsnag/', '@datadog/', '@honeybadger-io/', '@opentelemetry/', '@segment/', '@sentry/'];
  const names = ['@firebase/analytics', '@firebase/analytics-compat', 'analytics', 'bugsnag-js', 'mixpanel-browser', 'newrelic', 'posthog-js', 'rollbar'];
  return scopes.some(scope => name.startsWith(scope)) || names.includes(name);
}

function exoticSource(value: unknown): boolean {
  if (typeof value === 'string') {
    return /^(?:https?:|git(?:\+[^:]+)?:|github:|gitlab:|bitbucket:|ssh:|file:|link:|gh:)/i.test(value);
  }
  if (Array.isArray(value)) return value.some(exoticSource);
  if (object(value)) return Object.values(value).some(exoticSource);
  return false;
}

function unsupportedProtocol(value: unknown): boolean {
  if (typeof value === 'string') return /^(?:npm:|catalog:|workspace:)/i.test(value);
  if (Array.isArray(value)) return value.some(unsupportedProtocol);
  if (object(value)) return Object.values(value).some(unsupportedProtocol);
  return false;
}

// pnpm records a dependency on another project of the same workspace as `specifier: workspace:*`
// with `version: link:<path to that project>`. Those entries are the one non-registry source, so they
// are dropped here and every importer string that remains has to be a registry one.
function withoutWorkspaceMembers(importers: ObjectValue): ObjectValue {
  const member = (project: string, entry: unknown): boolean => {
    if (!object(entry) || entry.specifier !== 'workspace:*' || typeof entry.version !== 'string' || !entry.version.startsWith('link:')) return false;
    const target = posix.join(project, entry.version.slice(5));
    return target !== project && Object.hasOwn(importers, target);
  };
  const registry = (project: string, entries: unknown): unknown => !object(entries) ? entries :
    Object.fromEntries(Object.entries(entries).filter(([, entry]) => !member(project, entry)));
  return Object.fromEntries(Object.entries(importers).map(([project, groups]) => [project, !object(groups) ? groups :
    Object.fromEntries(Object.entries(groups).map(([group, entries]) => [group, registry(project, entries)]))]));
}

export function manifest(input: unknown): Finding[] {
  const invalid = (): Finding[] => finding('SEC-SUP-035', 'manifest', 'invalid manifest dependency groups');
  if (!object(input) || !object(input.manifest)) return invalid();
  const value = input.manifest;
  const members = Array.isArray(input.members) ? input.members : [];
  const findings: Finding[] = [];
  for (const field of ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies']) {
    const entries = value[field];
    if (entries === undefined) continue;
    if (!object(entries)) return invalid();
    for (const [name, version] of Object.entries(entries)) {
      const workspace = version === 'workspace:*' && members.includes(name);
      if (field === 'peerDependencies' || field === 'optionalDependencies') {
        if (!workspace) findings.push(...finding('SEC-SUP-035', `manifest.${field}.${name}`, 'only a known workspace member with workspace:* is allowed'));
      } else if (!workspace && (typeof version !== 'string' || !/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?$/.test(version))) {
        findings.push(...finding('SEC-SUP-033', `manifest.${field}.${name}`, 'only exact registry versions or known workspace:* members are allowed'));
      }
    }
  }
  return findings;
}

export function directDependencies(input: unknown): Finding[] {
  const invalid = (): Finding[] => finding('SEC-SUP-035', 'supply-chain/js-direct-deps.toml', 'dependency reason list is invalid');
  if (!object(input) || !Array.isArray(input.manifests) || typeof input.list !== 'string') return invalid();
  const listed = new Map<string, string>();
  for (const line of input.list.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed === '' || trimmed.startsWith('#')) continue;
    // One row: a package name, bare or in double quotes, then ` = ` and a single double-quoted reason
    // that holds no quote and no escape. Nothing may follow it, so a comment cannot pose as the reason.
    const row = /^("?)([a-z0-9@/._-]+)\1 = "([^"\\]*)"$/i.exec(trimmed);
    if (row === null || listed.has(row[2] as string)) return invalid();
    listed.set(row[2] as string, row[3] as string);
  }
  const used = new Set<string>();
  for (const entry of input.manifests) {
    if (!object(entry) || typeof entry.path !== 'string' || !object(entry.manifest)) return invalid();
    for (const field of ['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies']) {
      const group = entry.manifest[field];
      if (group === undefined) continue;
      if (!object(group)) return invalid();
      // Every entry that is not a workspace member needs its reason, however its version is written:
      // whether that version is acceptable is another check's question, and this one fails closed alone.
      for (const [name, version] of Object.entries(group)) {
        if (version !== 'workspace:*') used.add(name);
      }
    }
  }
  const findings: Finding[] = [];
  for (const name of [...used].sort()) {
    const reason = listed.get(name);
    if (reason === undefined) findings.push(...finding('SEC-SUP-035', `supply-chain/js-direct-deps.toml.${name}`, 'direct registry dependency is missing a written reason'));
    else if (reason.trim() === '') findings.push(...finding('SEC-SUP-035', `supply-chain/js-direct-deps.toml.${name}`, 'direct registry dependency must have a written reason'));
  }
  for (const name of [...listed.keys()].sort()) {
    if (!used.has(name)) findings.push(...finding('SEC-SUP-035', `supply-chain/js-direct-deps.toml.${name}`, 'reviewed dependency is not used by any manifest'));
  }
  return findings;
}

export function age(input: unknown): Finding[] {
  const published = object(input) && typeof input.published === 'string' ? Date.parse(input.published) : NaN;
  const now = object(input) && typeof input.now === 'string' ? Date.parse(input.now) : NaN;
  return Number.isFinite(published) && Number.isFinite(now) && now - published >= 604800000 ? [] :
    finding('SEC-SUP-034', 'pnpm.publication', 'publication must be known and at least seven days old');
}

export function licenses(input: unknown): Finding[] {
  const invalid = (): Finding[] => finding('SEC-SUP-029', 'licenses', 'invalid licence report');
  if (!object(input) || !Array.isArray(input.allowed) || !object(input.report)) return invalid();
  const allowed = input.allowed;
  const findings: Finding[] = [];
  for (const [license, entries] of Object.entries(input.report)) {
    if (!Array.isArray(entries)) return invalid();
    for (const entry of entries) {
      if (!object(entry) || typeof entry.name !== 'string' ||
          !Array.isArray(entry.versions) || entry.versions.length === 0 || entry.versions.some(version => typeof version !== 'string') ||
          !Array.isArray(entry.paths) || entry.paths.length === 0 || entry.paths.some(path => typeof path !== 'string') ||
          entry.license !== license) return invalid();
      if (typeof entry.license !== 'string' || !allowed.includes(entry.license)) {
        findings.push(...finding('SEC-SUP-029', `licenses.${entry.name}`, `licence ${entry.license || 'UNKNOWN'} is not allowed`));
      } else if (entry.license === 'MPL-2.0' && typeof entry.licenseText === 'string' && incompatibleMpl(entry.licenseText)) {
        findings.push(...finding('SEC-SUP-029', `licenses.${entry.name}`, 'licence MPL-2.0 marked incompatible with secondary licences is not allowed'));
      }
    }
  }
  return findings;
}

function incompatibleMpl(text: string): boolean {
  const normalized = text.replace(/[^a-z0-9]/gi, '').toLowerCase();
  // The complete SPDX MPL-2.0 text includes Exhibit B as a template, not an applied declaration.
  // Only this complete standard text is exempt; appended or separate notices are still inspected.
  // Primary source: https://raw.githubusercontent.com/spdx/license-list-data/main/text/MPL-2.0.txt
  if (createHash('sha256').update(normalized).digest('hex') === 'af1cb4cf22a4f33ded93b4bb0fc5bb3b5827a013d227b18150e28d55edc24f10') return false;
  return normalized.includes('thissourcecodeformisincompatiblewithsecondarylicenses');
}

export function effectiveRegistries(input: unknown): Finding[] {
  const invalid = (path: string): Finding[] => finding('SEC-SUP-033', path, 'only registry.npmjs.org is allowed');
  if (!object(input) || !object(input.config) || input.config.registry !== 'https://registry.npmjs.org/' ||
      !Array.isArray(input.names) || !object(input.config.registries)) return invalid('effective.registry');
  const config = input.config;
  const claimed = (scope: string): boolean => Object.entries(config.registries as ObjectValue).some(([url, declaration]) =>
    url !== 'https://registry.npmjs.org/' && object(declaration) && Array.isArray(declaration.scopes) && declaration.scopes.includes(scope));
  // In pnpm's report `@` is the scope of the default registry, so another registry holding it takes every package.
  if (claimed('@')) return invalid('effective.registry');
  const findings: Finding[] = [];
  for (const name of input.names) {
    if (typeof name !== 'string') return invalid('effective.registry');
    if (!name.startsWith('@')) continue;
    const scope = name.slice(0, name.indexOf('/'));
    const explicit = config[`${scope}:registry`];
    if ((explicit !== undefined && explicit !== 'https://registry.npmjs.org/') || claimed(scope)) {
      findings.push(...invalid(`effective.${name}`));
    }
  }
  return findings;
}

// Reads `allow` from the `[licenses]` table of the project's deny.toml, with no TOML library and no other
// program. It knows one spelling, the one that file uses: the table once, `allow = [` once, then one
// double-quoted licence and a comma on each line up to `]`. Anything else returns null, so the caller
// fails closed instead of guessing what a full TOML reader would have made of it.
export function licenceAllowList(text: string): string[] | null {
  if (text.includes('"""') || text.includes("'''")) return null;
  const lines = text.split(/\r?\n/).map(line => line.trim());
  const start = lines.indexOf('[licenses]');
  if (start === -1 || lines.lastIndexOf('[licenses]') !== start) return null;
  const next = lines.findIndex((line, index) => index > start && line.startsWith('['));
  const table = lines.slice(start + 1, next === -1 ? lines.length : next);
  const open = table.indexOf('allow = [');
  const close = table.indexOf(']');
  if (open === -1 || close < open || table.some((line, index) => index !== open && /^"?allow"?\s*=/.test(line))) return null;
  const allowed: string[] = [];
  for (const line of table.slice(open + 1, close)) {
    const licence = /^"([^"\\]*)",$/.exec(line);
    if (licence === null) return null;
    allowed.push(licence[1] as string);
  }
  return allowed;
}

function identities(input: unknown, integrity: boolean): string[] | null {
  if (!Array.isArray(input)) return null;
  const values: string[] = [];
  for (const entry of input) {
    if (!object(entry) || typeof entry.name !== 'string' || typeof entry.version !== 'string' ||
        (integrity && typeof entry.integrity !== 'string')) return null;
    values.push(JSON.stringify([entry.name, entry.version, ...(integrity ? [entry.integrity] : [])]));
  }
  return new Set(values).size === values.length ? values.sort() : null;
}

export function verificationInventory(input: unknown): Finding[] {
  const installed = object(input) ? identities(input.installed, true) : null;
  const verified = object(input) ? identities(input.verified, true) : null;
  return installed !== null && verified !== null && JSON.stringify(installed) === JSON.stringify(verified) ? [] :
    finding('SEC-SUP-036', 'verification.inventory', 'verifier inventory must exactly match installed package identities and integrity');
}

export function signatureReport(input: unknown): Finding[] {
  const invalid = (): Finding[] => finding('SEC-SUP-036', 'verification.signatures', 'all signatures and present provenance must verify for the exact installed inventory');
  if (!object(input) || !object(input.report)) return invalid();
  const report = input.report;
  if (!Array.isArray(report.invalid) || report.invalid.length !== 0 ||
      !Array.isArray(report.missing) || report.missing.length !== 0 || !Array.isArray(report.verified)) return invalid();
  if (report.verified.some(entry => !object(entry) || entry.registry !== 'https://registry.npmjs.org/' || !object(entry.attestations))) return invalid();
  const present = identities(input.present, false);
  const verified = identities(report.verified, false);
  return present !== null && verified !== null && JSON.stringify(present) === JSON.stringify(verified) ? [] : invalid();
}

export function licenseInventory(input: unknown): Finding[] {
  const invalid = (): Finding[] => finding('SEC-SUP-029', 'licenses.inventory', 'licence report must exactly cover every installed package version');
  if (!object(input) || !object(input.report)) return invalid();
  const installed = identities(input.installed, false);
  const rows: ObjectValue[] = [];
  for (const entries of Object.values(input.report)) {
    if (!Array.isArray(entries)) return invalid();
    for (const entry of entries) {
      if (!object(entry) || !Array.isArray(entry.versions)) return invalid();
      for (const version of entry.versions) rows.push({ name: entry.name, version });
    }
  }
  const reported = identities(rows, false);
  return installed !== null && reported !== null && JSON.stringify(installed) === JSON.stringify(reported) ? [] : invalid();
}

export function inspect(kind: string | undefined, input: unknown): Finding[] {
  switch (kind) {
    case 'settings': return settings(input);
    case 'lockfile': return lockfile(input);
    case 'manifest': return manifest(input);
    case 'direct-dependencies': return directDependencies(input);
    case 'age': return age(input);
    case 'licenses': return licenses(input);
    case 'effective-registries': return effectiveRegistries(input);
    case 'verification-inventory': return verificationInventory(input);
    case 'signature-report': return signatureReport(input);
    case 'license-inventory': return licenseInventory(input);
    default: return finding('SEC-SUP-033', 'input', 'unknown policy check');
  }
}
