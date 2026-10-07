import { useEffect } from 'react';
import type { MouseEvent } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { catalogueMenuActions, type CatalogueMenuActionId } from '../menu-actions.ts';

export type CatalogueMenuHandlers = {
  onPlay?: (() => void) | undefined;
  onPlayNext?: (() => void) | undefined;
  onAddToQueue?: (() => void) | undefined;
  onGoToAlbum?: (() => void) | undefined;
  onOpenArtist: (artistKey: string) => void;
};

export type GoToArtistMenuProps = CatalogueMenuHandlers & {
  open: boolean;
  artistKey: string;
  messages: DestinationMessages;
  onClose: () => void;
};

function runAction(id: CatalogueMenuActionId, artistKey: string, handlers: CatalogueMenuHandlers): void {
  if (id === 'play') {
    handlers.onPlay?.();
    return;
  }
  if (id === 'play-next') {
    handlers.onPlayNext?.();
    return;
  }
  if (id === 'add-to-queue') {
    handlers.onAddToQueue?.();
    return;
  }
  if (id === 'go-to-album') {
    handlers.onGoToAlbum?.();
    return;
  }
  handlers.onOpenArtist(artistKey);
}

export function GoToArtistMenu({
  open,
  artistKey,
  messages,
  onOpenArtist,
  onClose,
  onPlay,
  onPlayNext,
  onAddToQueue,
  onGoToAlbum,
}: GoToArtistMenuProps) {
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

  const handlers = { onPlay, onPlayNext, onAddToQueue, onGoToAlbum, onOpenArtist };

  return (
    <View
      dataSet={{ itemMenu: '1', contextMenu: '1' }}
      accessibilityRole="menu"
      accessibilityLabel={messages.contextMenu}
    >
      {catalogueMenuActions(messages).map((action) => (
        <View
          key={action.id}
          dataSet={{
            menuItem: action.id,
            ...(action.id === 'go-to-artist' ? { goToArtist: '1' } : {}),
          }}
          accessibilityRole="menuitem"
          accessibilityLabel={action.label}
          tabIndex={0}
          onClick={(event: MouseEvent<HTMLElement>) => {
            event.stopPropagation();
            runAction(action.id, artistKey, handlers);
            onClose();
          }}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              event.stopPropagation();
              runAction(action.id, artistKey, handlers);
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
