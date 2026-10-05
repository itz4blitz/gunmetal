export type Finding = { rule: string; path: string; message: string };
type ObjectValue = Record<string, unknown>;

function object(value: unknown): value is ObjectValue {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function finding(path: string, message: string): Finding {
  return { rule: 'testing-rule-6', path, message };
}
// What each status that is not a kill means. A timeout is here too: a hang is not a detection.
function outcomes(): Map<string, string> {
  return new Map([
    ['Survived', 'survived'],
    ['NoCoverage', 'is covered by no test'],
    ['Ignored', 'was ignored'],
    ['Timeout', 'timed out'],
    ['RuntimeError', 'ended in a run-time error'],
    ['Pending', 'was not tested'],
  ]);
}

// Reads a StrykerJS JSON report (mutation-testing-report-schema) and returns one finding for every mutant
// that was not killed. A mutant the type checker discarded (`CompileError`) is no finding, as a mutant
// that does not compile is none for cargo-mutants. Anything that cannot be read as a report, and a
// report that holds no mutant at all, fails closed.
export function inspect(report: unknown): Finding[] {
  const unreadable = [finding('report', 'not a mutation testing report')];
  if (!object(report) || !object(report.files)) return unreadable;
  const mutants: [string, unknown][] = [];
  for (const [file, entry] of Object.entries(report.files)) {
    if (!object(entry) || !Array.isArray(entry.mutants)) return unreadable;
    for (const mutant of entry.mutants) mutants.push([file, mutant]);
  }
  if (mutants.length === 0) return [finding('report', 'the report holds no mutant')];
  const findings: Finding[] = [];
  for (const [file, mutant] of mutants) {
    const start = object(mutant) && object(mutant.location) ? mutant.location.start : undefined;
    if (
      !object(mutant) ||
      typeof mutant.status !== 'string' ||
      typeof mutant.mutatorName !== 'string' ||
      !object(start) ||
      typeof start.line !== 'number' ||
      typeof start.column !== 'number'
    ) {
      return unreadable;
    }
    if (mutant.status === 'Killed' || mutant.status === 'CompileError') continue;
    const outcome = outcomes().get(mutant.status) ?? `has the unknown status ${mutant.status}`;
    findings.push(finding(`${file}:${start.line}:${start.column}`, `${mutant.mutatorName} mutant ${outcome}`));
  }
  return findings;
}

// The command line's whole answer for the report at `path`: what to print and the status to exit with.
export function verdict(path: string | undefined, read: (path: string) => string): { output: string; status: number } {
  let findings: Finding[];
  try {
    findings = inspect(JSON.parse(read(String(path))));
  } catch {
    findings = [finding('report', 'the mutation testing report could not be read')];
  }
  return { output: `${JSON.stringify(findings)}\n`, status: findings.length === 0 ? 0 : 1 };
}
