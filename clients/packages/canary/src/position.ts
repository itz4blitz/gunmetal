// The canary's pure function: where an item stands in a list, counted from one.
//
// The joining word is held at module level on purpose. A mutant in it is static: it is reached when the
// module loads and not from inside one test, so the mutation tool has to run every test for it. The
// canary keeps one, so that what a static mutant costs can be measured as soon as the mutation step runs.
const joiner = ' of ';

export function position(index: number, count: number): string {
  return `${index + 1}${joiner}${count}`;
}
