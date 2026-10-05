import { spawnSync } from 'node:child_process';
import { readFile, readdir, realpath } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { collectInstalled, Refusal } from './verify.ts';
import { inspect, licenceAllowList } from './policy.ts';

type ObjectValue = Record<string, unknown>;
function object(value: unknown): value is ObjectValue {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function invalidPolicy(): never {
  throw new Refusal([{ rule: 'SEC-SUP-029', path: 'licenses.policy', message: 'invalid project licence allow-list' }]);
}
async function check(directory: string): Promise<unknown> {
  // The project's one licence policy is read in-process, so no program found on PATH decides what is allowed.
  let allowed: string[] | null;
  try { allowed = licenceAllowList(await readFile(join(dirname(directory), 'deny.toml'), 'utf8')); }
  catch { return invalidPolicy(); }
  if (allowed === null || allowed.length === 0 || allowed.includes('') || new Set(allowed).size !== allowed.length) invalidPolicy();
  const { installed, manager } = await collectInstalled(directory);
  const packages: { name: string; version: string; license: string }[] = [];
  const findings = [];
  for (const entry of installed) {
    const manifest: unknown = JSON.parse(await readFile(join(entry.path, 'package.json'), 'utf8'));
    const license = object(manifest) && typeof manifest.license === 'string' ? manifest.license : 'UNKNOWN';
    if (!allowed.includes(license)) findings.push({ rule: 'SEC-SUP-029', path: `licenses.${entry.name}`, message: `licence ${license || 'UNKNOWN'} is not allowed` });
    if (license === 'MPL-2.0') {
      const directories = [entry.path];
      for (let current = directories.pop(); current !== undefined; current = directories.pop()) {
        for (const file of await readdir(current, { withFileTypes: true })) {
          const path = join(current, file.name);
          if (file.isSymbolicLink()) throw new Refusal([{ rule: 'SEC-SUP-029', path: `licenses.${entry.name}`, message: 'MPL source symlinks require review' }]);
          if (file.isDirectory() && file.name !== 'node_modules') directories.push(path);
          if (!file.isFile()) continue;
          const refusals = inspect('licenses', { allowed, report: { [license]: [{
            name: entry.name, versions: [entry.version], paths: [entry.path], license,
            licenseText: await readFile(path, 'utf8'),
          }] } });
          if (refusals.length !== 0) throw new Refusal(refusals);
        }
      }
    }
    packages.push({ name: entry.name, version: entry.version, license });
  }
  if (findings.length !== 0) throw new Refusal(findings);
  const result = spawnSync(manager, ['--dir', directory, 'licenses', 'list', '--json'], { encoding: 'utf8', timeout: 60000 });
  if (result.status !== 0) throw new Refusal([{ rule: 'SEC-SUP-029', path: 'licenses', message: 'invalid licence report' }]);
  const report: unknown = JSON.parse(result.stdout);
  if (!object(report)) throw new Refusal([{ rule: 'SEC-SUP-029', path: 'licenses', message: 'invalid licence report' }]);
  // Installed paths are real paths, so the manager's package root is resolved the same way before it is matched.
  const managerRoot = await realpath(dirname(manager));
  const managerPackage = packages.find(entry => installed.find(item => item.name === entry.name && item.version === entry.version)?.path === managerRoot);
  if (managerPackage === undefined) throw new Refusal([{ rule: 'SEC-SUP-029', path: 'licenses.inventory', message: 'licence report must exactly cover every installed package version' }]);
  const group = report[managerPackage.license];
  if (group !== undefined && !Array.isArray(group)) throw new Refusal([{ rule: 'SEC-SUP-029', path: 'licenses', message: 'invalid licence report' }]);
  report[managerPackage.license] = [...(Array.isArray(group) ? group : []), {
    name: managerPackage.name, versions: [managerPackage.version], paths: [managerRoot], license: managerPackage.license,
  }];
  for (const [kind, input] of [
    ['licenses', { allowed, report }],
    ['license-inventory', { installed, report }],
  ] as const) {
    const failures = inspect(kind, input);
    if (failures.length !== 0) throw new Refusal(failures);
  }
  return { allowed, packages };
}

try {
  const directory = process.argv[2];
  if (directory === undefined) invalidPolicy();
  process.stdout.write(`${JSON.stringify(await check(directory))}\n`);
} catch (error) {
  const findings = error instanceof Refusal ? error.findings : [{ rule: 'SEC-SUP-029', path: 'licenses', message: 'invalid licence report' }];
  process.stdout.write(`${JSON.stringify(findings)}\n`);
  process.exitCode = 1;
}
