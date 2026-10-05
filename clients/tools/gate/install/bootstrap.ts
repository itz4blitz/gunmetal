import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const checksum = '3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==';
class Refusal extends Error {
  findings: { rule: string; path: string; message: string }[];
  constructor(path: string, message: string) { super(message); this.findings = [{ rule: 'SEC-SUP-011', path, message }]; }
}
async function check(directory: string, cache: string | undefined): Promise<void> {
  if (process.versions.node !== '24.20.0') throw new Refusal('runtime.node', 'observed runtime must match the bootstrap pin');
  let archive: Uint8Array;
  if (cache !== undefined) archive = await readFile(cache);
  else {
    const response = await fetch('https://registry.npmjs.org/yaml/-/yaml-2.9.1.tgz', { signal: AbortSignal.timeout(30000) });
    if (!response.ok) throw new Refusal('bootstrap.parser', 'pinned parser archive is unavailable');
    archive = new Uint8Array(await response.arrayBuffer());
  }
  if (createHash('sha512').update(archive).digest('base64') !== checksum) {
    throw new Refusal('bootstrap.parser', 'parser archive must match the pinned checksum');
  }
  const tools = await mkdtemp(join(tmpdir(), 'gunmetal-parser-bootstrap-'));
  try {
    const parser = join(tools, 'node_modules/yaml');
    await mkdir(parser, { recursive: true });
    await writeFile(join(tools, 'yaml.tgz'), archive);
    const extraction = spawnSync('tar', ['--extract', '--gzip', '--file', join(tools, 'yaml.tgz'), '--directory', parser, '--strip-components=1'], { encoding: 'utf8', timeout: 10000 });
    if (extraction.status !== 0 || extraction.signal !== null) throw new Refusal('bootstrap.parser', 'verified parser archive could not be extracted');
    await writeFile(join(tools, 'package.json'), JSON.stringify({ private: true, type: 'module' }));
    for (const file of ['installation.ts', 'policy.ts', 'verify.ts']) {
      await cp(fileURLToPath(new URL(file, import.meta.url)), join(tools, file));
    }
    const result = spawnSync(process.execPath, [join(tools, 'installation.ts'), resolve(directory)], { encoding: 'utf8', timeout: 120000 });
    if ((result.status !== 0 && result.status !== 1) || result.signal !== null || result.stderr !== '') {
      throw new Refusal('bootstrap.execution', 'workspace check could not complete');
    }
    const value: unknown = JSON.parse(result.stdout);
    process.stdout.write(`${JSON.stringify(value)}\n`);
    process.exitCode = result.status;
  } finally { await rm(tools, { recursive: true, force: true }); }
}

try {
  const directory = process.argv[2];
  if (directory === undefined) throw new Refusal('bootstrap.execution', 'client workspace directory is required');
  await check(directory, process.argv[3]);
} catch (error) {
  const findings = error instanceof Refusal ? error.findings : [{ rule: 'SEC-SUP-011', path: 'bootstrap.execution', message: 'checksum-pinned parser bootstrap could not complete' }];
  process.stdout.write(`${JSON.stringify(findings)}\n`);
  process.exitCode = 1;
}
