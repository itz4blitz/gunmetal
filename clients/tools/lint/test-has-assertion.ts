import type { Rule } from 'eslint';

// Testing rule 3: a test that only runs the code proves nothing. Every function handed to `test` or `it`
// calls `expect` or `assert` somewhere inside it. A test made by `test.each` is not seen by this rule.
export function testHasAssertion(): Rule.RuleModule {
  return {
    meta: {
      messages: {
        bare: 'A test asserts something: compare a whole value with expect (testing rule 3).',
      },
    },
    create(context) {
      const tests = 'CallExpression[callee.name=/^(test|it)$/] > :function';
      // One entry for each test being read, set once its body asserts.
      const asserted: boolean[] = [];
      return {
        [tests]() {
          asserted.push(false);
        },
        [`${tests}:exit`](node: Rule.Node) {
          if (asserted.pop() !== true) context.report({ node, messageId: 'bare' });
        },
        'CallExpression[callee.name=/^(expect|assert)$/], CallExpression[callee.object.name=/^(expect|assert)$/]'() {
          asserted.fill(true);
        },
      };
    },
  };
}
