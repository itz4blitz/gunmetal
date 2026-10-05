import { chmod, writeFile } from 'node:fs/promises';
import { delimiter, join } from 'node:path';

// Test support: a stand-in for a program an attacker, a dependency's bin or a stale image leaves
// first on PATH. Running it leaves a marker beside it, so a test can prove it never ran.
export async function decoy(directory: string, name: string): Promise<{ env: NodeJS.ProcessEnv; script: string; marker: string }> {
  const script = join(directory, name);
  await writeFile(script, '#!/bin/sh\nprintf ran > "$0.ran"\n');
  await chmod(script, 0o755);
  return { env: { ...process.env, PATH: `${directory}${delimiter}${process.env.PATH ?? ''}` }, script, marker: `${script}.ran` };
}
