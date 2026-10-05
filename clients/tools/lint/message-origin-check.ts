import type { Rule } from 'eslint';

// Stub: message-origin-check.test.ts comes first and must fail on its assertions.
export function messageOriginCheck(): Rule.RuleModule {
  return {
    create() {
      return {};
    },
  };
}
