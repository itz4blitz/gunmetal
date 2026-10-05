import type { Rule } from 'eslint';

// Stub: no-danger.test.ts comes first and must fail on its assertions.
export function noDanger(): Rule.RuleModule {
  return {
    create() {
      return {};
    },
  };
}
