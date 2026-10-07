import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import { GoToArtistMenu } from './GoToArtistMenu.tsx';

afterEach(cleanup);

test('closed menu renders nothing', () => {
  const { container } = render(
    <GoToArtistMenu
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
      open
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={onOpenArtist}
      onClose={onClose}
    />,
  );
  const item = screen.getByRole('menuitem', { name: 'Go to artist' });
  expect(document.querySelector('[data-item-menu="1"]')).toBeTruthy();
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

test('optional catalogue handlers stay quiet when omitted', () => {
  const onClose = vi.fn();
  render(
    <GoToArtistMenu
      open
      artistKey="mira-sol"
      messages={destinationMessages()}
      onOpenArtist={vi.fn()}
      onClose={onClose}
    />,
  );
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Play next' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Add to queue' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Go to album' }));
  expect(onClose).toHaveBeenCalledTimes(4);
});
