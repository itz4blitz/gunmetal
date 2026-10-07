import { afterEach, expect, test } from 'vitest';
import {
  SYSTEM_THEME_QUERY,
  defaultResolvedTheme,
  defaultSystemThemeQuery,
  defaultTheme,
  isResolvedTheme,
  isThemeId,
  noSettingsStore,
  parseThemeChoice,
  resolveTheme,
  resolvedThemes,
  serializeThemeChoice,
  themes,
  type SystemThemeQuery,
} from './theme.ts';

test('the five CLI-141 choices are listed system-first and system is the default', () => {
  expect(themes()).toStrictEqual(['system', 'dark', 'light', 'oled', 'high-contrast']);
  expect(resolvedThemes()).toStrictEqual(['dark', 'light', 'oled', 'high-contrast']);
  expect(defaultTheme()).toStrictEqual('system');
  // Dark is the theme designed first: the fallback when the OS states no
  // preference or none can be asked (design-language §4).
  expect(defaultResolvedTheme()).toStrictEqual('dark');
});

test('choice guards accept every theme name and nothing else', () => {
  expect(isThemeId('system')).toStrictEqual(true);
  expect(isThemeId('dark')).toStrictEqual(true);
  expect(isThemeId('light')).toStrictEqual(true);
  expect(isThemeId('oled')).toStrictEqual(true);
  expect(isThemeId('high-contrast')).toStrictEqual(true);
  expect(isThemeId('sepia')).toStrictEqual(false);
  expect(isThemeId('')).toStrictEqual(false);
  expect(isResolvedTheme('dark')).toStrictEqual(true);
  expect(isResolvedTheme('oled')).toStrictEqual(true);
  expect(isResolvedTheme('system')).toStrictEqual(false);
  expect(isResolvedTheme('')).toStrictEqual(false);
});

test('system resolves to the OS preference and every named theme resolves to itself', () => {
  expect(resolveTheme('system', true)).toStrictEqual('dark');
  expect(resolveTheme('system', false)).toStrictEqual('light');
  expect(resolveTheme('dark', false)).toStrictEqual('dark');
  expect(resolveTheme('light', true)).toStrictEqual('light');
  expect(resolveTheme('oled', true)).toStrictEqual('oled');
  expect(resolveTheme('high-contrast', false)).toStrictEqual('high-contrast');
});

test('the system query is prefers-color-scheme dark', () => {
  expect(SYSTEM_THEME_QUERY).toStrictEqual('(prefers-color-scheme: dark)');
});

test('the default query asks the runtime and answers null when matchMedia is missing', () => {
  // jsdom has no globalThis.matchMedia: exactly the unsupported case.
  expect(defaultSystemThemeQuery(SYSTEM_THEME_QUERY)).toStrictEqual(null);
  const seen: string[] = [];
  const sentinel = { matches: true } as unknown as SystemThemeQuery;
  const original = globalThis.matchMedia;
  globalThis.matchMedia = ((query: string) => {
    seen.push(query);
    return sentinel;
  }) as unknown as typeof globalThis.matchMedia;
  try {
    expect(defaultSystemThemeQuery(SYSTEM_THEME_QUERY)).toStrictEqual(sentinel);
    expect(seen).toStrictEqual(['(prefers-color-scheme: dark)']);
  } finally {
    globalThis.matchMedia = original;
  }
});

test('a stored choice round-trips through one JSON object', () => {
  expect(serializeThemeChoice('oled')).toStrictEqual('{"theme":"oled"}');
  expect(serializeThemeChoice('system')).toStrictEqual('{"theme":"system"}');
  expect(parseThemeChoice(serializeThemeChoice('high-contrast'))).toStrictEqual('high-contrast');
  expect(parseThemeChoice(serializeThemeChoice('light'))).toStrictEqual('light');
});

test('anything malformed in storage reads as the default choice', () => {
  expect(parseThemeChoice(null)).toStrictEqual('system');
  // Not JSON.
  expect(parseThemeChoice('')).toStrictEqual('system');
  expect(parseThemeChoice('{theme:dark}')).toStrictEqual('system');
  // JSON, but not an object.
  expect(parseThemeChoice('42')).toStrictEqual('system');
  expect(parseThemeChoice('"dark"')).toStrictEqual('system');
  expect(parseThemeChoice('null')).toStrictEqual('system');
  // An object without a theme name in it.
  expect(parseThemeChoice('{}')).toStrictEqual('system');
  expect(parseThemeChoice('{"theme":42}')).toStrictEqual('system');
  expect(parseThemeChoice('{"theme":"sepia"}')).toStrictEqual('system');
  expect(parseThemeChoice('{"other":"dark"}')).toStrictEqual('system');
  // Future keys are tolerated.
  expect(parseThemeChoice('{"theme":"dark","volume":0.5}')).toStrictEqual('dark');
});

test('the store of a shell nobody wired one for remembers nothing', () => {
  const store = noSettingsStore();
  expect(store.read()).toStrictEqual(null);
  expect(store.write('{"theme":"dark"}')).toStrictEqual(undefined);
  expect(store.read()).toStrictEqual(null);
});

afterEach(() => {
  delete (globalThis as { matchMedia?: unknown }).matchMedia;
});
