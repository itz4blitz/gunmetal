/** Display types for the first-party jobs list; values arrive via props. */
export type PluginPlane = 'server' | 'client';

export type PluginSlotId =
  'metadata-provider' | 'lyrics-provider' | 'search-provider' | 'scrobbler' | 'theme-pack' | 'home-row';

/** Closed status for a job. Not a plugin load bit, and not a release badge. */
export type FirstPartyJobStatus = 'on' | 'not-serving' | 'not-in-build' | 'not-a-plugin';

export type PluginSlot = {
  id: PluginSlotId;
  status: FirstPartyJobStatus;
};
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellMessages } from '../../messages/en/shell.ts';
import { themes, type ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';

export type SettingsSection = 'appearance' | 'playback' | 'libraries' | 'connected' | 'about' | 'privacy';

export type SettingsLayout = 'stack' | 'side';

/**
 * The settings sections in order. Libraries is shown only where the host
 * passed libraries content: the walking skeleton's sources, not a promise
 * the R1 music app would be making.
 */
export function settingsSections(withLibraries = false): readonly SettingsSection[] {
  const tail: readonly SettingsSection[] = ['connected', 'about', 'privacy'];
  return withLibraries ? ['appearance', 'playback', 'libraries', ...tail] : ['appearance', 'playback', ...tail];
}

export function defaultSettingsSection(): SettingsSection {
  return 'appearance';
}

/**
 * The settings section a route names. `/settings` is appearance. A path that
 * is not a settings section answers undefined so the page keeps its own choice.
 */
export function settingsSectionFromPath(path: string): SettingsSection | undefined {
  if (path === '/settings' || path === '/settings/appearance') {
    return 'appearance';
  }
  if (path === '/settings/playback') {
    return 'playback';
  }
  if (path === '/settings/libraries') {
    return 'libraries';
  }
  if (path === '/settings/connected') {
    return 'connected';
  }
  if (path === '/settings/about') {
    return 'about';
  }
  if (path === '/settings/privacy') {
    return 'privacy';
  }
  return undefined;
}

export function settingsLayout(width: WidthClass): SettingsLayout {
  if (width === 'expanded') {
    return 'side';
  }
  if (width === 'wide') {
    return 'side';
  }
  return 'stack';
}

/** A section wears no release badge: a badge there would pretend a state exists. */
export function settingsRelease(section: SettingsSection): 'R1' | 'R2' | undefined {
  if (section === 'connected') {
    return 'R2';
  }
  if (section === 'libraries') {
    return 'R2';
  }
  return 'R1';
}

export function settingsSectionTitle(section: SettingsSection, messages: DestinationMessages): string {
  if (section === 'appearance') {
    return messages.settingsAppearance;
  }
  if (section === 'playback') {
    return messages.settingsPlayback;
  }
  if (section === 'libraries') {
    return messages.settingsLibraries;
  }
  if (section === 'connected') {
    return messages.settingsConnected;
  }
  if (section === 'about') {
    return messages.settingsAbout;
  }
  return messages.settingsPrivacy;
}

export function settingsSectionPath(section: SettingsSection): string {
  if (section === 'appearance') {
    return '/settings/appearance';
  }
  if (section === 'playback') {
    return '/settings/playback';
  }
  if (section === 'libraries') {
    return '/settings/libraries';
  }
  if (section === 'connected') {
    return '/settings/connected';
  }
  if (section === 'about') {
    return '/settings/about';
  }
  return '/settings/privacy';
}

export function settingsNavItems(
  messages: DestinationMessages,
  withLibraries = false,
): readonly { id: SettingsSection; label: string }[] {
  return settingsSections(withLibraries).map((id) => ({
    id,
    label: settingsSectionTitle(id, messages),
  }));
}

export function settingsSlotTitle(id: PluginSlotId, messages: DestinationMessages): string {
  if (id === 'metadata-provider') {
    return messages.settingsSlotMetadata;
  }
  if (id === 'lyrics-provider') {
    return messages.settingsSlotLyrics;
  }
  if (id === 'search-provider') {
    return messages.settingsSlotSearch;
  }
  if (id === 'scrobbler') {
    return messages.settingsSlotScrobble;
  }
  if (id === 'theme-pack') {
    return messages.settingsSlotTheme;
  }
  return messages.settingsSlotHome;
}

export function settingsSlotPlaneLabel(plane: PluginPlane, messages: DestinationMessages): string {
  if (plane === 'server') {
    return messages.settingsSlotServer;
  }
  return messages.settingsSlotClient;
}

/** The sentence under a job, when the job has one. Lyrics, search and scrobble do not. */
export function settingsJobDetail(id: PluginSlotId, messages: DestinationMessages): string | undefined {
  if (id === 'metadata-provider') {
    return messages.settingsJobCoverArt;
  }
  if (id === 'theme-pack') {
    return messages.settingsJobThemes;
  }
  if (id === 'home-row') {
    return messages.settingsJobHome;
  }
  return undefined;
}

/** Status words. Not-serving and not-a-plugin have none: a tag there would pretend a plugin state. */
export function settingsJobStatusLabel(status: FirstPartyJobStatus, messages: DestinationMessages): string | undefined {
  if (status === 'on') {
    return messages.settingsJobOn;
  }
  if (status === 'not-in-build') {
    return messages.settingsJobNotInBuild;
  }
  return undefined;
}

/** A choice's catalogue label: the name on its preview card. */
export function settingsThemeLabel(id: ThemeId, messages: ShellMessages): string {
  if (id === 'system') {
    return messages.shellThemeSystem;
  }
  if (id === 'dark') {
    return messages.themeDark;
  }
  if (id === 'light') {
    return messages.themeLight;
  }
  if (id === 'oled') {
    return messages.themeOled;
  }
  return messages.themeHighContrast;
}

/** The theme preview cards: the five choices, System first. */
export function settingsSwatchLabels(messages: ShellMessages): readonly { id: ThemeId; label: string }[] {
  return themes().map((id) => ({ id, label: settingsThemeLabel(id, messages) }));
}

/**
 * Radio-group arrows for the theme cards: the theme the key moves to, wrapping
 * at both ends. Any other key answers undefined and leaves the choice alone.
 */
export function settingsThemeForKey(current: ThemeId, key: string): ThemeId | undefined {
  const order = themes();
  if (key === 'ArrowRight' || key === 'ArrowDown') {
    return order[(order.indexOf(current) + 1) % order.length];
  }
  if (key === 'ArrowLeft' || key === 'ArrowUp') {
    return order[(order.indexOf(current) + order.length - 1) % order.length];
  }
  return undefined;
}

/** One setting row's words: what the setting is called and what it does. */
export type SettingsRowCopy = { id: string; label: string; hint: string };

export type SettingsLevelling = 'off' | 'track' | 'album';

export type SettingsCrossfadeSeconds = 0 | 2 | 4 | 6 | 8 | 12;

/** Label and hint for each playback setting. The pane renders each as a control. */
export function settingsPlaybackRows(
  messages: DestinationMessages,
): readonly [SettingsRowCopy, SettingsRowCopy, SettingsRowCopy] {
  return [
    { id: 'levelling', label: messages.settingsPlaybackLevelling, hint: messages.settingsPlaybackLevellingHint },
    { id: 'crossfade', label: messages.settingsPlaybackCrossfade, hint: messages.settingsPlaybackCrossfadeHint },
    { id: 'output', label: messages.settingsPlaybackOutput, hint: messages.settingsPlaybackOutputHint },
  ];
}

/** Services a person links for themselves once signed plugins exist; none is wired yet. */
export function settingsConnectedRows(messages: DestinationMessages): readonly SettingsRowCopy[] {
  return [
    { id: 'scrobble', label: messages.settingsConnectedScrobble, hint: messages.settingsConnectedScrobbleHint },
    { id: 'lyrics', label: messages.settingsSlotLyrics, hint: messages.settingsConnectedLyricsHint },
  ];
}
