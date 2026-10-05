import { createHash } from 'node:crypto';
import { chmod, mkdir, readFile, symlink, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { archiveFiles, nativePnpm, nativePnpmChecksum, NativePnpmRefusal, Refusal } from './native-pnpm.ts';

// Builds, from the pinned archive alone, the one layout `native-pnpm.ts` accepts:
//   <root>/pnpm.tgz
//   <root>/node_modules/@pnpm/exe.linux-x64/<the archive's files>
//   <root>/bin/pnpm -> ../node_modules/@pnpm/exe.linux-x64/pnpm
// usage: node native-pnpm-layout.ts <downloaded pnpm.tgz> <root that does not exist yet>
try {
  const [archivePath, root] = process.argv.slice(2);
  if (archivePath === undefined || root === undefined) throw new NativePnpmRefusal('native pnpm archive and layout root are required');
  const archive = await readFile(archivePath);
  if (createHash('sha512').update(archive).digest('base64') !== nativePnpmChecksum) {
    throw new NativePnpmRefusal('native pnpm archive must match the pinned checksum');
  }
  const packageRoot = join(root, 'node_modules/@pnpm/exe.linux-x64');
  await mkdir(root);
  await mkdir(packageRoot, { recursive: true });
  for (const [name, data] of archiveFiles(archive)) await writeFile(join(packageRoot, name.slice('package/'.length)), data);
  await chmod(join(packageRoot, 'pnpm'), 0o755);
  await mkdir(join(root, 'bin'));
  await symlink('../node_modules/@pnpm/exe.linux-x64/pnpm', join(root, 'bin/pnpm'));
  await writeFile(join(root, 'pnpm.tgz'), archive);
  process.stdout.write(`${JSON.stringify({ root, executable: await nativePnpm(root) })}\n`);
} catch (error) {
  const findings = error instanceof Refusal ? error.findings : [{ rule: 'SEC-SUP-011', path: 'runtime.pnpm', message: 'native pnpm layout could not be built' }];
  process.stdout.write(`${JSON.stringify(findings)}\n`);
  process.exitCode = 1;
}
