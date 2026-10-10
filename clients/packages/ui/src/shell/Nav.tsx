import type { ReactNode } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../messages/en/destinations.ts';
import type { ShellMessages } from '../messages/en/shell.ts';
import { parseMediaPath } from '../router/media-path.ts';
import { Icon, type IconName } from './Icon.tsx';
import type { Pin } from './pins.ts';

export type NavItem = {
  path: string;
  label: string;
  icon: IconName;
  itemId?: string;
};

/** Watch is listed only for a host that passes a watch area. */
export function navItems(
  messages: ShellMessages,
  destinations: DestinationMessages,
  withWatch = false,
): readonly NavItem[] {
  // Section pages are pins. The argument stays so the shell can keep passing the catalogue.
  void destinations;
  const watch: readonly NavItem[] = withWatch ? [{ path: '/watch', label: messages.navWatch, icon: 'watch' }] : [];
  return [
    { path: '/', label: messages.navHome, icon: 'home' },
    { path: '/search', label: messages.navSearch, icon: 'search' },
    { path: '/library', label: messages.navLibrary, icon: 'library' },
    ...watch,
    { path: '/store', label: messages.navStore, icon: 'store' },
    { path: '/settings', label: messages.navSettings, icon: 'settings' },
  ];
}

function iconForPin(pin: Pin): IconName {
  if (pin.itemId !== undefined || parseMediaPath(pin.path) !== undefined) {
    return 'disc';
  }
  if (pin.path.startsWith('/settings')) {
    return 'settings';
  }
  return 'library';
}

/** A section stays unselected while an item is open on that section. A media address selects itself. */
export function navItemSelected(item: NavItem, activePath: string, activeItemId: string | undefined): boolean {
  if (item.itemId !== undefined) {
    return item.itemId === activeItemId;
  }
  if (item.path !== activePath) {
    return false;
  }
  if (activeItemId !== undefined && (item.path === '/' || item.path === '/search' || item.path === '/library')) {
    return false;
  }
  return true;
}

export function withPins(items: readonly NavItem[], pins: readonly Pin[]): readonly NavItem[] {
  const present = new Set(items.filter((item) => item.itemId === undefined).map((item) => item.path));
  const appended: NavItem[] = [];
  for (const pin of pins) {
    if (pin.itemId === undefined && present.has(pin.path)) {
      continue;
    }
    const next: NavItem = { path: pin.path, label: pin.label, icon: iconForPin(pin) };
    if (pin.itemId !== undefined) {
      next.itemId = pin.itemId;
    }
    appended.push(next);
  }
  return [...items, ...appended];
}

function glyphKey(path: string): string {
  if (path === '/') {
    return 'home';
  }
  return path.slice(1).replaceAll('/', '-');
}

export type NavProps = {
  id: 'nav-tabs' | 'nav-rail' | 'nav-sidebar';
  label: string;
  items: readonly NavItem[];
  activePath: string;
  activeItemId?: string | undefined;
  onNavigate: (path: string, itemId?: string) => void;
  brand?: ReactNode;
  footer?: ReactNode;
};

export function Nav({ id, label, items, activePath, activeItemId, onNavigate, brand, footer }: NavProps) {
  return (
    <View id={id} accessibilityRole="navigation" accessibilityLabel={label}>
      {brand}
      {items.map((item) => {
        const selected = navItemSelected(item, activePath, activeItemId);
        const glyph = item.itemId === undefined ? glyphKey(item.path) : `item-${item.itemId}`;
        return (
          <View
            key={item.itemId === undefined ? item.path : `${item.path}:${item.itemId}`}
            id={`nav-item-${glyph}`}
            dataSet={{ navGlyph: glyph, selected: selected ? '1' : '0' }}
            accessibilityRole="link"
            accessibilityLabel={item.label}
            aria-current={selected ? 'page' : undefined}
            tabIndex={0}
            onClick={() => {
              onNavigate(item.path, item.itemId);
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                onNavigate(item.path, item.itemId);
              }
            }}
          >
            <Icon name={item.icon} />
            <Text dataSet={{ navLabel: '1' }}>{item.label}</Text>
          </View>
        );
      })}
      {footer}
    </View>
  );
}
