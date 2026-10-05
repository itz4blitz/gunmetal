import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const client = fileURLToPath(new URL('../../../', import.meta.url));
function verify(directory: string): { status: number | null; signal: string | null; findings: unknown; stderr: string } {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('verify.ts', import.meta.url)), directory], {
    encoding: 'utf8', timeout: 180000,
  });
  return { status: result.status, signal: result.signal, findings: result.stdout.trim() === '' ? null : JSON.parse(result.stdout), stderr: result.stderr };
}

// Verifies: SEC-SUP-036. Real registry verification must cover both the installed manager and project graph.
test('real signature and provenance audit covers the exact installed project and manager inventory', () => {
  const identities = [
    { name: '@pnpm/exe.linux-x64', version: '12.7.0', integrity: 'sha512-gGW7NJFmr33IJ6KZu+1w90KBtFxMVf/+AUG8aKJpy4v6dRnUd5v84PcH2ejUuxp/fm3zM0IY6h9LwcYPu9dxdg==' },
    { name: 'yaml', version: '2.9.1', integrity: 'sha512-3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==' },
  ];
  assert.deepEqual(verify(client), {
    status: 0, signal: null, stderr: '', findings: {
      installed: identities, verified: identities,
      signatures: { invalid: [], missing: [], provenance: [{ name: '@pnpm/exe.linux-x64', version: '12.7.0' }] },
    },
  });
});

// Verifies: SEC-SUP-036. The collector observes physical manifests, rather than trusting a supplied inventory.
test('a counterfeit physically installed identity fails before signature verification', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-counterfeit-install-'));
  try {
    for (const file of ['package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml']) await cp(join(client, file), join(directory, file));
    await cp(join(client, 'node_modules'), join(directory, 'node_modules'), { recursive: true });
    const path = join(directory, 'node_modules/.pnpm/yaml@2.9.1/node_modules/yaml/package.json');
    const manifest = JSON.parse(await readFile(path, 'utf8')) as Record<string, unknown>;
    await writeFile(path, JSON.stringify({ ...manifest, name: 'counterfeit' }));
    assert.deepEqual(verify(directory), {
      status: 1, signal: null, stderr: '', findings: [{
        rule: 'SEC-SUP-036', path: 'verification.installation',
        message: 'installed package identity must match its locked registry identity',
      }],
    });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
