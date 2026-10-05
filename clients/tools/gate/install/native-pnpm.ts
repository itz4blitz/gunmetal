import { createHash } from 'node:crypto';
import { readFile, realpath, stat } from 'node:fs/promises';
import { join } from 'node:path';

type Finding = { rule: string; path: string; message: string };
export class NativePnpmRefusal extends Error {
  findings: Finding[];
  constructor(message: string) {
    super(message);
    this.findings = [{ rule: 'SEC-SUP-011', path: 'runtime.pnpm', message }];
  }
}

export const nativePnpmChecksum = 'gGW7NJFmr33IJ6KZu+1w90KBtFxMVf/+AUG8aKJpy4v6dRnUd5v84PcH2ejUuxp/fm3zM0IY6h9LwcYPu9dxdg==';

// The TeamCity bootstrap checks the archive before extraction and retains it
// beside the extracted package, so this check binds every execution to those bytes.
export async function nativePnpm(root = process.env.GUNMETAL_NATIVE_PNPM_ROOT, checksum = nativePnpmChecksum): Promise<string> {
  if (typeof root !== 'string' || root === '') throw new NativePnpmRefusal('native pnpm root is required');
  const archive = join(root, 'pnpm.tgz');
  const packageRoot = join(root, 'node_modules/@pnpm/exe.linux-x64');
  const expected = join(packageRoot, 'pnpm');
  const bin = join(root, 'bin/pnpm');
  try {
    const info = await stat(archive);
    if (!info.isFile() || createHash('sha512').update(await readFile(archive)).digest('base64') !== checksum) {
      throw new NativePnpmRefusal('native pnpm archive must match the pinned checksum');
    }
    const packageInfo = await stat(expected);
    if (!packageInfo.isFile() || (packageInfo.mode & 0o111) === 0 || await realpath(bin) !== await realpath(expected)) {
      throw new NativePnpmRefusal('native pnpm bin entry must resolve to the verified package executable');
    }
    const manifest = JSON.parse(await readFile(join(packageRoot, 'package.json'), 'utf8')) as { name?: unknown; version?: unknown };
    if (manifest.name !== '@pnpm/exe.linux-x64' || manifest.version !== '12.7.0') {
      throw new NativePnpmRefusal('native pnpm package identity must match the pinned manager');
    }
    return expected;
  } catch (error) {
    if (error instanceof NativePnpmRefusal) throw error;
    throw new NativePnpmRefusal('native pnpm layout could not be verified');
  }
}
