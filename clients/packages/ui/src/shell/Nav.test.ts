import { expect, test } from 'vitest';
import { destinationMessages } from '../messages/en/destinations.ts';
import { shellMessages } from '../messages/en/shell.ts';
import { navItems } from './Nav.tsx';

test('primary nav items are Home, Search, Library and the settings sections from the catalogue', () => {
  expect(navItems(shellMessages(), destinationMessages())).toStrictEqual([
    { path: '/', label: 'Home', icon: 'home' },
    { path: '/search', label: 'Search', icon: 'search' },
    { path: '/library', label: 'Library', icon: 'library' },
    { path: '/settings/appearance', label: 'Appearance', icon: 'settings' },
    { path: '/settings/playback', label: 'Playback', icon: 'settings' },
    { path: '/settings/extensions', label: 'Extensions / Plugins', icon: 'settings' },
    { path: '/settings/about', label: 'About this connection', icon: 'settings' },
    { path: '/settings/privacy', label: 'Privacy', icon: 'settings' },
  ]);
});
