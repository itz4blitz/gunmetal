import { expect, test } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import { shellMessages } from '../../messages/en/shell.ts';
import {
  defaultSettingsSection,
  settingsConnectedRows,
  settingsJobDetail,
  settingsJobStatusLabel,
  settingsLayout,
  settingsNavItems,
  settingsPlaybackRows,
  settingsRelease,
  settingsSectionFromPath,
  settingsSectionPath,
  settingsSectionTitle,
  settingsSections,
  settingsSlotPlaneLabel,
  settingsSlotTitle,
  settingsSwatchLabels,
  settingsThemeForKey,
  settingsThemeLabel,
} from './settings.ts';

test('settings sections are the five 2026 catalogue panes in listed order', () => {
  expect(settingsSections()).toStrictEqual(['appearance', 'playback', 'connected', 'about', 'privacy']);
  expect(defaultSettingsSection()).toStrictEqual('appearance');
});

test('with libraries content, the libraries pane sits third in the order', () => {
  expect(settingsSections(true)).toStrictEqual([
    'appearance',
    'playback',
    'libraries',
    'connected',
    'about',
    'privacy',
  ]);
});

test('a settings route names its pane, and any other path names none', () => {
  expect(settingsSectionFromPath('/settings')).toStrictEqual('appearance');
  expect(settingsSectionFromPath('/settings/appearance')).toStrictEqual('appearance');
  expect(settingsSectionFromPath('/settings/playback')).toStrictEqual('playback');
  expect(settingsSectionFromPath('/settings/libraries')).toStrictEqual('libraries');
  expect(settingsSectionFromPath('/settings/extensions')).toStrictEqual(undefined);
  expect(settingsSectionFromPath('/settings/about')).toStrictEqual('about');
  expect(settingsSectionFromPath('/settings/privacy')).toStrictEqual('privacy');
  expect(settingsSectionFromPath('/settings/connected')).toStrictEqual('connected');
  expect(settingsSectionFromPath('/store')).toStrictEqual(undefined);
  expect(settingsSectionFromPath('/library')).toStrictEqual(undefined);
  expect(settingsSectionFromPath('')).toStrictEqual(undefined);
});

test('every section answers its settings address', () => {
  expect(settingsSectionPath('appearance')).toStrictEqual('/settings/appearance');
  expect(settingsSectionPath('playback')).toStrictEqual('/settings/playback');
  expect(settingsSectionPath('connected')).toStrictEqual('/settings/connected');
  expect(settingsSectionPath('about')).toStrictEqual('/settings/about');
  expect(settingsSectionPath('privacy')).toStrictEqual('/settings/privacy');
});

test('settings layout is a left list on expanded and wide and stacked on compact', () => {
  expect(settingsLayout('compact')).toStrictEqual('stack');
  expect(settingsLayout('medium')).toStrictEqual('stack');
  expect(settingsLayout('expanded')).toStrictEqual('side');
  expect(settingsLayout('wide')).toStrictEqual('side');
});

test('release badges are honest R1 or R2', () => {
  expect(settingsRelease('appearance')).toStrictEqual('R1');
  expect(settingsRelease('playback')).toStrictEqual('R1');
  expect(settingsRelease('about')).toStrictEqual('R1');
  expect(settingsRelease('privacy')).toStrictEqual('R1');
  expect(settingsRelease('connected')).toStrictEqual('R2');
  expect(settingsRelease('libraries')).toStrictEqual('R2');
});

test('section titles and nav items are the catalogue literals', () => {
  const messages = destinationMessages();
  expect(settingsSectionTitle('appearance', messages)).toStrictEqual('Appearance');
  expect(settingsSectionTitle('playback', messages)).toStrictEqual('Playback');
  expect(settingsSectionTitle('libraries', messages)).toStrictEqual('Libraries');
  expect(settingsSectionTitle('connected', messages)).toStrictEqual('Connected services');
  expect(settingsSectionTitle('about', messages)).toStrictEqual('About this connection');
  expect(settingsSectionTitle('privacy', messages)).toStrictEqual('Privacy');
  expect(settingsNavItems(messages)).toStrictEqual([
    { id: 'appearance', label: 'Appearance' },
    { id: 'playback', label: 'Playback' },
    { id: 'connected', label: 'Connected services' },
    { id: 'about', label: 'About this connection' },
    { id: 'privacy', label: 'Privacy' },
  ]);
  expect(settingsNavItems(messages, true)).toStrictEqual([
    { id: 'appearance', label: 'Appearance' },
    { id: 'playback', label: 'Playback' },
    { id: 'libraries', label: 'Libraries' },
    { id: 'connected', label: 'Connected services' },
    { id: 'about', label: 'About this connection' },
    { id: 'privacy', label: 'Privacy' },
  ]);
  expect(settingsSectionPath('libraries')).toStrictEqual('/settings/libraries');
});

test('every section has its own path', () => {
  expect(settingsSectionPath('appearance')).toStrictEqual('/settings/appearance');
  expect(settingsSectionPath('playback')).toStrictEqual('/settings/playback');
  expect(settingsSectionPath('libraries')).toStrictEqual('/settings/libraries');
  expect(settingsSectionPath('connected')).toStrictEqual('/settings/connected');
  expect(settingsSectionPath('about')).toStrictEqual('/settings/about');
  expect(settingsSectionPath('privacy')).toStrictEqual('/settings/privacy');
});

test('job detail and status are the catalogue literals, with no plugin version or badge', () => {
  const messages = destinationMessages();
  expect(settingsJobDetail('metadata-provider', messages)).toStrictEqual(
    'Built into this library host. Cover Art Archive.',
  );
  expect(settingsJobDetail('lyrics-provider', messages)).toStrictEqual(undefined);
  expect(settingsJobDetail('search-provider', messages)).toStrictEqual(undefined);
  expect(settingsJobDetail('scrobbler', messages)).toStrictEqual(undefined);
  expect(settingsJobDetail('theme-pack', messages)).toStrictEqual(
    'Not a separate plugin. Themes are the settings appearance control.',
  );
  expect(settingsJobDetail('home-row', messages)).toStrictEqual('Not a separate plugin.');
  expect(settingsJobStatusLabel('on', messages)).toStrictEqual('On');
  expect(settingsJobStatusLabel('not-serving', messages)).toStrictEqual(undefined);
  expect(settingsJobStatusLabel('not-in-build', messages)).toStrictEqual('Not in this build');
  expect(settingsJobStatusLabel('not-a-plugin', messages)).toStrictEqual(undefined);
});

test('plugin slot titles and planes are the catalogue literals', () => {
  const messages = destinationMessages();
  expect(settingsSlotTitle('metadata-provider', messages)).toStrictEqual('Metadata and artwork');
  expect(settingsSlotTitle('lyrics-provider', messages)).toStrictEqual('Lyrics lookup');
  expect(settingsSlotTitle('search-provider', messages)).toStrictEqual('Catalog search');
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
