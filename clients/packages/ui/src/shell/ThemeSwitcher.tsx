import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { themes, type ThemeId } from './theme.ts';

export type ThemeSwitcherProps = {
  messages: ShellMessages;
  theme: ThemeId;
  onThemeChange: (theme: ThemeId) => void;
};

function themeLabel(messages: ShellMessages, theme: ThemeId): string {
  if (theme === 'dark') {
    return messages.themeDark;
  }
  if (theme === 'light') {
    return messages.themeLight;
  }
  if (theme === 'oled') {
    return messages.themeOled;
  }
  return messages.themeHighContrast;
}

export function ThemeSwitcher({ messages, theme, onThemeChange }: ThemeSwitcherProps) {
  return (
    <View id="theme-switcher" accessibilityRole="group" accessibilityLabel={messages.themeLabel}>
      <Text id="theme-label">{messages.themeLabel}</Text>
      {themes().map((id) => {
        const selected = id === theme;
        return (
          <View
            key={id}
            id={`theme-${id}`}
            accessibilityRole="button"
            accessibilityLabel={themeLabel(messages, id)}
            accessibilityState={{ selected }}
            tabIndex={0}
            onClick={() => {
              onThemeChange(id);
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                onThemeChange(id);
              }
            }}
          >
            <Text>{themeLabel(messages, id)}</Text>
          </View>
        );
      })}
    </View>
  );
}
