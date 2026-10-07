/**
 * The theme choices of the shell and their resolution (design-language §4,
 * CLI-141). A choice is what the person picks and what is stored; the shell
 * paints with a resolved theme — `system` becomes dark or light by asking
 * the operating system through `matchMedia('(prefers-color-scheme: dark)')`,
 * falling back to dark when the system states no preference or cannot be
 * asked. Pure naming and parsing: the Shell applies the result and the
 * composition root decides where the choice is kept.
 *
 * A stored choice is never trusted: anything that does not read as a theme
 * name falls back to the default, mirroring the pane-width parsing.
 */
export type ResolvedTheme = 'dark' | 'light' | 'oled' | 'high-contrast';

export type ThemeId = 'system' | ResolvedTheme;

/** The choices as Settings lists them: System first, then the four themes. */
export function themes(): readonly ThemeId[] {
  return ['system', 'dark', 'light', 'oled', 'high-contrast'];
}

/** The themes with a paint of their own; the values `data-theme` may take. */
export function resolvedThemes(): readonly ResolvedTheme[] {
  return ['dark', 'light', 'oled', 'high-contrast'];
}

export function isThemeId(value: string): value is ThemeId {
  return themes().some((theme) => theme === value);
}

export function isResolvedTheme(value: string): value is ResolvedTheme {
  return resolvedThemes().some((theme) => theme === value);
}

/** The choice a fresh shell makes: follow the operating system. */
export function defaultTheme(): ThemeId {
  return 'system';
}

/** What `system` means when no OS preference is available: dark. */
export function defaultResolvedTheme(): ResolvedTheme {
  return 'dark';
}

export function resolveTheme(choice: ThemeId, systemPrefersDark: boolean): ResolvedTheme {
  if (choice === 'system') {
    return systemPrefersDark ? 'dark' : 'light';
  }
  return choice;
}

/** The one media query the shell asks about the operating system. */
export const SYSTEM_THEME_QUERY = '(prefers-color-scheme: dark)';

/**
 * The slice of MediaQueryList the shell needs. A structural stand-in so
 * tests can inject a fake, and so the shell never touches `window` directly.
 */
export type SystemThemeQuery = {
  matches: boolean;
  addEventListener(type: 'change', listener: () => void): void;
  removeEventListener(type: 'change', listener: () => void): void;
};

/**
 * The runtime's media query, or null where matchMedia does not exist (old
 * embeds, non-browser runtimes). Null means "cannot be asked" and resolves
 * to the dark fallback.
 */
export function defaultSystemThemeQuery(query: string): SystemThemeQuery | null {
  const ask = globalThis.matchMedia;
  return typeof ask === 'function' ? ask(query) : null;
}

/** Reads a stored choice. Any shape other than the one written is the default. */
export function parseThemeChoice(raw: string | null): ThemeId {
  if (raw === null) {
    return defaultTheme();
  }
  let stored: unknown;
  try {
    stored = JSON.parse(raw);
  } catch {
    return defaultTheme();
  }
  if (typeof stored !== 'object' || stored === null) {
    return defaultTheme();
  }
  const theme = (stored as Record<string, unknown>)['theme'];
  if (typeof theme === 'string' && isThemeId(theme)) {
    return theme;
  }
  return defaultTheme();
}

export function serializeThemeChoice(choice: ThemeId): string {
  return JSON.stringify({ theme: choice });
}

/**
 * Where the composition root keeps the person's settings. apps/demo wires
 * the browser's localStorage; a test wires a literal. A store that cannot
 * read or write (blocked storage, private window) simply returns null and
 * drops the write. Only preferences live here, never Activity, Identity or
 * Secret data (SEC-PRV-019).
 */
export type SettingsStore = {
  read(): string | null;
  write(value: string): void;
};

/** The store of a shell nobody wired one for: remembers nothing. */
export function noSettingsStore(): SettingsStore {
  return {
    read: () => null,
    write: () => undefined,
  };
}
