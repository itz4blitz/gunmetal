import { cleanup, render } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { Icon } from './Icon.tsx';

afterEach(cleanup);

test('a decorative icon is a hidden 20px stroked svg on the 24 grid', () => {
  const { container } = render(<Icon name="home" />);
  const svg = container.querySelector('svg');
  expect(svg?.getAttribute('data-icon')).toStrictEqual('home');
  expect(svg?.getAttribute('viewBox')).toStrictEqual('0 0 24 24');
  expect(svg?.getAttribute('width')).toStrictEqual('20');
  expect(svg?.getAttribute('height')).toStrictEqual('20');
  expect(svg?.getAttribute('fill')).toStrictEqual('none');
  expect(svg?.getAttribute('stroke')).toStrictEqual('currentColor');
  expect(svg?.getAttribute('stroke-linecap')).toStrictEqual('round');
  expect(svg?.getAttribute('stroke-linejoin')).toStrictEqual('round');
  expect(svg?.getAttribute('aria-hidden')).toStrictEqual('true');
  expect(svg?.hasAttribute('role')).toStrictEqual(false);
  expect(svg?.hasAttribute('aria-label')).toStrictEqual(false);
  // One path, and nothing inside the icon that could carry style or script.
  expect(svg?.children.length).toStrictEqual(1);
  expect(svg?.querySelector('path')?.getAttribute('d')).toStrictEqual(
    'M3 10.6 12 3l9 7.6V20a1 1 0 0 1-1 1h-5v-6.5H9V21H4a1 1 0 0 1-1-1z',
  );
  expect(svg?.querySelector('style')).toStrictEqual(null);
  expect(svg?.hasAttribute('style')).toStrictEqual(false);
});

test('a labelled icon is an image with its name and takes the asked size', () => {
  const { container } = render(<Icon name="close" label="Close" size={24} />);
  const svg = container.querySelector('svg');
  expect(svg?.getAttribute('data-icon')).toStrictEqual('close');
  expect(svg?.getAttribute('role')).toStrictEqual('img');
  expect(svg?.getAttribute('aria-label')).toStrictEqual('Close');
  expect(svg?.hasAttribute('aria-hidden')).toStrictEqual(false);
  expect(svg?.getAttribute('width')).toStrictEqual('24');
  expect(svg?.getAttribute('height')).toStrictEqual('24');
  expect(svg?.querySelector('path')?.getAttribute('d')).toStrictEqual('M6 6l12 12 M18 6 6 18');
});

test('the watch icon is a screen with a play mark', () => {
  const { container } = render(<Icon name="watch" />);
  expect(container.querySelector('svg')?.getAttribute('data-icon')).toStrictEqual('watch');
  expect(container.querySelector('path')?.getAttribute('d')).toStrictEqual(
    'M4 5h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z M8 20.5h8 M10 8.5v5l4.5-2.5z',
  );
});
