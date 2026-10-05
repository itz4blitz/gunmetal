import { parseAllDocuments, visit } from 'yaml';
import { createHash } from 'node:crypto';

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

export function parseYaml(text: string): unknown[] {
    const documents = parseAllDocuments(text, {
      strict: true, uniqueKeys: true, stringKeys: true, version: '1.2',
    });
    if (documents.length === 0) throw new Error('invalid YAML');
    return documents.map(document => {
      if (document.errors.length !== 0 || document.warnings.length !== 0) throw new Error('invalid YAML');
      visit(document, { Alias: () => { throw new Error('aliases are forbidden'); } });
      return document.toJS({ maxAliasCount: 0 }) as unknown;
    });
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
    if (!object(value) || value.lockfileVersion !== '9.0' || !object(value.packages)) return invalid();
    if (unsupportedProtocol(value.importers, true)) {
      findings.push(...finding('SEC-SUP-033', `lockfile[${index}].importers`, 'unsupported dependency protocol is forbidden'));
    } else if (exoticSource(value.importers)) {
      findings.push(...finding('SEC-SUP-033', `lockfile[${index}].importers`, 'exotic dependency sources are forbidden'));
    }
    if (unsupportedProtocol(value.snapshots, false)) {
      findings.push(...finding('SEC-SUP-033', `lockfile[${index}].snapshots`, 'unsupported dependency protocol is forbidden'));
    } else if (exoticSource(value.snapshots)) {
      findings.push(...finding('SEC-SUP-033', `lockfile[${index}].snapshots`, 'exotic dependency sources are forbidden'));
    }
    for (const [key, entry] of Object.entries(value.packages)) {
      const path = `lockfile[${index}].packages.${key}`;
      const identity = /^(?:(@[a-z0-9._-]+\/[a-z0-9._-]+)|([a-z0-9._-]+))@(\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?)$/.exec(key);
      if (identity === null) {
        findings.push(...finding('SEC-SUP-033', path, 'only canonical registry package identities are allowed'));
      }
      const name = identity?.[1] ?? identity?.[2] ?? '';
      if (tracking(name)) {
        findings.push(...finding('SEC-CLI-027', path, `tracking package ${name} is forbidden`));
      }
      const resolution = object(entry) ? entry.resolution : null;
      if (!object(resolution) || typeof resolution.integrity !== 'string' ||
          Object.keys(resolution).some(field => field !== 'integrity' && field !== 'revision')) {
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

function unsupportedProtocol(value: unknown, allowWorkspace: boolean): boolean {
  if (typeof value === 'string') {
    if (/^(?:npm:|catalog:)/i.test(value)) return true;
    return /^workspace:/i.test(value) && (!allowWorkspace || value !== 'workspace:*');
  }
  if (Array.isArray(value)) return value.some(entry => unsupportedProtocol(entry, allowWorkspace));
  if (object(value)) return Object.values(value).some(entry => unsupportedProtocol(entry, allowWorkspace));
  return false;
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
    const boundary = trimmed.indexOf(' = ');
    if (boundary === -1) return invalid();
    const rawName = trimmed.slice(0, boundary);
    const name = rawName.startsWith('"') && rawName.endsWith('"') ? rawName.slice(1, -1) : rawName;
    const rawReason = trimmed.slice(boundary + 3);
    if (!rawReason.startsWith('"') || !rawReason.endsWith('"')) return invalid();
    const reason = rawReason.slice(1, -1);
    if (!/^[a-z0-9@/._-]+$/i.test(name) || listed.has(name)) return invalid();
    listed.set(name, reason);
  }
  const used = new Set<string>();
  for (const entry of input.manifests) {
    if (!object(entry) || typeof entry.path !== 'string' || !object(entry.manifest)) return invalid();
    for (const field of ['dependencies', 'devDependencies']) {
      const group = entry.manifest[field];
      if (group === undefined) continue;
      if (!object(group)) return invalid();
      for (const [name, version] of Object.entries(group)) {
        if (version === 'workspace:*') continue;
        if (typeof version === 'string' && /^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?$/.test(version)) used.add(name);
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
  const findings: Finding[] = [];
  for (const name of input.names) {
    if (typeof name !== 'string') return invalid('effective.registry');
    if (!name.startsWith('@')) continue;
    const scope = name.slice(0, name.indexOf('/'));
    const explicit = config[`${scope}:registry`];
    const alternatives = Object.entries(config.registries as ObjectValue).filter(([url, declaration]) =>
      url !== 'https://registry.npmjs.org/' && object(declaration) && Array.isArray(declaration.scopes) && declaration.scopes.includes(scope));
    if ((explicit !== undefined && explicit !== 'https://registry.npmjs.org/') || alternatives.length !== 0) {
      findings.push(...invalid(`effective.${name}`));
    }
  }
  return findings;
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
