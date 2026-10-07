import { useEffect } from 'react';

/**
 * Closes an open menu on Escape and on a press anywhere outside it.
 *
 * "Outside" is everything except the menu itself (the element whose
 * data-menu-id is `menuId`) and its own opener (the control whose
 * aria-controls names it). A press on the opener is left to that control, so
 * a second click on it toggles the menu shut instead of closing it here and
 * re-opening it there. The press is watched in the capture phase, so a
 * control that stops the event from bubbling still dismisses the menu.
 */
export function useMenuDismiss(open: boolean, menuId: string, onClose: () => void): void {
  useEffect(() => {
    if (!open) {
      return;
    }
    const own = `[data-menu-id="${menuId}"], [aria-controls="${menuId}"]`;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };
    const onPress = (event: Event) => {
      const target = event.target;
      if (target instanceof Element && target.closest(own) !== null) {
        return;
      }
      onClose();
    };
    globalThis.addEventListener('keydown', onKey);
    globalThis.addEventListener('pointerdown', onPress, true);
    return () => {
      globalThis.removeEventListener('keydown', onKey);
      globalThis.removeEventListener('pointerdown', onPress, true);
    };
  }, [open, menuId, onClose]);
}
