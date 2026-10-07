import type { ReactNode } from 'react';
import { Text, View } from 'react-native-web';
import type { ShellMessages } from '../messages/en/shell.ts';
import { Icon, type IconName } from './Icon.tsx';

export type NavItem = {
  path: string;
  label: string;
  icon: IconName;
};

export function navItems(messages: ShellMessages): readonly NavItem[] {
  return [
    { path: '/', label: messages.navHome, icon: 'home' },
    { path: '/search', label: messages.navSearch, icon: 'search' },
    { path: '/library', label: messages.navLibrary, icon: 'library' },
  ];
}

function glyphKey(path: string): string {
  return path === '/' ? 'home' : path.slice(1);
}

export type NavProps = {
  id: 'nav-tabs' | 'nav-rail' | 'nav-sidebar';
  label: string;
  items: readonly NavItem[];
  activePath: string;
  onNavigate: (path: string) => void;
  brand?: ReactNode;
  footer?: ReactNode;
};

export function Nav({ id, label, items, activePath, onNavigate, brand, footer }: NavProps) {
  return (
    <View id={id} accessibilityRole="navigation" accessibilityLabel={label}>
      {brand}
      {items.map((item) => {
        const selected = item.path === activePath;
        const itemId = glyphKey(item.path);
        return (
          <View
            key={item.path}
            id={`nav-item-${itemId}`}
            dataSet={{ navGlyph: itemId, selected: selected ? '1' : '0' }}
            accessibilityRole="link"
            accessibilityLabel={item.label}
            accessibilityState={{ selected }}
            tabIndex={0}
            onClick={() => {
              onNavigate(item.path);
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                onNavigate(item.path);
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
