import tseslint from 'typescript-eslint';
import {
  jsxNoScriptUrl,
  messageOriginCheck,
  noDanger,
  noEscapeHatchComment,
  testHasAssertion,
} from './tools/lint/index.ts';

// The lint of the client workspace. Every rule is an error, and the gate allows no warning either.
// The rules written for this project are in tools/lint, where they are tested like any other code.

const html = 'It writes a string as HTML; render untrusted text as text (SEC-CLI-001).';

export default [
  // The one path the lint skips: planted-fault fixtures, which are faulty on purpose and are not source.
  { name: 'gunmetal/skipped', ignores: ['tools/fixtures/**'] },
  ...tseslint.configs.strict,
  {
    name: 'gunmetal/everywhere',
    files: ['**/*.{js,ts,tsx}'],
    // A comment cannot switch a rule off: it has no effect, and ESLint reports it.
    linterOptions: { noInlineConfig: true, reportUnusedDisableDirectives: 'error' },
    plugins: {
      gunmetal: {
        rules: {
          'jsx-no-script-url': jsxNoScriptUrl(),
          'message-origin-check': messageOriginCheck(),
          'no-danger': noDanger(),
          'no-escape-hatch-comment': noEscapeHatchComment(),
          'test-has-assertion': testHasAssertion(),
        },
      },
    },
    rules: {
      // HTML and code sinks (SEC-CLI-001, SEC-API-045, SEC-MED-057, SEC-HIS-027, SEC-TM-036).
      'gunmetal/no-danger': 'error',
      'gunmetal/jsx-no-script-url': 'error',
      'no-eval': 'error',
      'no-implied-eval': 'error',
      'no-new-func': 'error',
      'no-script-url': 'error',
      'no-restricted-properties': [
        'error',
        { property: 'innerHTML', message: html },
        { property: 'outerHTML', message: html },
        { property: 'insertAdjacentHTML', message: html },
        { object: 'document', property: 'write', message: html },
        { object: 'document', property: 'writeln', message: html },
      ],
      // SEC-API-050.
      'gunmetal/message-origin-check': 'error',
      // The escape hatches of the testing rules (ground rules 2 to 4 of the client plan).
      'gunmetal/no-escape-hatch-comment': 'error',
      '@typescript-eslint/ban-ts-comment': [
        'error',
        {
          'ts-check': false,
          'ts-expect-error': 'allow-with-description',
          'ts-ignore': true,
          'ts-nocheck': true,
          minimumDescriptionLength: 10,
        },
      ],
      'no-restricted-syntax': [
        'error',
        {
          selector:
            'MemberExpression[object.name=/^(test|it|describe|suite)$/][property.name=/^(skip|only|todo|fails|skipIf|runIf)$/]',
          message: 'A test is never skipped, narrowed, left to do or expected to fail (testing rule 2).',
        },
        {
          selector: 'MemberExpression[property.name=/^(toBeTruthy|toBeFalsy|toBeDefined)$/]',
          message: 'Compare the whole value: this matcher accepts almost anything (testing rule 3).',
        },
        {
          selector: 'MemberExpression[property.name=/Snapshot$/]',
          message: 'A snapshot is written by the code it checks; compare with a literal (testing rule 4).',
        },
      ],
    },
  },
  {
    name: 'gunmetal/tests',
    files: ['**/*.test.{ts,tsx}'],
    rules: { 'gunmetal/test-has-assertion': 'error' },
  },
];
