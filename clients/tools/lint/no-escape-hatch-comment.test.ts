import { Linter } from 'eslint';
import tseslint from 'typescript-eslint';
import { expect, test } from 'vitest';
import { noEscapeHatchComment } from './no-escape-hatch-comment.ts';

function lint(code: string): unknown {
  return new Linter({ configType: 'flat' })
    .verify(
      code,
      [
        {
          files: ['**/*.ts'],
          languageOptions: { parser: tseslint.parser },
          plugins: { gunmetal: { rules: { 'no-escape-hatch-comment': noEscapeHatchComment() } } },
          rules: { 'gunmetal/no-escape-hatch-comment': 'error' },
        },
      ],
      'module.ts',
    )
    .map(({ ruleId, message, line, column }) => ({ ruleId, message, line, column }));
}
const message = 'This comment switches a check off; no file or line is excused from the gate (testing rule 2).';

for (const comment of [
  '/* v8 ignore next */',
  '/* v8 ignore start */',
  '/* c8 ignore next 3 */',
  '/* istanbul ignore next */',
  '// istanbul ignore file',
  '/* node:coverage disable */',
  '// Stryker disable all',
  '// Stryker disable next-line ArithmeticOperator: the sum is checked elsewhere',
  '//stryker DISABLE all',
] as const) {
  test(`the comment ${comment} fails the lint`, () => {
    expect(lint(`export const one = 1;\n${comment}\nexport const two = 2;`)).toStrictEqual([
      { ruleId: 'gunmetal/no-escape-hatch-comment', message, line: 2, column: 1 },
    ]);
  });
}

test('every such comment in a file is reported', () => {
  expect(lint('/* v8 ignore next */\nexport const one = 1; // Stryker disable all')).toStrictEqual([
    { ruleId: 'gunmetal/no-escape-hatch-comment', message, line: 1, column: 1 },
    { ruleId: 'gunmetal/no-escape-hatch-comment', message, line: 2, column: 23 },
  ]);
});

test('a comment that only mentions such a switch, or explains the code, passes', () => {
  expect(
    lint(
      '// The v8 ignore comment is refused here.\n// Verifies: SEC-CLI-001\n/* Stryker is the mutation tool. */\nexport const one = 1;',
    ),
  ).toStrictEqual([]);
});
