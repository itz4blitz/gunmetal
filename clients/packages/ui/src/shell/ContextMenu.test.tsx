import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { destinationMessages } from '../messages/en/destinations.ts';
import { ContextMenu } from './ContextMenu.tsx';
import { catalogueMenuActions } from './menu-actions.ts';

afterEach(cleanup);

test('context menu stays unmounted until open and lists every catalogue action as Text', () => {
  const closed = render(
    <ContextMenu
      open={false}
      label="Actions"
      actions={catalogueMenuActions(destinationMessages())}
      onClose={vi.fn()}
      onAction={vi.fn()}
    />,
  );
  expect(document.querySelector('[data-context-menu="1"]')).toBeNull();
  expect(screen.queryByRole('menu')).toBeNull();
  closed.unmount();

  const onClose = vi.fn();
  const onAction = vi.fn();
  const { container } = render(
    <ContextMenu
      open
      label="Actions"
      actions={catalogueMenuActions(destinationMessages())}
      onClose={onClose}
      onAction={onAction}
    />,
  );
  const menu = screen.getByRole('menu', { name: 'Actions' });
  expect(menu.getAttribute('data-context-menu')).toStrictEqual('1');
  const items = screen.getAllByRole('menuitem');
  expect(items.map((item) => item.getAttribute('aria-label'))).toStrictEqual([
    'Play',
    'Play next',
    'Add to queue',
    'Go to album',
    'Go to artist',
  ]);
  expect(
    [...container.querySelectorAll('[data-menu-label="1"]')].map((node) => node.textContent),
  ).toStrictEqual(['Play', 'Play next', 'Add to queue', 'Go to album', 'Go to artist']);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  expect(onAction).toHaveBeenCalledWith('play-next');
  expect(onClose).toHaveBeenCalledTimes(1);
});

test('Escape closes the open menu and other keys leave it', () => {
  const onClose = vi.fn();
  render(
    <ContextMenu
      open
      label="Actions"
      actions={catalogueMenuActions(destinationMessages())}
      onClose={onClose}
      onAction={vi.fn()}
    />,
  );
  fireEvent.keyDown(window, { key: 'Tab' });
  expect(onClose).toHaveBeenCalledTimes(0);
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(onClose).toHaveBeenCalledTimes(1);
});

test('menu items activate on Enter and Space and ignore other keys', () => {
  const onClose = vi.fn();
  const onAction = vi.fn();
  render(
    <ContextMenu
      open
      label="Actions"
      actions={catalogueMenuActions(destinationMessages())}
      onClose={onClose}
      onAction={onAction}
    />,
  );
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Add to queue' }), { key: 'Enter' });
  expect(onAction).toHaveBeenCalledWith('add-to-queue');
  expect(onClose).toHaveBeenCalledTimes(1);
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to album' }), { key: ' ' });
  expect(onAction).toHaveBeenCalledWith('go-to-album');
  expect(onClose).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to artist' }), { key: 'Tab' });
  expect(onAction).toHaveBeenCalledTimes(2);
  expect(onClose).toHaveBeenCalledTimes(2);
});
