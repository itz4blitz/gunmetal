import type { Rule } from 'eslint';

// Stub: test-has-assertion.test.ts comes first and must fail on its assertions.
export function testHasAssertion(): Rule.RuleModule {
  return {
    create() {
      return {};
    },
  };
}
