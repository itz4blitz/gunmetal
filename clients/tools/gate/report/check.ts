import { readFileSync } from 'node:fs';
import { verdict } from './mutation-report.ts';

// usage: node tools/gate/report/check.ts <the JSON report StrykerJS wrote>
// Prints the findings as JSON and exits with 1 unless every mutant in the report was killed.
const { output, status } = verdict(process.argv[2], (path) => String(readFileSync(path)));
process.stdout.write(output);
process.exitCode = status;
