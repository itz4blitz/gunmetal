import type { AST, Rule } from 'eslint';

// How a comment that switches the coverage or the mutation tool off begins, in lower case.
function hatches(): readonly string[] {
  return ['v8 ignore', 'c8 ignore', 'istanbul ignore', 'node:coverage', 'stryker disable'];
}

// Testing rule 2: no file or line is excused from coverage or mutation by a comment. ESLint's own
// switches are refused by the `noInlineConfig` setting, and TypeScript's by typescript-eslint.
export function noEscapeHatchComment(): Rule.RuleModule {
  return {
    meta: {
      messages: {
        hatch: 'This comment switches a check off; no file or line is excused from the gate (testing rule 2).',
      },
    },
    create(context) {
      return {
        Program() {
          for (const comment of context.sourceCode.getAllComments()) {
            const text = comment.value.trim().toLowerCase();
            if (hatches().some((hatch) => text.startsWith(hatch))) {
              // The parser gives every comment its place in the file.
              context.report({ loc: comment.loc as AST.SourceLocation, messageId: 'hatch' });
            }
          }
        },
      };
    },
  };
}
