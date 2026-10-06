declare module 'react-native-web' {
  import type { KeyboardEventHandler, MouseEventHandler, ReactNode } from 'react';

  export type AccessibilityState = {
    selected?: boolean;
  };

  export type ViewProps = {
    children?: ReactNode;
    id?: string;
    accessibilityRole?: string;
    accessibilityLabel?: string;
    accessibilityState?: AccessibilityState;
    tabIndex?: number;
    onClick?: MouseEventHandler<HTMLElement>;
    onKeyDown?: KeyboardEventHandler<HTMLElement>;
    dataSet?: Record<string, string>;
  };

  export type TextProps = {
    children?: ReactNode;
    id?: string;
    accessibilityRole?: string;
    accessibilityLabel?: string;
  };

  export function View(props: ViewProps): ReactNode;
  export function Text(props: TextProps): ReactNode;
}
