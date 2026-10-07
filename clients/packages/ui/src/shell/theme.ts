export type ThemeId = 'dark' | 'light' | 'oled' | 'high-contrast';

export function themes(): readonly ThemeId[] {
  return ['dark', 'light', 'oled', 'high-contrast'];
}

export function isThemeId(value: string): value is ThemeId {
  return themes().some((theme) => theme === value);
}

export function defaultTheme(): ThemeId {
  return 'dark';
}
