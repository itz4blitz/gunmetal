// One declaration for the React Native primitives the client uses, in place
// of a types package (client plan CP-006: no @types/react-native-web, no
// react-native in the lockfile). Browser adapters and the DOM stay out.
declare module 'react-native-web' {
  import type { KeyboardEventHandler, MouseEventHandler, ReactNode } from 'react';

  export type AccessibilityState = {
    selected?: boolean | undefined;
    disabled?: boolean | undefined;
  };

  export type AccessibilityValue = {
    min?: number | undefined;
    max?: number | undefined;
    now?: number | undefined;
    text?: string | undefined;
  };

  export type ViewProps = {
    children?: ReactNode | undefined;
    id?: string | undefined;
    accessibilityRole?: string | undefined;
    accessibilityLabel?: string | undefined;
    accessibilityState?: AccessibilityState | undefined;
    accessibilityElementsHidden?: boolean | undefined;
    accessibilityValue?: AccessibilityValue | undefined;
    tabIndex?: number | undefined;
    onClick?: MouseEventHandler<HTMLElement> | undefined;
    onContextMenu?: MouseEventHandler<HTMLElement> | undefined;
    onKeyDown?: KeyboardEventHandler<HTMLElement> | undefined;
    dataSet?: Record<string, string> | undefined;
    style?: Record<string, string | number> | undefined;
  };

  export type TextProps = {
    children?: ReactNode | undefined;
    id?: string | undefined;
    accessibilityRole?: string | undefined;
    accessibilityLabel?: string | undefined;
    accessibilityState?: AccessibilityState | undefined;
    dataSet?: Record<string, string> | undefined;
    style?: Record<string, string | number> | undefined;
  };

  export function View(props: ViewProps): ReactNode;
  export function Text(props: TextProps): ReactNode;
}
