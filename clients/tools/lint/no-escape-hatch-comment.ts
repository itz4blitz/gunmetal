import type { Rule } from 'eslint';

// Stub: no-escape-hatch-comment.test.ts comes first and must fail on its assertions.
export function noEscapeHatchComment(): Rule.RuleModule {
  return {
    create() {
      return {};
    },
  };
}
