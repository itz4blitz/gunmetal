import { useState, type ReactNode } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellMessages } from '../../messages/en/shell.ts';
import type { ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';
import { pluginSlots } from '../../../../fake-server/src/plugin-slots.ts';
import { ThemeSwitcher } from '../ThemeSwitcher.tsx';
import {
  defaultSettingsSection,
  settingsLayout,
  settingsNavItems,
  settingsRelease,
  settingsSectionTitle,
  settingsSections,
  settingsSlotPlaneLabel,
  settingsSlotTitle,
  type SettingsSection,
} from './settings.ts';

export type SettingsProps = {
  messages: DestinationMessages;
  shellMessages: ShellMessages;
  theme: ThemeId;
  onThemeChange: (theme: ThemeId) => void;
  width: WidthClass;
};

function activateKey(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

function ReleaseBadge({ section, messages }: { section: SettingsSection; messages: DestinationMessages }) {
  const release = settingsRelease(section);
  return (
    <Text dataSet={{ settingsBadge: release }}>
      {release === 'R1' ? messages.settingsBadgeR1 : messages.settingsBadgeR2}
    </Text>
  );
}

function SettingsPane({
  id,
  section,
  messages,
  children,
}: {
  id: string;
  section: SettingsSection;
  messages: DestinationMessages;
  children: ReactNode;
}) {
  return (
    <View id={id} dataSet={{ settingsSection: section, settingsPanel: '1' }}>
      <View dataSet={{ settingsHeadingRow: '1' }}>
        <Text accessibilityRole="header" dataSet={{ type: 'title2' }}>
          {settingsSectionTitle(section, messages)}
        </Text>
        <ReleaseBadge section={section} messages={messages} />
      </View>
      {children}
    </View>
  );
}

export function Settings({
  messages,
  shellMessages,
  theme,
  onThemeChange,
  width,
}: SettingsProps) {
  const layout = settingsLayout(width);
  const [section, setSection] = useState<SettingsSection>(defaultSettingsSection());
  const visible = layout === 'stack' ? settingsSections() : [section];

  return (
    <View id="destination-settings" dataSet={{ settingsLayout: layout }}>
      <Text id="destination-headline" accessibilityRole="header">
        {messages.settingsHeadline}
      </Text>
      {layout === 'side' ? (
        <View
          id="settings-nav"
          accessibilityRole="tablist"
          accessibilityLabel={messages.settingsNav}
        >
          {settingsNavItems(messages).map((item) => (
            <View
              key={item.id}
              id={`settings-nav-${item.id}`}
              accessibilityRole="tab"
              accessibilityLabel={item.label}
              accessibilityState={{ selected: section === item.id }}
              dataSet={{ settingsNavItem: item.id, selected: section === item.id ? '1' : '0' }}
              tabIndex={0}
              onClick={() => {
                setSection(item.id);
              }}
              onKeyDown={(event) => {
                activateKey(event, () => {
                  setSection(item.id);
                });
              }}
            >
              <Text>{item.label}</Text>
            </View>
          ))}
        </View>
      ) : null}
      <View id="settings-panels">
      {visible.includes('appearance') ? (
        <SettingsPane id="settings-appearance" section="appearance" messages={messages}>
          <ThemeSwitcher messages={shellMessages} theme={theme} onThemeChange={onThemeChange} />
        </SettingsPane>
      ) : null}
      {visible.includes('playback') ? (
        <SettingsPane id="settings-playback" section="playback" messages={messages}>
          <View dataSet={{ settingsStub: 'playback', emptyCard: '1' }}>
            <Text dataSet={{ settingsPlaceholder: 'playback' }}>
              {messages.settingsPlaybackPlaceholder}
            </Text>
          </View>
        </SettingsPane>
      ) : null}
      {visible.includes('connected') ? (
        <SettingsPane id="settings-connected" section="connected" messages={messages}>
          <View dataSet={{ emptyCard: '1', emptyRow: '1' }}>
            <View dataSet={{ emptyMark: '1' }} />
            <Text dataSet={{ emptyState: 'connected' }}>{messages.settingsConnectedEmpty}</Text>
          </View>
        </SettingsPane>
      ) : null}
      {visible.includes('extensions') ? (
        <SettingsPane id="settings-extensions" section="extensions" messages={messages}>
          <View dataSet={{ emptyCard: '1', emptyRow: '1' }}>
            <View dataSet={{ emptyMark: '1' }} />
            <Text dataSet={{ emptyState: 'extensions' }}>{messages.settingsExtensionsBody}</Text>
          </View>
          <View id="settings-plugin-slots">
            {pluginSlots().map((slot) => (
              <View
                key={slot.id}
                dataSet={{
                  pluginSlot: slot.id,
                  slotLoaded: slot.loaded ? '1' : '0',
                  slotPlane: slot.plane,
                }}
              >
                <Text dataSet={{ slotTitle: slot.id }}>{settingsSlotTitle(slot.id, messages)}</Text>
                <Text dataSet={{ slotPlane: slot.plane }}>{settingsSlotPlaneLabel(slot.plane, messages)}</Text>
                <Text dataSet={{ slotState: slot.loaded ? '1' : '0' }}>{messages.settingsSlotUnloaded}</Text>
              </View>
            ))}
          </View>
        </SettingsPane>
      ) : null}
      {visible.includes('about') ? (
        <SettingsPane id="settings-about" section="about" messages={messages}>
          <Text dataSet={{ settingsFact: 'data' }}>{messages.settingsAboutData}</Text>
          <Text dataSet={{ settingsFact: 'address' }}>{messages.settingsAboutAddress}</Text>
          <Text dataSet={{ settingsFact: 'version' }}>{messages.settingsAboutVersion}</Text>
        </SettingsPane>
      ) : null}
      {visible.includes('privacy') ? (
        <SettingsPane id="settings-privacy" section="privacy" messages={messages}>
          <Text dataSet={{ settingsPrivacy: '1' }}>{messages.settingsPrivacyBody}</Text>
        </SettingsPane>
      ) : null}
      </View>
    </View>
  );
}
