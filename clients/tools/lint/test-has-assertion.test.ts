import { Linter } from 'eslint';
import tseslint from 'typescript-eslint';
import { expect, test } from 'vitest';
import { testHasAssertion } from './test-has-assertion.ts';

function lint(code: string): unknown {
  return new Linter({ configType: 'flat' })
    .verify(
      code,
      [
        {
          files: ['**/*.ts'],
          languageOptions: { parser: tseslint.parser },
          plugins: { gunmetal: { rules: { 'test-has-assertion': testHasAssertion() } } },
          rules: { 'gunmetal/test-has-assertion': 'error' },
        },
      ],
      'module.test.ts',
    )
    .map(({ ruleId, message, line, column }) => ({ ruleId, message, line, column }));
}
function bare(line: number, column: number): unknown {
  return {
    ruleId: 'gunmetal/test-has-assertion',
    message: 'A test asserts something: compare a whole value with expect (testing rule 3).',
    line,
    column,
  };
}

for (const [name, code, column] of [
  ['a test that only runs the code', "test('it runs', () => { position(0, 12); });", 17],
  ['an it block that only runs the code', "it('runs', async function () { await position(0, 12); });", 12],
  ['a test that leaves the comparing to a helper', "test('it runs', () => { check(position(0, 12)); });", 17],
] as const) {
  test(`${name} fails the lint`, () => {
    expect(lint(code)).toStrictEqual([bare(1, column)]);
  });
}

for (const [name, code] of [
  ['an expectation', "test('it counts', () => { expect(position(0, 12)).toStrictEqual('1 of 12'); });"],
  ['a soft expectation', "test('it counts', () => { expect.soft(position(0, 12)).toStrictEqual('1 of 12'); });"],
  ['an assertion from node:assert', "it('counts', () => { assert.deepEqual(position(0, 12), '1 of 12'); });"],
  ['a bare assert call', "it('counts', () => { assert(position(0, 12) === '1 of 12'); });"],
  [
    'an expectation inside a callback of the test',
    "test('it counts', () => { rows.forEach(row => { expect(row).toStrictEqual(1); }); });",
  ],
] as const) {
  test(`a test with ${name} passes`, () => {
    expect(lint(code)).toStrictEqual([]);
  });
}

test('each test needs its own assertion, and code outside a test is not a test', () => {
  expect(
    lint(
      [
        'expect.extend(matchers);',
        "test('it counts', () => { expect(position(0, 12)).toStrictEqual('1 of 12'); });",
        "test('it runs', () => { position(0, 12); });",
        'describe(name, () => { position(0, 12); });',
      ].join('\n'),
    ),
  ).toStrictEqual([bare(3, 17)]);
});
