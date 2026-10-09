import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import { GoToArtistMenu } from './GoToArtistMenu.tsx';

afterEach(cleanup);

test('closed menu renders nothing', () => {
  const { container } = render(
    <GoToArtistMenu
      menuId="menu-1"
      open={false}
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  expect(container.querySelector('[data-item-menu="1"]')).toBeNull();
  expect(screen.queryByRole('menuitem', { name: 'Go to artist' })).toBeNull();
});

test('go to artist activates by pointer and keyboard and escape only closes', () => {
  const onOpenArtist = vi.fn();
  const onClose = vi.fn();
  render(
    <GoToArtistMenu
      menuId="menu-1"
      open
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={onOpenArtist}
      onClose={onClose}
    />,
  );
  const item = screen.getByRole('menuitem', { name: 'Go to artist' });
  expect(document.querySelector('[data-item-menu="1"]')).not.toBeNull();
  expect(item.getAttribute('data-go-to-artist')).toStrictEqual('1');
  fireEvent.click(item);
  expect(onOpenArtist).toHaveBeenCalledWith('mira-sol');
  expect(onClose).toHaveBeenCalledTimes(1);

  fireEvent.keyDown(item, { key: 'Enter' });
  expect(onOpenArtist).toHaveBeenCalledTimes(2);
  fireEvent.keyDown(item, { key: ' ' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(item, { key: 'Tab' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
  expect(onClose).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
  expect(onClose).toHaveBeenCalledTimes(4);
  fireEvent.keyDown(item, { key: 'Escape' });
  expect(onOpenArtist).toHaveBeenCalledTimes(3);
  expect(onClose).toHaveBeenCalledTimes(5);
});

test('catalogue actions invoke optional handlers when they are provided', () => {
  const onPlay = vi.fn();
  const onPlayNext = vi.fn();
  const onAddToQueue = vi.fn();
  const onGoToAlbum = vi.fn();
  const onOpenArtist = vi.fn();
  const onClose = vi.fn();
  render(
    <GoToArtistMenu
      menuId="menu-1"
      open
      artistKey="keratin"
      messages={destinationMessages()}
      onOpenArtist={onOpenArtist}
      onPlay={onPlay}
      onPlayNext={onPlayNext}
      onAddToQueue={onAddToQueue}
      onGoToAlbum={onGoToAlbum}
      onClose={onClose}
    />,
  );
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onPlay).toHaveBeenCalledTimes(1);
  expect(onPlayNext).toHaveBeenCalledTimes(1);
  expect(onAddToQueue).toHaveBeenCalledTimes(1);
  expect(onGoToAlbum).toHaveBeenCalledTimes(1);
  expect(onOpenArtist).not.toHaveBeenCalled();
  expect(onClose).toHaveBeenCalledTimes(4);

  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Play' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Play next' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Add to queue' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Go to album' }), { key: ' ' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Play' }), { key: 'Tab' });
  expect(onPlay).toHaveBeenCalledTimes(2);
  expect(onPlayNext).toHaveBeenCalledTimes(2);
  expect(onAddToQueue).toHaveBeenCalledTimes(2);
  expect(onGoToAlbum).toHaveBeenCalledTimes(2);
  expect(onClose).toHaveBeenCalledTimes(8);
});

test('items whose handlers are omitted are not offered at all', () => {
  const onClose = vi.fn();
  render(
    <GoToArtistMenu
      menuId="menu-1"
      open
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={vi.fn()}
      onClose={onClose}
    />,
  );
  // Only what is wired: go to artist. A dead item would close the menu and
  // do nothing, which is worse than not showing it.
  expect(screen.getAllByRole('menuitem').map((item) => item.textContent)).toStrictEqual(['Go to artist']);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to artist' }));
  expect(onClose).toHaveBeenCalledTimes(1);
});

test('an open menu is placed inside the viewport from its anchor, and a closed one is not', () => {
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1200 });
  Object.defineProperty(window, 'innerHeight', { configurable: true, value: 800 });
  // jsdom lays nothing out; the placement measures the menu, so the test
  // lends it the size a real menu has (196 × 190 from the sheets).
  const real = HTMLElement.prototype.getBoundingClientRect;
  HTMLElement.prototype.getBoundingClientRect = function () {
    return { width: 196, height: 190, left: 0, top: 0, right: 196, bottom: 190, x: 0, y: 0, toJSON: () => ({}) };
  };
  try {
    const placed = render(
      <GoToArtistMenu
        menuId="menu-place"
        open
        artistKey="mira-sol"
        messages={destinationMessages()}
        onOpenArtist={vi.fn()}
        onClose={vi.fn()}
        at={{ x: 20, y: 700 }}
      />,
    );
    const menu = document.querySelector('[data-item-menu="1"]') as HTMLElement;
    // The anchor is the pointer at (20, 700): the menu's top-left lands
    // there; 700 + 190 would pass the bottom edge (792), so the menu flips
    // above the pointer, resting on it.
    expect(menu.style.getPropertyValue('left')).toStrictEqual('20px');
    expect(menu.style.getPropertyValue('top')).toStrictEqual('510px');
    // The portalled menu takes focus so keyboard users are inside it.
    expect((document.activeElement as HTMLElement).getAttribute('data-menu-item')).toStrictEqual('go-to-artist');
    placed.unmount();
  } finally {
    HTMLElement.prototype.getBoundingClientRect = real;
  }
  // A closed menu renders nothing and writes nothing.
  render(
    <GoToArtistMenu
      menuId="menu-place"
      open={false}
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={vi.fn()}
      onClose={vi.fn()}
      at={{ x: 20, y: 700 }}
    />,
  );
  expect(document.querySelector('[data-item-menu="1"]')).toBeNull();
});

test('closing hands focus back to the trigger the menu opened from', () => {
  const view = render(
    <GoToArtistMenu
      menuId="menu-focus"
      open={false}
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  // The trigger the opener rendered beside the menu.
  const trigger = document.createElement('button');
  trigger.setAttribute('aria-controls', 'menu-focus');
  document.body.append(trigger);
  // Open, then close: focus lands on the opener, not lost to the void.
  view.rerender(
    <GoToArtistMenu
      menuId="menu-focus"
      open
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  view.rerender(
    <GoToArtistMenu
      menuId="menu-focus"
      open={false}
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  expect(document.activeElement).toStrictEqual(trigger);
  trigger.remove();
});
