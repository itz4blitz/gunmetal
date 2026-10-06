export type Kind = { name: string; mutated: boolean };

// The canary's data table: the four kinds of client code and whether each is in the mutated set. A table
// is returned by a function and never held at module level, so each mutant in it is covered per test.
export function kinds(): readonly Kind[] {
  return [
    { name: 'pure function', mutated: true },
    { name: 'component', mutated: true },
    { name: 'data table', mutated: true },
    { name: 'browser adapter', mutated: false },
  ];
}
