import { createHash } from 'node:crypto';
import { readFile, realpath, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { gunzipSync } from 'node:zlib';

type Finding = { rule: string; path: string; message: string };
// The one refusal type every install check catches, so a finding raised here reaches the result as itself.
export class Refusal extends Error {
  findings: Finding[];
  constructor(findings: Finding[]) { super('installation verification refused'); this.findings = findings; }
}
export class NativePnpmRefusal extends Refusal {
  constructor(message: string) { super([{ rule: 'SEC-SUP-011', path: 'runtime.pnpm', message }]); }
}

export const nativePnpmChecksum = 'gGW7NJFmr33IJ6KZu+1w90KBtFxMVf/+AUG8aKJpy4v6dRnUd5v84PcH2ejUuxp/fm3zM0IY6h9LwcYPu9dxdg==';

// Reads an npm package archive: gzip over ustar, regular files directly under `package/`.
// Any other entry is refused rather than skipped.
export function archiveFiles(archive: Uint8Array): Map<string, Buffer> {
  const tar = gunzipSync(archive);
  const files = new Map<string, Buffer>();
  let offset = 0;
  while (offset + 512 <= tar.length && tar[offset] !== 0) {
    const header = tar.subarray(offset, offset + 512);
    const name = header.subarray(0, 100).toString('latin1').replace(/\0[\s\S]*$/, '');
    const size = Number.parseInt(header.subarray(124, 136).toString('latin1'), 8);
    const start = offset + 512;
    if (header[156] !== 0x30 || !/^package\/[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name) || !(size >= 0) || start + size > tar.length) {
      throw new Error('unsupported native pnpm archive entry');
    }
    files.set(name, tar.subarray(start, start + size));
    offset = start + Math.ceil(size / 512) * 512;
  }
  return files;
}

// Before every use this proves, in order: the retained archive has the pinned SHA-512; `bin/pnpm`
// resolves to the package executable; that executable and its `package.json` are byte for byte the
// entries of the archive just checked; and they name the pinned manager.
// It does not cover the other files beside the executable, the Node runtime running this check, or a
// writer that replaces the executable between this comparison and the caller's exec of the returned path.
export async function nativePnpm(root = process.env.GUNMETAL_NATIVE_PNPM_ROOT, checksum = nativePnpmChecksum): Promise<string> {
  if (typeof root !== 'string' || root === '') throw new NativePnpmRefusal('native pnpm root is required');
  const archive = join(root, 'pnpm.tgz');
  const packageRoot = join(root, 'node_modules/@pnpm/exe.linux-x64');
  const expected = join(packageRoot, 'pnpm');
  const bin = join(root, 'bin/pnpm');
  try {
    const info = await stat(archive);
    const bytes = info.isFile() ? await readFile(archive) : null;
    if (bytes === null || createHash('sha512').update(bytes).digest('base64') !== checksum) {
      throw new NativePnpmRefusal('native pnpm archive must match the pinned checksum');
    }
    const files = archiveFiles(bytes);
    const packageInfo = await stat(expected);
    if (!packageInfo.isFile() || (packageInfo.mode & 0o111) === 0 || await realpath(bin) !== await realpath(expected)) {
      throw new NativePnpmRefusal('native pnpm bin entry must resolve to the verified package executable');
    }
    const verified = async (name: string): Promise<Buffer> => {
      const packed = files.get(`package/${name}`);
      if (packed === undefined || !packed.equals(await readFile(join(packageRoot, name)))) {
        throw new NativePnpmRefusal('native pnpm package files must match the verified archive');
      }
      return packed;
    };
    await verified('pnpm');
    const manifest = JSON.parse((await verified('package.json')).toString('utf8')) as { name?: unknown; version?: unknown };
    if (manifest.name !== '@pnpm/exe.linux-x64' || manifest.version !== '12.7.0') {
      throw new NativePnpmRefusal('native pnpm package identity must match the pinned manager');
    }
    return expected;
  } catch (error) {
    if (error instanceof NativePnpmRefusal) throw error;
    throw new NativePnpmRefusal('native pnpm layout could not be verified');
  }
}
