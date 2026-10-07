/** Display types for the plugin-slot list; values arrive via props. */
export type PluginPlane = 'server' | 'client';

export type PluginSlotId =
  'metadata-provider' | 'lyrics-provider' | 'search-provider' | 'scrobbler' | 'theme-pack' | 'home-row';

export type PluginSlot = {
  id: PluginSlotId;
  plane: PluginPlane;
  featureId: 'INT-075' | 'INT-077' | 'INT-078' | 'INT-081' | 'INT-083';
  loaded: boolean;
  /** What the slot's manifest advertises, when one exists (ADR 22: data only). */
  manifest?: { title: string; version: string };
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

/** Playback settings that arrive with the playback controller; none is wired yet. */
export function settingsPlaybackRows(messages: DestinationMessages): readonly SettingsRowCopy[] {
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
