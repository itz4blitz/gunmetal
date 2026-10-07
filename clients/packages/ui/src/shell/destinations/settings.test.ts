import { expect, test } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import {
  defaultSettingsSection,
  settingsLayout,
  settingsNavItems,
  settingsRelease,
  settingsSectionTitle,
  settingsSections,
  settingsSlotPlaneLabel,
  settingsSlotTitle,
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
