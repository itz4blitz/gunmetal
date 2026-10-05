import { expect, test } from 'vitest';
import { inspect, verdict } from './mutation-report.ts';

// A literal StrykerJS JSON report (mutation-testing-report-schema) holding one mutant per given status.
function report(...statuses: string[]): unknown {
  return {
    schemaVersion: '1.0',
    thresholds: { high: 100, low: 100 },
    files: {
      'packages/canary/src/position.ts': {
        language: 'typescript',
        source: 'export function position() {}',
        mutants: statuses.map((status, index) => ({
          id: String(index),
          mutatorName: 'ArithmeticOperator',
          replacement: 'index - 1',
          location: { start: { line: 4 + index, column: 13 }, end: { line: 4 + index, column: 22 } },
          status,
        })),
      },
    },
  };
}
function finding(path: string, message: string): unknown {
  return { rule: 'testing-rule-6', path, message };
}
const unreadable = [finding('report', 'not a mutation testing report')];

test('a report in which every mutant was killed passes', () => {
  expect(inspect(report('Killed', 'Killed'))).toStrictEqual([]);
});

test('a mutant the type checker discarded is not a survivor', () => {
  expect(inspect(report('Killed', 'CompileError'))).toStrictEqual([]);
});

for (const [status, message] of [
  ['Survived', 'ArithmeticOperator mutant survived'],
  ['NoCoverage', 'ArithmeticOperator mutant is covered by no test'],
  ['Ignored', 'ArithmeticOperator mutant was ignored'],
  ['Timeout', 'ArithmeticOperator mutant timed out'],
  ['RuntimeError', 'ArithmeticOperator mutant ended in a run-time error'],
  ['Pending', 'ArithmeticOperator mutant was not tested'],
  ['Detected', 'ArithmeticOperator mutant has the unknown status Detected'],
] as const) {
  test(`a report holding one ${status} mutant fails and names it`, () => {
    expect(inspect(report('Killed', status, 'Killed'))).toStrictEqual([
      finding('packages/canary/src/position.ts:5:13', message),
    ]);
  });
}

test('every mutant that was not killed is reported, in the order of the report', () => {
  expect(inspect(report('Survived', 'Killed', 'Timeout'))).toStrictEqual([
    finding('packages/canary/src/position.ts:4:13', 'ArithmeticOperator mutant survived'),
    finding('packages/canary/src/position.ts:6:13', 'ArithmeticOperator mutant timed out'),
  ]);
});

test('a report that holds no mutant fails, because a run that tested nothing proves nothing', () => {
  expect(inspect(report())).toStrictEqual([finding('report', 'the report holds no mutant')]);
  expect(inspect({ files: {} })).toStrictEqual([finding('report', 'the report holds no mutant')]);
});

const killed = {
  mutatorName: 'ArithmeticOperator',
  location: { start: { line: 4, column: 13 } },
  status: 'Killed',
};
for (const [name, value] of [
  ['nothing', null],
  ['a list', []],
  ['text', 'Killed'],
  ['no files', { schemaVersion: '1.0' }],
  ['files that are a list', { files: [] }],
  ['files that are text', { files: 'position.ts' }],
  ['a file that is nothing', { files: { 'position.ts': null } }],
  ['a file whose mutants are missing', { files: { 'position.ts': {} } }],
  ['a file whose mutants are an object', { files: { 'position.ts': { mutants: { 0: killed } } } }],
  ['a mutant that is nothing', { files: { 'position.ts': { mutants: [null] } } }],
  ['a mutant that is text', { files: { 'position.ts': { mutants: ['Killed'] } } }],
  ['a mutant without a status', { files: { 'position.ts': { mutants: [{ ...killed, status: undefined }] } } }],
  ['a mutant whose status is not text', { files: { 'position.ts': { mutants: [{ ...killed, status: 1 }] } } }],
  ['a mutant without a mutator', { files: { 'position.ts': { mutants: [{ ...killed, mutatorName: undefined }] } } }],
  ['a mutant without a location', { files: { 'position.ts': { mutants: [{ ...killed, location: undefined }] } } }],
  ['a mutant whose location is a list', { files: { 'position.ts': { mutants: [{ ...killed, location: [] }] } } }],
  ['a mutant without a start', { files: { 'position.ts': { mutants: [{ ...killed, location: {} }] } } }],
  [
    'a mutant whose start is nothing',
    { files: { 'position.ts': { mutants: [{ ...killed, location: { start: null } }] } } },
  ],
  [
    'a mutant without a line',
    { files: { 'position.ts': { mutants: [{ ...killed, location: { start: { column: 13 } } }] } } },
  ],
  [
    'a mutant whose line is text',
    { files: { 'position.ts': { mutants: [{ ...killed, location: { start: { line: '4', column: 13 } } }] } } },
  ],
  [
    'a mutant without a column',
    { files: { 'position.ts': { mutants: [{ ...killed, location: { start: { line: 4 } } }] } } },
  ],
  [
    'a mutant whose column is text',
    { files: { 'position.ts': { mutants: [{ ...killed, location: { start: { line: 4, column: '13' } } }] } } },
  ],
] as const) {
  test(`a report that is ${name} fails closed`, () => {
    expect(inspect(value)).toStrictEqual(unreadable);
  });
}

test('one unreadable mutant fails the whole report, whatever the others say', () => {
  expect(inspect({ files: { 'position.ts': { mutants: [{ ...killed, status: 'Survived' }, null] } } })).toStrictEqual(
    unreadable,
  );
});

// The command line's whole answer: what it prints and the status it exits with.
const reports = new Map<string, string>([
  ['killed.json', JSON.stringify(report('Killed'))],
  ['survived.json', JSON.stringify(report('Survived'))],
  ['broken.json', '{"files":'],
]);
function read(path: string): string {
  const text = reports.get(path);
  if (text === undefined) throw new Error(`ENOENT: ${path}`);
  return text;
}

test('the command prints no finding and succeeds when every mutant was killed', () => {
  expect(verdict('killed.json', read)).toStrictEqual({ output: '[]\n', status: 0 });
});

test('the command prints each finding as JSON and fails when a mutant survived', () => {
  expect(verdict('survived.json', read)).toStrictEqual({
    output:
      '[{"rule":"testing-rule-6","path":"packages/canary/src/position.ts:4:13","message":"ArithmeticOperator mutant survived"}]\n',
    status: 1,
  });
});

for (const [name, path] of [
  ['no report path', undefined],
  ['a report that does not exist', 'absent.json'],
  ['a report that is not JSON', 'broken.json'],
] as const) {
  test(`the command fails closed on ${name}`, () => {
    expect(verdict(path, read)).toStrictEqual({
      output: '[{"rule":"testing-rule-6","path":"report","message":"the mutation testing report could not be read"}]\n',
      status: 1,
    });
  });
}
