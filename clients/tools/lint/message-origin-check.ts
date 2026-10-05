import type { Rule } from 'eslint';

type Typed = { type: string } | undefined;

function inline(handler: Typed): boolean {
  return handler?.type === 'FunctionExpression' || handler?.type === 'ArrowFunctionExpression';
}

// SEC-API-050: a `message` listener receives data from any window that can reach this one, so it reads
// `event.origin` before it acts. The listener is written inline, where that check can be seen. Whether
// the origin is compared with an exact list is for the listener's own tests; this finds the listener
// that never reads it.
export function messageOriginCheck(): Rule.RuleModule {
  return {
    meta: {
      messages: {
        unchecked: 'A message listener checks event.origin against an exact list before it acts (SEC-API-050).',
        hidden: 'A message listener is written inline, so that its origin check can be seen (SEC-API-050).',
      },
    },
    create(context) {
      const added = 'CallExpression[callee.property.name="addEventListener"][arguments.0.value="message"]';
      const assigned = 'AssignmentExpression[left.property.name="onmessage"]';
      const handlers = `${added} > :function:nth-child(2), ${assigned} > FunctionExpression.right, ${assigned} > ArrowFunctionExpression.right`;
      // One entry for each listener being read, set once its body reads an origin.
      const read: boolean[] = [];
      return {
        [handlers]() {
          read.push(false);
        },
        [`${handlers}:exit`](node: Rule.Node) {
          if (read.pop() !== true) context.report({ node, messageId: 'unchecked' });
        },
        'MemberExpression[property.name="origin"]'() {
          read.fill(true);
        },
        [added](node: Rule.Node & { arguments: Typed[] }) {
          if (!inline(node.arguments[1])) context.report({ node, messageId: 'hidden' });
        },
        [assigned](node: Rule.Node & { right: Typed }) {
          if (!inline(node.right)) context.report({ node, messageId: 'hidden' });
        },
      };
    },
  };
}
