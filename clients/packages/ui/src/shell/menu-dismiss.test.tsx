import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { useMenuDismiss } from './menu-dismiss.ts';

afterEach(cleanup);

function Harness({ open, onClose }: { open: boolean; onClose: () => void }) {
  useMenuDismiss(open, 'menu-a', onClose);
  return (
    <div>
      <button type="button" aria-controls="menu-a">
        <span>Opener</span>
      </button>
      <button type="button" aria-controls="menu-b">
        Other opener
      </button>
      <div data-menu-id="menu-a">
        <span>Inside</span>
      </div>
      <div data-menu-id="menu-b">Other menu</div>
      <p>Elsewhere</p>
    </div>
  );
}

function mount(open: boolean): { closed: string[] } {
  const closed: string[] = [];
  render(
    <Harness
      open={open}
      onClose={() => {
        closed.push('close');
      }}
    />,
  );
  return { closed };
}

test('a press outside the menu and its opener closes it', () => {
  const { closed } = mount(true);
  fireEvent.pointerDown(screen.getByText('Elsewhere'));
  expect(closed).toStrictEqual(['close']);
  // Another menu and another menu's opener are outside this one.
  fireEvent.pointerDown(screen.getByText('Other menu'));
  fireEvent.pointerDown(screen.getByText('Other opener'));
  expect(closed).toStrictEqual(['close', 'close', 'close']);
  // A press that is not on an element at all (the window itself) is outside too.
  fireEvent.pointerDown(window);
  expect(closed).toStrictEqual(['close', 'close', 'close', 'close']);
});

test('a press inside the menu or on its own opener is left alone', () => {
  const { closed } = mount(true);
  fireEvent.pointerDown(screen.getByText('Inside'));
  fireEvent.pointerDown(document.querySelector('[data-menu-id="menu-a"]') as HTMLElement);
  fireEvent.pointerDown(screen.getByText('Opener'));
  fireEvent.pointerDown(document.querySelector('[aria-controls="menu-a"]') as HTMLElement);
  expect(closed).toStrictEqual([]);
});

test('the press is seen even when a control stops it from bubbling', () => {
  const { closed } = mount(true);
  const elsewhere = screen.getByText('Elsewhere');
  elsewhere.addEventListener('pointerdown', (event) => {
    event.stopPropagation();
  });
  fireEvent.pointerDown(elsewhere);
  expect(closed).toStrictEqual(['close']);
});

test('Escape closes; other keys do not', () => {
  const { closed } = mount(true);
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(closed).toStrictEqual(['close']);
  fireEvent.keyDown(window, { key: 'Enter' });
  fireEvent.keyDown(window, { key: 'Tab' });
  expect(closed).toStrictEqual(['close']);
});

test('a closed menu listens for nothing, and an unmounted one stops listening', () => {
  const idle = mount(false);
  fireEvent.pointerDown(screen.getByText('Elsewhere'));
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(idle.closed).toStrictEqual([]);
  cleanup();

  const gone = mount(true);
  cleanup();
  fireEvent.pointerDown(window);
  fireEvent.keyDown(window, { key: 'Escape' });
  expect(gone.closed).toStrictEqual([]);
});
