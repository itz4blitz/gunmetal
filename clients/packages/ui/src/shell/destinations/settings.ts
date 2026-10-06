import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { WidthClass } from '../width.ts';

export type SettingsSection =
  | 'appearance'
  | 'playback'
  | 'connected'
  | 'extensions'
  | 'about'
  | 'privacy';

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

export function settingsSectionTitle(
  section: SettingsSection,
  messages: DestinationMessages,
): string {
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

export function settingsNavItems(
  messages: DestinationMessages,
): readonly { id: SettingsSection; label: string }[] {
  return settingsSections().map((id) => ({
    id,
    label: settingsSectionTitle(id, messages),
  }));
}
