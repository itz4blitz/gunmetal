import type { Rule } from 'eslint';

// Stub: jsx-no-script-url.test.ts comes first and must fail on its assertions.
export function jsxNoScriptUrl(): Rule.RuleModule {
  return {
    create() {
      return {};
    },
  };
}
