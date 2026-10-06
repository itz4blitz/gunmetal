import type { Rule } from 'eslint';

// The project's own form of `react/no-danger` (SEC-CLI-001, SEC-API-045, SEC-MED-057, SEC-HIS-027).
// `dangerouslySetInnerHTML` renders a string as HTML. It is refused as an attribute of any element or
// component and as a property of any object, so it cannot reach an element through a props object.
export function noDanger(): Rule.RuleModule {
  return {
    meta: {
      messages: {
        danger: 'dangerouslySetInnerHTML renders a string as HTML; render untrusted text as text (SEC-CLI-001).',
      },
    },
    create(context) {
      const report = (node: Rule.Node): void => {
        context.report({ node, messageId: 'danger' });
      };
      return {
        'JSXAttribute[name.name="dangerouslySetInnerHTML"]': report,
        'Property[key.name="dangerouslySetInnerHTML"]': report,
        'Property[key.value="dangerouslySetInnerHTML"]': report,
      };
    },
  };
}
