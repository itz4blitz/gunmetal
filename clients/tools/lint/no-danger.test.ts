import { Linter } from 'eslint';
import tseslint from 'typescript-eslint';
import { expect, test } from 'vitest';
import { noDanger } from './no-danger.ts';

function lint(code: string): unknown {
  return new Linter({ configType: 'flat' })
    .verify(
      code,
      [
        {
          files: ['**/*.tsx'],
          languageOptions: { parser: tseslint.parser },
          plugins: { gunmetal: { rules: { 'no-danger': noDanger() } } },
          rules: { 'gunmetal/no-danger': 'error' },
        },
      ],
      'screen.tsx',
    )
    .map(({ ruleId, message, line, column }) => ({ ruleId, message, line, column }));
}
const message = 'dangerouslySetInnerHTML renders a string as HTML; render untrusted text as text (SEC-CLI-001).';

// Verifies: SEC-CLI-001, SEC-API-045, SEC-MED-057, SEC-HIS-027, SEC-TM-036
for (const [name, code, column] of [
  ['an attribute of a DOM element', 'export const view = <div dangerouslySetInnerHTML={{ __html: title }} />;', 26],
  ['an attribute of a component', 'export const view = <Panel dangerouslySetInnerHTML={{ __html: title }} />;', 28],
  [
    'a property of a props object',
    "export const view = createElement('div', { dangerouslySetInnerHTML: { __html: title } });",
    44,
  ],
  ['a quoted property of a props object', "export const props = { 'dangerouslySetInnerHTML': { __html: title } };", 24],
] as const) {
  test(`dangerouslySetInnerHTML as ${name} fails the lint`, () => {
    expect(lint(code)).toStrictEqual([{ ruleId: 'gunmetal/no-danger', message, line: 1, column }]);
  });
}

test('rendering the same string as text passes', () => {
  expect(
    lint('export const view = <div title={title}>{title}</div>;\nexport const props = { innerText: title };'),
  ).toStrictEqual([]);
});
