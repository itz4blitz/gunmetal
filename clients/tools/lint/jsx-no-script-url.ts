import type { Rule } from 'eslint';

type Value = { type: string; value?: unknown; expression?: Value; quasis?: { value: { cooked?: unknown } }[] };

// What a browser's URL parser does before it reads the scheme: tabs and newlines are removed wherever
// they stand, and leading control characters and spaces are dropped. So `java\nscript:` is a script URL.
function scriptUrl(text: unknown): boolean {
  if (typeof text !== 'string') return false;
  // Leading characters up to and including the space are the controls and the space the parser drops.
  const address = text.replace(/[\t\n\r]/g, '').replace(/^[\u0000- ]+/, '');
  return /^javascript:/i.test(address);
}

// The project's own form of `react/jsx-no-script-url` (SEC-API-045, SEC-MED-057): an attribute whose
// value is a literal, or a template that starts with one, may not be a `javascript:` URL.
export function jsxNoScriptUrl(): Rule.RuleModule {
  return {
    meta: {
      messages: {
        script: 'A javascript: URL runs its text as code; it is never an attribute value (SEC-API-045).',
      },
    },
    create(context) {
      return {
        JSXAttribute(node: Rule.Node & { value: Value | null }) {
          const value = node.value?.type === 'JSXExpressionContainer' ? node.value.expression : node.value;
          const text = value?.type === 'TemplateLiteral' ? value.quasis?.[0]?.value.cooked : value?.value;
          if (scriptUrl(text)) context.report({ node, messageId: 'script' });
        },
      };
    },
  };
}
