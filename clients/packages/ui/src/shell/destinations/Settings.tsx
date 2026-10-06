import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellMessages } from '../../messages/en/shell.ts';
import type { ThemeId } from '../theme.ts';
import { ThemeSwitcher } from '../ThemeSwitcher.tsx';

export type SettingsProps = {
  messages: DestinationMessages;
  shellMessages: ShellMessages;
  theme: ThemeId;
  onThemeChange: (theme: ThemeId) => void;
};

export function Settings({ messages, shellMessages, theme, onThemeChange }: SettingsProps) {
  return (
    <View id="destination-settings">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.settingsHeadline}
      </Text>
      <View id="settings-appearance" dataSet={{ settingsSection: 'appearance' }}>
        <Text accessibilityRole="header" dataSet={{ type: 'title2' }}>
          {messages.settingsAppearance}
        </Text>
        <ThemeSwitcher messages={shellMessages} theme={theme} onThemeChange={onThemeChange} />
      </View>
      <View id="settings-demo-data" dataSet={{ settingsSection: 'demo-data' }}>
        <Text accessibilityRole="header" dataSet={{ type: 'title2' }}>
          {messages.settingsDemoData}
        </Text>
        <Text dataSet={{ settingsDemoBody: '1' }}>{messages.settingsDemoDataBody}</Text>
      </View>
      <View id="settings-playback" dataSet={{ settingsSection: 'playback' }}>
        <Text accessibilityRole="header" dataSet={{ type: 'title2' }}>
          {messages.settingsPlayback}
        </Text>
        <View dataSet={{ settingsStub: 'playback', emptyCard: '1' }}>
          <Text dataSet={{ settingsPlaceholder: 'playback' }}>
            {messages.settingsPlaybackPlaceholder}
          </Text>
        </View>
      </View>
      <View id="settings-about" dataSet={{ settingsSection: 'about' }}>
        <Text accessibilityRole="header" dataSet={{ type: 'title2' }}>
          {messages.settingsAbout}
        </Text>
        <Text dataSet={{ settingsAbout: '1' }}>{messages.settingsAboutBody}</Text>
      </View>
    </View>
  );
}
