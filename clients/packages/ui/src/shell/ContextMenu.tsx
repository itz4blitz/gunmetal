import { useEffect } from 'react';
import { Text, View } from 'react-native-web';
import type { CatalogueMenuAction } from './menu-actions.ts';

export type ContextMenuProps = {
  open: boolean;
  label: string;
  actions: readonly CatalogueMenuAction[];
  onClose: () => void;
  onAction: (id: CatalogueMenuAction['id']) => void;
};

export function ContextMenu({ open, label, actions, onClose, onAction }: ContextMenuProps) {
  useEffect(() => {
    if (!open) {
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };
    globalThis.addEventListener('keydown', onKey);
    return () => {
      globalThis.removeEventListener('keydown', onKey);
    };
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  return (
    <View dataSet={{ contextMenu: '1' }} accessibilityRole="menu" accessibilityLabel={label}>
      {actions.map((action) => (
        <View
          key={action.id}
          dataSet={{ menuItem: action.id }}
          accessibilityRole="menuitem"
          accessibilityLabel={action.label}
          tabIndex={0}
          onClick={() => {
            onAction(action.id);
            onClose();
          }}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              onAction(action.id);
              onClose();
            }
          }}
        >
          <Text dataSet={{ menuLabel: '1' }}>{action.label}</Text>
        </View>
      ))}
    </View>
  );
}
