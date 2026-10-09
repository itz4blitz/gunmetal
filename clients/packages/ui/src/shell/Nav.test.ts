import { expect, test } from 'vitest';
import { destinationMessages } from '../messages/en/destinations.ts';
import { shellMessages } from '../messages/en/shell.ts';
import { navItemSelected, navItems, withPins, type NavItem } from './Nav.tsx';

test('primary nav items are Home, Search, Library, Store and Settings', () => {
  expect(navItems(shellMessages(), destinationMessages())).toStrictEqual([
    { path: '/', label: 'Home', icon: 'home' },
    { path: '/search', label: 'Search', icon: 'search' },
    { path: '/library', label: 'Library', icon: 'library' },
    { path: '/store', label: 'Store', icon: 'store' },
    { path: '/settings', label: 'Settings', icon: 'settings' },
  ]);
});

test('withPins appends pins that are not already listed, in pin order', () => {
  const items: readonly NavItem[] = [
    { path: '/', label: 'Home', icon: 'home' },
    { path: '/search', label: 'Search', icon: 'search' },
    { path: '/library', label: 'Library', icon: 'library' },
    { path: '/settings', label: 'Settings', icon: 'settings' },
  ];
  expect(
    withPins(items, [
      { path: '/library', label: 'Library again' },
      { path: '/settings/appearance', label: 'Appearance' },
      { path: '/playlists/loved', label: 'Loved' },
    ]),
  ).toStrictEqual([
    { path: '/', label: 'Home', icon: 'home' },
    { path: '/search', label: 'Search', icon: 'search' },
    { path: '/library', label: 'Library', icon: 'library' },
    { path: '/settings', label: 'Settings', icon: 'settings' },
    { path: '/settings/appearance', label: 'Appearance', icon: 'settings' },
    { path: '/playlists/loved', label: 'Loved', icon: 'library' },
  ]);
  expect(
    withPins(items, [
      { path: '/library', label: 'Library again' },
      { path: '/library', label: 'Harbour Lights', itemId: 'demo-album-01' },
      { path: '/library', label: 'Mira Sol', itemId: 'mira-sol' },
    ]),
  ).toStrictEqual([
    { path: '/', label: 'Home', icon: 'home' },
    { path: '/search', label: 'Search', icon: 'search' },
    { path: '/library', label: 'Library', icon: 'library' },
    { path: '/settings', label: 'Settings', icon: 'settings' },
    { path: '/library', label: 'Harbour Lights', icon: 'disc', itemId: 'demo-album-01' },
    { path: '/library', label: 'Mira Sol', icon: 'disc', itemId: 'mira-sol' },
  ]);
  expect(
    withPins(items, [
      { path: '/music/albums/harbour-lights', label: 'Harbour Lights' },
      { path: '/watch/movies/inception', label: 'Inception' },
    ]),
  ).toStrictEqual([
    { path: '/', label: 'Home', icon: 'home' },
    { path: '/search', label: 'Search', icon: 'search' },
    { path: '/library', label: 'Library', icon: 'library' },
    { path: '/settings', label: 'Settings', icon: 'settings' },
    { path: '/music/albums/harbour-lights', label: 'Harbour Lights', icon: 'disc' },
    { path: '/watch/movies/inception', label: 'Inception', icon: 'disc' },
  ]);
});

test('a media address selects itself, and a section does not while an item is open', () => {
  const home: NavItem = { path: '/', label: 'Home', icon: 'home' };
  const search: NavItem = { path: '/search', label: 'Search', icon: 'search' };
  const library: NavItem = { path: '/library', label: 'Library', icon: 'library' };
  const album: NavItem = { path: '/library', label: 'Harbour Lights', icon: 'disc', itemId: 'demo-album-01' };
  const slug: NavItem = { path: '/music/albums/harbour-lights', label: 'Harbour Lights', icon: 'disc' };
  expect(navItemSelected(album, '/library', 'demo-album-01')).toStrictEqual(true);
  expect(navItemSelected(album, '/library', 'demo-album-02')).toStrictEqual(false);
  expect(navItemSelected(home, '/search', undefined)).toStrictEqual(false);
  expect(navItemSelected(home, '/', 'demo-album-01')).toStrictEqual(false);
  expect(navItemSelected(search, '/search', 'demo-album-01')).toStrictEqual(false);
  expect(navItemSelected(library, '/library', 'demo-album-01')).toStrictEqual(false);
  expect(navItemSelected(slug, '/music/albums/harbour-lights', 'demo-album-01')).toStrictEqual(true);
  expect(navItemSelected(library, '/library', undefined)).toStrictEqual(true);
});
