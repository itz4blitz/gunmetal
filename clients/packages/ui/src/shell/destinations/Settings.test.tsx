import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import { destinationMessages } from '../../messages/en/destinations.ts';
import { pluginSlots } from '../../../../fake-server/src/plugin-slots.ts';
import { shellMessages } from '../../messages/en/shell.ts';
import type { ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';
import { Settings } from './Settings.tsx';

afterEach(cleanup);

const here = dirname(fileURLToPath(import.meta.url));

function renderSettings(width: WidthClass, onThemeChange: (theme: ThemeId) => void = () => {}) {
  return render(
    <Settings
      pluginSlots={pluginSlots()}
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
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual('stack');
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
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual('stack');
  expect(document.querySelector('#settings-nav')).toBeNull();
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toStrictEqual(6);
});

test('expanded and wide settings put a left list beside one selected pane', () => {
  const expanded = renderSettings('expanded');
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual('side');
  expect(screen.getByRole('tablist', { name: 'Settings sections' }).id).toStrictEqual('settings-nav');
  expect(document.querySelector('#settings-nav-appearance')?.getAttribute('data-selected')).toStrictEqual('1');
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toStrictEqual(1);
  expect(document.querySelector('#settings-appearance')).toBeTruthy();
  expect(document.querySelector('#settings-privacy')).toBeNull();
  expanded.unmount();

  renderSettings('wide');
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual('side');
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
  expect(document.querySelector('#settings-appearance [data-settings-badge="R1"]')?.textContent).toStrictEqual('R1');
  expect(document.querySelector('#settings-playback [data-settings-badge="R1"]')?.textContent).toStrictEqual('R1');
  expect(document.querySelector('#settings-about [data-settings-badge="R1"]')?.textContent).toStrictEqual('R1');
  expect(document.querySelector('#settings-privacy [data-settings-badge="R1"]')?.textContent).toStrictEqual('R1');
  expect(document.querySelector('#settings-connected [data-settings-badge="R2"]')?.textContent).toStrictEqual('R2');
  expect(document.querySelector('#settings-extensions [data-settings-badge="R2"]')?.textContent).toStrictEqual('R2');
  expect(document.querySelector('[data-settings-placeholder="playback"]')?.textContent).toStrictEqual(
    'Gain, crossfade and output arrive with CorePort (CP-020).',
  );
  expect(document.querySelector('#settings-connected [data-empty-state="connected"]')?.textContent).toStrictEqual(
    'Scrobblers and lyrics lookup arrive as signed plugins in R2.',
  );
  expect(document.querySelector('#settings-connected [data-empty-mark="1"]')).toBeTruthy();
  expect(document.querySelector('#settings-extensions [data-empty-state="extensions"]')?.textContent).toStrictEqual(
    'Plugins run as WebAssembly with per-grant consent; none load in this build.',
  );
  expect(document.querySelector('#settings-extensions [data-empty-mark="1"]')).toBeTruthy();
  expect(document.querySelector('#settings-plugin-slots')).toBeTruthy();
  expect(
    [...document.querySelectorAll('#settings-plugin-slots [data-plugin-slot]')].map((node) => [
      node.getAttribute('data-plugin-slot'),
      node.getAttribute('data-slot-plane'),
      node.getAttribute('data-slot-loaded'),
      node.querySelector('[data-slot-title]')?.textContent,
      node.querySelector('[data-slot-plane]')?.textContent,
      node.querySelector('[data-slot-state]')?.textContent,
    ]),
  ).toStrictEqual([
    ['metadata-provider', 'server', '0', 'Metadata and artwork', 'Server', 'Not loaded'],
    ['lyrics-provider', 'server', '0', 'Lyrics lookup', 'Server', 'Not loaded'],
    ['search-provider', 'server', '0', 'Catalogue search', 'Server', 'Not loaded'],
    ['scrobbler', 'server', '0', 'Scrobblers', 'Server', 'Not loaded'],
    ['theme-pack', 'client', '0', 'Themes', 'Client', 'Not loaded'],
    ['home-row', 'client', '0', 'Home rows', 'Client', 'Not loaded'],
  ]);
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

test('the appearance pane previews all four themes and selects through the same callback', () => {
  const picked: ThemeId[] = [];
  renderSettings('compact', (theme) => {
    picked.push(theme);
  });
  const strip = document.querySelector('#settings-theme-preview');
  expect(strip).toBeTruthy();
  const cards = [...document.querySelectorAll('#settings-theme-preview [data-theme-swatch]')];
  expect(cards.map((card) => card.id)).toStrictEqual([
    'settings-theme-swatch-dark',
    'settings-theme-swatch-light',
    'settings-theme-swatch-oled',
    'settings-theme-swatch-high-contrast',
  ]);
  expect(cards.map((card) => card.getAttribute('data-theme-swatch'))).toStrictEqual([
    'dark',
    'light',
    'oled',
    'high-contrast',
  ]);
  // The current theme is the selected card; the others are not.
  expect(document.querySelector('[data-theme-swatch="dark"]')?.getAttribute('data-selected')).toStrictEqual('1');
  expect(document.querySelector('[data-theme-swatch="light"]')?.getAttribute('data-selected')).toStrictEqual('0');
  // Card names carry the theme prefix from the shell catalogue, so they never
  // collide with the switcher segments' own names.
  fireEvent.click(screen.getByRole('button', { name: 'Theme: Light' }));
  fireEvent.keyDown(screen.getByRole('button', { name: 'Theme: OLED' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('button', { name: 'Theme: High contrast' }), { key: 'Tab' });
  expect(picked).toStrictEqual(['light', 'oled']);
  expect(screen.getAllByRole('button', { name: 'Theme: Dark' }).length).toStrictEqual(1);
});

test('the preview strip rides along on the side layout and stacked layouts alike', () => {
  renderSettings('expanded');
  expect(document.querySelector('#settings-appearance #settings-theme-preview')).toBeTruthy();
  expect(document.querySelectorAll('#settings-theme-preview [data-theme-swatch]').length).toStrictEqual(4);
});

// Verifies: design-language §3 (brass is scarce), §8 (focus rings), §9 (reduced motion), SEC-API-044 (no url/data)
test('area-settings.css keeps the steel-and-brass contract for settings', async () => {
  const css = await readFile(join(here, '../../../../../apps/demo/public/area-settings.css'), 'utf8');
  expect(css.length).toBeGreaterThan(0);
  // The R1 badge is steel and the R2 badge is the one brass badge.
  expect(css.includes("[data-settings-badge='R1']")).toBe(true);
  expect(css.includes("[data-settings-badge='R2']")).toBe(true);
  // Every theme has its swatch drawn from static token values; brass outlines the selected one.
  for (const id of ['dark', 'light', 'oled', 'high-contrast']) {
    expect(css.includes(`[data-theme-swatch='${id}']`)).toBe(true);
  }
  expect(css.includes("[data-theme-swatch][data-selected='1']")).toBe(true);
  // Keyboard focus rings and reduced-motion handling exist.
  expect(css.includes('outline: 2px solid var(--gm-focus-ring)')).toBe(true);
  expect(css.includes('prefers-reduced-motion: reduce')).toBe(true);
  // No external or inline assets: stylesheets stay inside the CSP (SEC-API-044).
  expect(css.includes('url(')).toBe(false);
  expect(css.includes('data:')).toBe(false);
  // Hostile corpus strings never appear in this surface.
  expect(css.toLowerCase().includes('hostile')).toBe(false);
});
