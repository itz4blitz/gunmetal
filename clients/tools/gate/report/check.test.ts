import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, expect, test, vi } from 'vitest';

afterEach(() => {
  vi.restoreAllMocks();
  vi.resetModules();
  process.exitCode = undefined;
});

// Runs the command in this process on a report file holding `text`: what it printed and its exit status.
async function command(text: string): Promise<unknown> {
  const directory = await mkdtemp(join(tmpdir(), 'gunmetal-mutation-report-'));
  const argv = process.argv;
  try {
    const path = join(directory, 'mutation.json');
    await writeFile(path, text);
    const printed: string[] = [];
    vi.spyOn(process.stdout, 'write').mockImplementation((chunk) => {
      printed.push(String(chunk));
      return true;
    });
    process.argv = ['node', 'check.ts', path];
    await import('./check.ts');
    return { output: printed.join(''), status: process.exitCode };
  } finally {
    process.argv = argv;
    await rm(directory, { recursive: true, force: true });
  }
}
function mutant(status: string): string {
  return JSON.stringify({
    files: {
      'tools/lint/no-danger.ts': {
        mutants: [{ mutatorName: 'StringLiteral', location: { start: { line: 9, column: 27 } }, status }],
      },
    },
  });
}

test('the command reads the report it is given and succeeds when every mutant was killed', async () => {
  expect(await command(mutant('Killed'))).toStrictEqual({ output: '[]\n', status: 0 });
});

test('the command prints the survivor and exits with a failure', async () => {
  expect(await command(mutant('Survived'))).toStrictEqual({
    output:
      '[{"rule":"testing-rule-6","path":"tools/lint/no-danger.ts:9:27","message":"StringLiteral mutant survived"}]\n',
    status: 1,
  });
});
