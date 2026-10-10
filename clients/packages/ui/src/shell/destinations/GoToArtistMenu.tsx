import { useEffect, useLayoutEffect, useRef, type MouseEvent } from 'react';
import { createPortal } from 'react-dom';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { catalogueMenuActions, type CatalogueMenuActionId } from '../menu-actions.ts';
import { MENU_EDGE_PX, placeMenu, type MenuPoint } from '../menu-anchor.ts';
import { useMenuDismiss } from '../menu-dismiss.ts';

export type CatalogueMenuHandlers = {
  onPlay?: (() => void) | undefined;
  onPlayNext?: (() => void) | undefined;
  onAddToQueue?: (() => void) | undefined;
  onGoToAlbum?: (() => void) | undefined;
  onOpenArtist: (artistKey: string) => void;
};

export type GoToArtistMenuProps = CatalogueMenuHandlers & {
  open: boolean;
  /** Names this menu; its opener points at it with aria-controls. */
  menuId: string;
  artistKey: string;
  messages: DestinationMessages;
  onClose: () => void;
  /**
   * Where the menu opens: the trigger's bottom-right corner, captured when
   * it was opened. The menu opens below that point, flips above the
   * trigger near the viewport's bottom, and never pokes past an edge.
   */
  at?: MenuPoint | undefined;
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
  menuId,
  artistKey,
  messages,
  onOpenArtist,
  onClose,
  onPlay,
  onPlayNext,
  onAddToQueue,
  onGoToAlbum,
  at = { x: 0, y: 0 },
}: GoToArtistMenuProps) {
  useMenuDismiss(open, menuId, onClose);

  /* The menu portals out of the tile and is fixed to the viewport. A
     portal is the only honest escape: the tiles carry entrance transforms,
     and a transformed ancestor becomes the containing block for fixed
     elements — a menu left inside the tile would be "fixed" to the tile
     and clipped by its shelf scroller. The portal lands on the shell root
     (not the body): the design tokens live on #token-shell's theme scope,
     and a body-level menu would paint itself without any of them. The
     coordinates are measured from the rendered menu and written through
     the CSSOM before the first paint — never a style attribute. */
  useLayoutEffect(() => {
    if (!open) {
      return;
    }
    const menu = document.querySelector(`[data-menu-id="${menuId}"]`) as HTMLElement;
    const rect = menu.getBoundingClientRect();
    const placed = placeMenu(
      at,
      { width: rect.width, height: rect.height },
      { width: globalThis.innerWidth, height: globalThis.innerHeight },
      MENU_EDGE_PX,
    );
    menu.style.setProperty('left', `${placed.left}px`);
    menu.style.setProperty('top', `${placed.top}px`);
    /* The menu is portalled to the shell root, so tabbing cannot reach it
       from the trigger: the first item takes focus on open. */
    const first = menu.querySelector('[data-menu-item]') as HTMLElement;
    first.focus();
  }, [open, menuId, at]);

  /* Closing hands focus back to the opener, the way a native menu does. */
  const wasOpen = useRef(false);
  useEffect(() => {
    if (open) {
      wasOpen.current = true;
      return;
    }
    if (!wasOpen.current) {
      return;
    }
    wasOpen.current = false;
    const trigger = document.querySelector(`[aria-controls="${menuId}"]`) as HTMLElement;
    trigger.focus();
  }, [open, menuId]);

  if (!open) {
    return null;
  }

  const handlers = { onPlay, onPlayNext, onAddToQueue, onGoToAlbum, onOpenArtist };
  /* An item whose handler is not wired would close the menu and do nothing,
     so it is not offered. The menu never lies about what it can do. */
  const offered = catalogueMenuActions(messages).filter((action) => {
    if (action.id === 'play') {
      return onPlay !== undefined;
    }
    if (action.id === 'play-next') {
      return onPlayNext !== undefined;
    }
    if (action.id === 'add-to-queue') {
      return onAddToQueue !== undefined;
    }
    if (action.id === 'go-to-album') {
      return onGoToAlbum !== undefined;
    }
    return true;
  });

  return createPortal(
    <View
      dataSet={{ itemMenu: '1', contextMenu: '1', menuId }}
      accessibilityRole="menu"
      accessibilityLabel={messages.contextMenu}
    >
      {offered.map((action) => (
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
    </View>,
    document.getElementById('token-shell') ?? document.body,
  );
}
