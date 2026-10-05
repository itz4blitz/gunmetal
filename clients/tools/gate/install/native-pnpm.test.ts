import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmod, mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { nativePnpm } from './native-pnpm.ts';

async function layout(): Promise<{ root: string; checksum: string; executable: string }> {
  const root = await mkdtemp(join(tmpdir(), 'gunmetal-native-pnpm-'));
  const archive = Buffer.from('verified native pnpm fixture');
  const checksum = createHash('sha512').update(archive).digest('base64');
  await writeFile(join(root, 'pnpm.tgz'), archive);
  const packageRoot = join(root, 'node_modules/@pnpm/exe.linux-x64');
  await mkdir(join(root, 'bin'), { recursive: true });
  await mkdir(packageRoot, { recursive: true });
  const executable = join(packageRoot, 'pnpm');
  await writeFile(executable, '#!/bin/sh\nexit 0\n');
  await chmod(executable, 0o755);
  await writeFile(join(packageRoot, 'package.json'), JSON.stringify({ name: '@pnpm/exe.linux-x64', version: '12.7.0' }));
  await symlink('../node_modules/@pnpm/exe.linux-x64/pnpm', join(root, 'bin/pnpm'));
  return { root, checksum, executable };
}

// Verifies: SEC-SUP-011, SEC-SUP-033. TeamCity must use the checksum-bound native manager layout, never PATH discovery.
test('native pnpm accepts only the verified extracted package reached through its bin directory', async () => {
  const fixture = await layout();
  try {
    assert.deepEqual(await nativePnpm(fixture.root, fixture.checksum), fixture.executable);
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('native pnpm rejects a bin entry that does not resolve to the verified package executable', async () => {
  const fixture = await layout();
  try {
    await rm(join(fixture.root, 'bin/pnpm'));
    await writeFile(join(fixture.root, 'bin/pnpm'), '#!/bin/sh\nexit 0\n');
    await chmod(join(fixture.root, 'bin/pnpm'), 0o755);
    await assert.rejects(nativePnpm(fixture.root, fixture.checksum), {
      findings: [{ rule: 'SEC-SUP-011', path: 'runtime.pnpm', message: 'native pnpm bin entry must resolve to the verified package executable' }],
    });
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('native pnpm rejects an archive whose bytes do not match the provisioned checksum', async () => {
  const fixture = await layout();
  try {
    await writeFile(join(fixture.root, 'pnpm.tgz'), 'substituted archive');
    await assert.rejects(nativePnpm(fixture.root, fixture.checksum), {
      findings: [{ rule: 'SEC-SUP-011', path: 'runtime.pnpm', message: 'native pnpm archive must match the pinned checksum' }],
    });
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});
