import { expect, test } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import { shellMessages } from '../../messages/en/shell.ts';
import {
  defaultSettingsSection,
  settingsConnectedRows,
  settingsLayout,
  settingsNavItems,
  settingsPlaybackRows,
  settingsRelease,
  settingsSectionTitle,
  settingsSections,
  settingsSlotPlaneLabel,
  settingsSlotTitle,
  settingsSwatchLabels,
  settingsThemeForKey,
  settingsThemeLabel,
} from './settings.ts';

test('settings sections are the six 2026 catalogue panes in listed order', () => {
  expect(settingsSections()).toStrictEqual(['appearance', 'playback', 'connected', 'extensions', 'about', 'privacy']);
  expect(defaultSettingsSection()).toStrictEqual('appearance');
});

test('settings layout is a left list on expanded and wide and stacked on compact', () => {
  expect(settingsLayout('compact')).toStrictEqual('stack');
  expect(settingsLayout('medium')).toStrictEqual('stack');
  expect(settingsLayout('expanded')).toStrictEqual('side');
  expect(settingsLayout('wide')).toStrictEqual('side');
});

test('release badges are honest R1 for present panes and R2 for plugins', () => {
  expect(settingsRelease('appearance')).toStrictEqual('R1');
  expect(settingsRelease('playback')).toStrictEqual('R1');
  expect(settingsRelease('about')).toStrictEqual('R1');
  expect(settingsRelease('privacy')).toStrictEqual('R1');
  expect(settingsRelease('connected')).toStrictEqual('R2');
  expect(settingsRelease('extensions')).toStrictEqual('R2');
});

test('section titles and nav items are the catalogue literals', () => {
  const messages = destinationMessages();
  expect(settingsSectionTitle('appearance', messages)).toStrictEqual('Appearance');
  expect(settingsSectionTitle('playback', messages)).toStrictEqual('Playback');
  expect(settingsSectionTitle('connected', messages)).toStrictEqual('Connected services');
  expect(settingsSectionTitle('extensions', messages)).toStrictEqual('Extensions / Plugins');
  expect(settingsSectionTitle('about', messages)).toStrictEqual('About this connection');
  expect(settingsSectionTitle('privacy', messages)).toStrictEqual('Privacy');
  expect(settingsNavItems(messages)).toStrictEqual([
    { id: 'appearance', label: 'Appearance' },
    { id: 'playback', label: 'Playback' },
    { id: 'connected', label: 'Connected services' },
    { id: 'extensions', label: 'Extensions / Plugins' },
    { id: 'about', label: 'About this connection' },
    { id: 'privacy', label: 'Privacy' },
  ]);
});

test('plugin slot titles and planes are the catalogue literals', () => {
  const messages = destinationMessages();
  expect(settingsSlotTitle('metadata-provider', messages)).toStrictEqual('Metadata and artwork');
  expect(settingsSlotTitle('lyrics-provider', messages)).toStrictEqual('Lyrics lookup');
  expect(settingsSlotTitle('search-provider', messages)).toStrictEqual('Catalogue search');
  expect(settingsSlotTitle('scrobbler', messages)).toStrictEqual('Scrobblers');
  expect(settingsSlotTitle('theme-pack', messages)).toStrictEqual('Themes');
  expect(settingsSlotTitle('home-row', messages)).toStrictEqual('Home rows');
  expect(settingsSlotPlaneLabel('server', messages)).toStrictEqual('Server');
  expect(settingsSlotPlaneLabel('client', messages)).toStrictEqual('Client');
});

test('the theme preview strip lists the five choices system-first with catalogue labels', () => {
  const shell = shellMessages();
  expect(settingsSwatchLabels(shell)).toStrictEqual([
    { id: 'system', label: 'System' },
    { id: 'dark', label: 'Dark' },
    { id: 'light', label: 'Light' },
    { id: 'oled', label: 'OLED' },
    { id: 'high-contrast', label: 'High contrast' },
  ]);
  expect(settingsThemeLabel('system', shell)).toStrictEqual('System');
  expect(settingsThemeLabel('dark', shell)).toStrictEqual('Dark');
  expect(settingsThemeLabel('high-contrast', shell)).toStrictEqual('High contrast');
});

test('radio-group arrows step through the five choices in order and wrap at both ends', () => {
  expect(settingsThemeForKey('system', 'ArrowRight')).toStrictEqual('dark');
  expect(settingsThemeForKey('dark', 'ArrowRight')).toStrictEqual('light');
  expect(settingsThemeForKey('light', 'ArrowDown')).toStrictEqual('oled');
  expect(settingsThemeForKey('oled', 'ArrowRight')).toStrictEqual('high-contrast');
  expect(settingsThemeForKey('high-contrast', 'ArrowRight')).toStrictEqual('system');
  expect(settingsThemeForKey('system', 'ArrowLeft')).toStrictEqual('high-contrast');
  expect(settingsThemeForKey('dark', 'ArrowLeft')).toStrictEqual('system');
  expect(settingsThemeForKey('high-contrast', 'ArrowUp')).toStrictEqual('oled');
  expect(settingsThemeForKey('light', 'ArrowLeft')).toStrictEqual('dark');
  // Every other key leaves the choice alone.
  expect(settingsThemeForKey('system', 'Enter')).toStrictEqual(undefined);
  expect(settingsThemeForKey('dark', 'Tab')).toStrictEqual(undefined);
  expect(settingsThemeForKey('dark', 'a')).toStrictEqual(undefined);
});

test('playback and connected rows name what is coming, with the catalogue words', () => {
  const messages = destinationMessages();
  expect(settingsPlaybackRows(messages)).toStrictEqual([
    {
      id: 'levelling',
      label: 'Volume levelling',
      hint: 'Plays tracks at a consistent loudness, from the tags in your files.',
    },
    { id: 'crossfade', label: 'Crossfade', hint: 'Blends the end of one track into the start of the next.' },
    { id: 'output', label: 'Output device', hint: 'Chooses the speakers or headphones this device plays through.' },
  ]);
  expect(settingsConnectedRows(messages)).toStrictEqual([
    {
      id: 'scrobble',
      label: 'Scrobbling',
      hint: 'Sends what you play to a listening-history service you link yourself.',
    },
    { id: 'lyrics', label: 'Lyrics lookup', hint: 'Finds lyrics for tracks whose files have none.' },
  ]);
});
