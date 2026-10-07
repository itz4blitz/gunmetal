import { expect, test } from 'vitest';
import { defaultTheme, isThemeId, themes } from './theme.ts';

test('the four CLI-141 themes are listed and dark is the default', () => {
  expect(themes()).toStrictEqual(['dark', 'light', 'oled', 'high-contrast']);
  expect(defaultTheme()).toStrictEqual('dark');
  expect(isThemeId('dark')).toStrictEqual(true);
  expect(isThemeId('light')).toStrictEqual(true);
  expect(isThemeId('oled')).toStrictEqual(true);
  expect(isThemeId('high-contrast')).toStrictEqual(true);
  expect(isThemeId('system')).toStrictEqual(false);
  expect(isThemeId('')).toStrictEqual(false);
});
