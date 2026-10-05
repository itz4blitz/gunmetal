import { readFileSync } from 'node:fs';
import { inspect } from './policy.ts';

const input: unknown = JSON.parse(readFileSync(0, 'utf8'));
const findings = inspect(process.argv[2], input);
process.stdout.write(`${JSON.stringify(findings)}\n`);
process.exitCode = findings.length === 0 ? 0 : 1;
