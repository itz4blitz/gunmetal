export type Finding = { rule: string; path: string; message: string };

// Stub: the tests in mutation-report.test.ts come first and must fail on their assertions.
export function inspect(_report: unknown): Finding[] {
  return [];
}

export function verdict(_path: string | undefined, _read: (path: string) => string): { output: string; status: number } {
  return { output: '', status: 0 };
}
