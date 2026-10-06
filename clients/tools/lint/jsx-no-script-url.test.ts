import { Linter } from 'eslint';
import tseslint from 'typescript-eslint';
import { expect, test } from 'vitest';
import { jsxNoScriptUrl } from './jsx-no-script-url.ts';

function lint(code: string): unknown {
  return new Linter({ configType: 'flat' })
    .verify(
      code,
      [
        {
          files: ['**/*.tsx'],
          languageOptions: { parser: tseslint.parser },
          plugins: { gunmetal: { rules: { 'jsx-no-script-url': jsxNoScriptUrl() } } },
          rules: { 'gunmetal/jsx-no-script-url': 'error' },
        },
      ],
      'screen.tsx',
    )
    .map(({ ruleId, message, line, column }) => ({ ruleId, message, line, column }));
}
const refused = [
  {
    ruleId: 'gunmetal/jsx-no-script-url',
    message: 'A javascript: URL runs its text as code; it is never an attribute value (SEC-API-045).',
    line: 1,
    column: 24,
  },
];

// Verifies: SEC-API-045, SEC-MED-057
// Each value is one the browser's URL parser reads as the javascript: scheme.
for (const [name, attribute] of [
  ['a plain script URL', 'href="javascript:alert(1)"'],
  ['a script URL in mixed case', 'href="JaVaScRiPt:alert(1)"'],
  ['a script URL after spaces', 'href="  javascript:alert(1)"'],
  ['a script URL after a control character', 'href="\u0001javascript:alert(1)"'],
  ['a script URL broken by a tab', 'href="java\tscript:alert(1)"'],
  ['a script URL in an expression', "href={'javascript:alert(1)'}"],
  ['a script URL broken by an escaped newline', "href={'java\\nscript:alert(1)'}"],
  ['a script URL broken by an escaped carriage return', "href={'java\\rscript:alert(1)'}"],
  ['a script URL at the start of a template', 'href={`javascript:${code}`}'],
] as const) {
  test(`${name} as an attribute value fails the lint`, () => {
    expect(lint(`export const view = <a ${attribute} />;`)).toStrictEqual(refused);
  });
}

for (const [name, attribute] of [
  ['an https URL that only mentions the scheme', 'href="https://example.test/javascript:"'],
  ['the word without its colon', 'href="javascript"'],
  ['a scheme broken by a space, which the browser does not join', 'href="java script:alert(1)"'],
  ['a value that is not a literal', 'href={address}'],
  ['an attribute without a value', 'hidden'],
  ['a number', 'tabIndex={0}'],
  ['a template that starts with an expression', 'href={`${base}javascript:`}'],
] as const) {
  test(`${name} passes`, () => {
    expect(lint(`export const view = <a ${attribute} />;`)).toStrictEqual([]);
  });
}
