export type Kind = { name: string; mutated: boolean };

// The canary's data table. Stub: kinds.test.ts comes first and must fail on its assertion.
export function kinds(): readonly Kind[] {
  return [];
}
