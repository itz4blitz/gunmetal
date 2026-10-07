import { expect, test } from 'vitest';
import { shellMessages } from '../messages/en/shell.ts';
import { navItems } from './Nav.tsx';

test('primary nav items are Home, Search and Library from the catalogue', () => {
  expect(navItems(shellMessages())).toStrictEqual([
    { path: '/', label: 'Home', icon: 'home' },
    { path: '/search', label: 'Search', icon: 'search' },
    { path: '/library', label: 'Library', icon: 'library' },
  ]);
});
