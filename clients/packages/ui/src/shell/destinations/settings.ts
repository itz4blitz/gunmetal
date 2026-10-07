/** Display types for the plugin-slot list; values arrive via props. */
export type PluginPlane = 'server' | 'client';

export type PluginSlotId =
  'metadata-provider' | 'lyrics-provider' | 'search-provider' | 'scrobbler' | 'theme-pack' | 'home-row';

export type PluginSlot = {
  id: PluginSlotId;
  plane: PluginPlane;
  featureId: 'INT-075' | 'INT-077' | 'INT-078' | 'INT-081' | 'INT-083';
  loaded: boolean;
};
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellMessages } from '../../messages/en/shell.ts';
import { themes, type ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';

export type SettingsSection = 'appearance' | 'playback' | 'connected' | 'extensions' | 'about' | 'privacy';

export type SettingsLayout = 'stack' | 'side';

export function settingsSections(): readonly SettingsSection[] {
  return ['appearance', 'playback', 'connected', 'extensions', 'about', 'privacy'];
}

export function defaultSettingsSection(): SettingsSection {
  return 'appearance';
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

export function settingsRelease(section: SettingsSection): 'R1' | 'R2' {
  if (section === 'connected') {
    return 'R2';
  }
  if (section === 'extensions') {
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
  if (section === 'connected') {
    return messages.settingsConnected;
  }
  if (section === 'extensions') {
    return messages.settingsExtensions;
  }
  if (section === 'about') {
    return messages.settingsAbout;
  }
  return messages.settingsPrivacy;
}

export function settingsNavItems(messages: DestinationMessages): readonly { id: SettingsSection; label: string }[] {
  return settingsSections().map((id) => ({
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

/** The switcher's label for a theme; the preview strip captions use the same words. */
export function settingsThemeLabel(id: ThemeId, messages: ShellMessages): string {
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

/** The theme preview strip: the same four themes as the switcher, in switcher order. */
export function settingsSwatchLabels(messages: ShellMessages): readonly { id: ThemeId; label: string }[] {
  return themes().map((id) => ({ id, label: settingsThemeLabel(id, messages) }));
}
