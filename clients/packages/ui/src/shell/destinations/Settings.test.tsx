import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import { shellMessages } from '../../messages/en/shell.ts';
import type { ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';
import { Settings } from './Settings.tsx';

afterEach(cleanup);

const here = dirname(fileURLToPath(import.meta.url));

function renderSettings(width: WidthClass, onThemeChange: (theme: ThemeId) => void = () => {}) {
  return render(
    <Settings
      messages={destinationMessages()}
      shellMessages={shellMessages()}
      theme="dark"
      onThemeChange={onThemeChange}
      width={width}
    />,
  );
}

test('compact settings stacks every pane and hides the section list', () => {
  renderSettings('compact');
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual(
    'stack',
  );
  expect(document.querySelector('#settings-nav')).toBeNull();
  expect(screen.getByRole('heading', { name: 'Settings' }).id).toStrictEqual('destination-headline');
  expect(screen.getByRole('heading', { name: 'Appearance' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Playback' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Connected services' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Extensions / Plugins' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'About this connection' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Privacy' })).toBeTruthy();
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toStrictEqual(6);
  expect(document.querySelector('#settings-appearance #theme-switcher')).toBeTruthy();
});

test('medium settings stay stacked like compact', () => {
  renderSettings('medium');
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual(
    'stack',
  );
  expect(document.querySelector('#settings-nav')).toBeNull();
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toStrictEqual(6);
});

test('expanded and wide settings put a left list beside one selected pane', () => {
  const expanded = renderSettings('expanded');
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual(
    'side',
  );
  expect(screen.getByRole('tablist', { name: 'Settings sections' }).id).toStrictEqual('settings-nav');
  expect(document.querySelector('#settings-nav-appearance')?.getAttribute('data-selected')).toStrictEqual(
    '1',
  );
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toStrictEqual(1);
  expect(document.querySelector('#settings-appearance')).toBeTruthy();
  expect(document.querySelector('#settings-privacy')).toBeNull();
  expanded.unmount();

  renderSettings('wide');
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual(
    'side',
  );
  expect(screen.getByRole('tab', { name: 'Playback' }).id).toStrictEqual('settings-nav-playback');
  fireEvent.click(screen.getByRole('tab', { name: 'Privacy' }));
  expect(document.querySelector('#settings-privacy')).toBeTruthy();
  expect(document.querySelector('#settings-appearance')).toBeNull();
  expect(document.querySelector('#settings-nav-privacy')?.getAttribute('data-selected')).toStrictEqual('1');
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Connected services' }), { key: 'Enter' });
  expect(document.querySelector('#settings-connected')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('tab', { name: 'About this connection' }), { key: ' ' });
  expect(document.querySelector('#settings-about')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Extensions / Plugins' }), { key: 'Tab' });
  expect(document.querySelector('#settings-about')).toBeTruthy();
  expect(document.querySelector('#settings-extensions')).toBeNull();
});

test('each stacked pane carries an honest R1 or R2 badge and the catalogue body', () => {
  renderSettings('compact');
  expect(document.querySelector('#settings-appearance [data-settings-badge="R1"]')?.textContent).toStrictEqual(
    'R1',
  );
  expect(document.querySelector('#settings-playback [data-settings-badge="R1"]')?.textContent).toStrictEqual(
    'R1',
  );
  expect(document.querySelector('#settings-about [data-settings-badge="R1"]')?.textContent).toStrictEqual(
    'R1',
  );
  expect(document.querySelector('#settings-privacy [data-settings-badge="R1"]')?.textContent).toStrictEqual(
    'R1',
  );
  expect(document.querySelector('#settings-connected [data-settings-badge="R2"]')?.textContent).toStrictEqual(
    'R2',
  );
  expect(document.querySelector('#settings-extensions [data-settings-badge="R2"]')?.textContent).toStrictEqual(
    'R2',
  );
  expect(
    document.querySelector('[data-settings-placeholder="playback"]')?.textContent,
  ).toStrictEqual('Gain, crossfade and output arrive with CorePort (CP-020).');
  expect(document.querySelector('#settings-connected [data-empty-state="connected"]')?.textContent).toStrictEqual(
    'Scrobblers and lyrics lookup arrive as signed plugins in R2.',
  );
  expect(document.querySelector('#settings-connected [data-empty-mark="1"]')).toBeTruthy();
  expect(
    document.querySelector('#settings-extensions [data-empty-state="extensions"]')?.textContent,
  ).toStrictEqual('Plugins run as WebAssembly with per-grant consent; none load in this build.');
  expect(document.querySelector('#settings-extensions [data-empty-mark="1"]')).toBeTruthy();
  expect(document.querySelector('[data-settings-fact="data"]')?.textContent).toStrictEqual('Demo data');
  expect(document.querySelector('[data-settings-fact="address"]')?.textContent).toStrictEqual('loopback');
  expect(document.querySelector('[data-settings-fact="version"]')?.textContent).toStrictEqual('demo');
  expect(document.querySelector('[data-settings-privacy]')?.textContent).toStrictEqual(
    'History and loves stay on this profile; this demo has no server yet.',
  );
});

// Verifies: SEC-EXT-018, SEC-TM-065
test('the extensions pane names Wasm grants and this build loads no plugin host', async () => {
  renderSettings('compact');
  expect(document.querySelector('[data-plugin-host]')).toBeNull();
  expect(document.querySelector('#settings-extensions script')).toBeNull();
  expect(document.querySelector('#settings-extensions iframe')).toBeNull();
  expect(document.querySelector('[data-plugin-row]')).toBeNull();
  expect(document.querySelector('[data-plugin-grant]')).toBeNull();
  expect(screen.queryByRole('button', { name: /install|enable|load plugin/i })).toBeNull();
  const settingsSource = await readFile(join(here, 'Settings.tsx'), 'utf8');
  const logicSource = await readFile(join(here, 'settings.ts'), 'utf8');
  const combined = `${settingsSource}\n${logicSource}`;
  expect(combined.includes('WebAssembly')).toStrictEqual(false);
  expect(combined.includes('.wasm')).toStrictEqual(false);
  expect(combined.includes('createElement')).toStrictEqual(false);
  expect(combined.includes('loadPlugin')).toStrictEqual(false);
  expect(combined.includes('pluginHost')).toStrictEqual(false);
});

test('appearance theme segments still notify the shell', () => {
  const themes: ThemeId[] = [];
  renderSettings('compact', (theme) => {
    themes.push(theme);
  });
  fireEvent.click(screen.getByRole('button', { name: 'Light' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'OLED' }), { key: 'Enter' });
  expect(themes).toStrictEqual(['light', 'oled']);
});
