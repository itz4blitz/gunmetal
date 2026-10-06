declare module 'react-native-web' {
  import type { ReactNode } from 'react';

  export type ViewProps = {
    children?: ReactNode;
    id?: string;
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
