import { Linter } from 'eslint';
import tseslint from 'typescript-eslint';
import { expect, test } from 'vitest';
import { messageOriginCheck } from './message-origin-check.ts';

function lint(code: string): unknown {
  return new Linter({ configType: 'flat' })
    .verify(
      code,
      [
        {
          files: ['**/*.ts'],
          languageOptions: { parser: tseslint.parser },
          plugins: { gunmetal: { rules: { 'message-origin-check': messageOriginCheck() } } },
          rules: { 'gunmetal/message-origin-check': 'error' },
        },
      ],
      'module.ts',
    )
    .map(({ ruleId, message, line, column }) => ({ ruleId, message, line, column }));
}
function unchecked(line: number, column: number): unknown {
  return {
    ruleId: 'gunmetal/message-origin-check',
    message: 'A message listener checks event.origin against an exact list before it acts (SEC-API-050).',
    line,
    column,
  };
}
function hidden(line: number, column: number): unknown {
  return {
    ruleId: 'gunmetal/message-origin-check',
    message: 'A message listener is written inline, so that its origin check can be seen (SEC-API-050).',
    line,
    column,
  };
}

// Verifies: SEC-API-050
for (const [name, code, column] of [
  ['an arrow function added as a listener', "window.addEventListener('message', event => { act(event.data); });", 36],
  ['a function added as a listener', "window.addEventListener('message', function (event) { act(event.data); });", 36],
  ['an arrow function assigned to onmessage', 'window.onmessage = event => { act(event.data); };', 20],
  ['a function assigned to onmessage', 'port.onmessage = function (event) { act(event.data); };', 18],
] as const) {
  test(`${name} that never reads the origin fails the lint`, () => {
    expect(lint(code)).toStrictEqual([unchecked(1, column)]);
  });
}

// Verifies: SEC-API-050
for (const [name, code] of [
  ['a listener named elsewhere', "window.addEventListener('message', handle);"],
  ['a listener that is missing', "window.addEventListener('message');"],
  ['a handler named elsewhere and assigned to onmessage', 'window.onmessage = handle;'],
] as const) {
  test(`${name} fails the lint, because its check cannot be seen`, () => {
    expect(lint(code)).toStrictEqual([hidden(1, 1)]);
  });
}

for (const [name, code] of [
  [
    'an added arrow function',
    "window.addEventListener('message', event => { if (event.origin !== expected) return; act(event.data); });",
  ],
  [
    'an added function',
    "window.addEventListener('message', function (event) { if (!allowed.has(event.origin)) return; act(event.data); });",
  ],
  [
    'an assigned arrow function',
    'window.onmessage = event => { if (event.origin !== expected) return; act(event.data); };',
  ],
  [
    'an assigned function',
    'window.onmessage = function (event) { if (event.origin !== expected) return; act(event.data); };',
  ],
] as const) {
  test(`${name} that reads the origin passes`, () => {
    expect(lint(code)).toStrictEqual([]);
  });
}

test('listeners for other events and other ways of listening are not this rule', () => {
  expect(
    lint(
      [
        "window.addEventListener('click', event => { act(event); });",
        'window.onclick = event => { act(event); };',
        "emitter.on('message', event => { act(event); });",
        'window.addEventListener(kind, handle);',
      ].join('\n'),
    ),
  ).toStrictEqual([]);
});

test('each listener needs its own check: one that reads the origin does not excuse the next', () => {
  expect(
    lint(
      [
        'export const origin = location.origin;',
        "window.addEventListener('message', event => { if (event.origin !== expected) return; act(event.data); });",
        "window.addEventListener('message', event => { act(event.data); });",
        'port.onmessage = event => { act(event.data); };',
      ].join('\n'),
    ),
  ).toStrictEqual([unchecked(3, 36), unchecked(4, 18)]);
});
