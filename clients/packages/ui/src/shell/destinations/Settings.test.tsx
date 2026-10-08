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
      pluginSlots={pluginSlots(true)}
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
  // Every pane has its own section heading, in catalogue order, under the page title.
  expect(screen.getAllByRole('heading').map((heading) => heading.textContent)).toStrictEqual([
    'Settings',
    'Appearance',
    'Playback',
    'Connected services',
    'Extensions / Plugins',
    'About this connection',
    'Privacy',
  ]);
  expect(
    [...document.querySelectorAll('[data-settings-panel="1"]')].map((pane) => [
      pane.id,
      pane.getAttribute('data-settings-section'),
      pane.querySelector('[data-settings-heading-row] [role="heading"]')?.getAttribute('data-type'),
    ]),
  ).toStrictEqual([
    ['settings-appearance', 'appearance', 'title2'],
    ['settings-playback', 'playback', 'title2'],
    ['settings-connected', 'connected', 'title2'],
    ['settings-extensions', 'extensions', 'title2'],
    ['settings-about', 'about', 'title2'],
    ['settings-privacy', 'privacy', 'title2'],
  ]);
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toStrictEqual(6);
  // The theme is chosen in one place: the preview cards. No second control.
  expect(document.querySelector('#theme-switcher')).toBeNull();
  expect(document.querySelector('#settings-appearance #settings-theme-preview')?.getAttribute('role')).toStrictEqual(
    'radiogroup',
  );
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
  const shownPanes = () => [...document.querySelectorAll('[data-settings-panel="1"]')].map((pane) => pane.id);
  expect(shownPanes()).toStrictEqual(['settings-appearance']);
  expect(document.querySelector('#settings-privacy')).toBeNull();
  expanded.unmount();

  renderSettings('wide');
  expect(document.querySelector('#destination-settings')?.getAttribute('data-settings-layout')).toStrictEqual('side');
  expect(screen.getByRole('tab', { name: 'Playback' }).id).toStrictEqual('settings-nav-playback');
  // The selected section is exposed to assistive technology, not only painted.
  expect(
    [...document.querySelectorAll('#settings-nav [role="tab"]')].map((tab) => tab.getAttribute('aria-selected')),
  ).toStrictEqual(['true', 'false', 'false', 'false', 'false', 'false']);
  const shown = () => [...document.querySelectorAll('[data-settings-panel="1"]')].map((pane) => pane.id);
  fireEvent.click(screen.getByRole('tab', { name: 'Privacy' }));
  expect(shown()).toStrictEqual(['settings-privacy']);
  expect(document.querySelector('#settings-appearance')).toBeNull();
  expect(document.querySelector('#settings-nav-privacy')?.getAttribute('data-selected')).toStrictEqual('1');
  expect(document.querySelector('#settings-nav-privacy')?.getAttribute('aria-selected')).toStrictEqual('true');
  expect(document.querySelector('#settings-nav-appearance')?.getAttribute('aria-selected')).toStrictEqual('false');
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Connected services' }), { key: 'Enter' });
  expect(shown()).toStrictEqual(['settings-connected']);
  fireEvent.keyDown(screen.getByRole('tab', { name: 'About this connection' }), { key: ' ' });
  expect(shown()).toStrictEqual(['settings-about']);
  fireEvent.keyDown(screen.getByRole('tab', { name: 'Extensions / Plugins' }), { key: 'Tab' });
  expect(shown()).toStrictEqual(['settings-about']);
  expect(document.querySelector('#settings-extensions')).toBeNull();
});

test('each stacked pane carries an honest R1 or R2 badge and the catalogue body', () => {
  renderSettings('compact');
  expect(document.querySelector('#settings-appearance [data-settings-badge="R1"]')?.textContent).toStrictEqual('R1');
  expect(document.querySelector('#settings-playback [data-settings-badge="R1"]')?.textContent).toStrictEqual('R1');
  expect(document.querySelector('#settings-about [data-settings-badge="R1"]')?.textContent).toStrictEqual('R1');
  expect(document.querySelector('#settings-privacy [data-settings-badge="R1"]')?.textContent).toStrictEqual('R1');
  expect(document.querySelector('#settings-connected [data-settings-badge="R2"]')?.textContent).toStrictEqual('R2');
  expect(document.querySelector('#settings-extensions [data-settings-badge]')).toBeNull();
  expect(document.querySelector('[data-settings-placeholder="playback"]')?.textContent).toStrictEqual(
    'Gain, crossfade and output arrive with CorePort (CP-020).',
  );
  expect(document.querySelector('#settings-connected [data-empty-state="connected"]')?.textContent).toStrictEqual(
    'Scrobblers and lyrics lookup arrive as signed plugins in R2.',
  );
  expect(document.querySelector('#settings-extensions [data-empty-state="extensions"]')?.textContent).toStrictEqual(
    'These jobs belong to this library. They are not plugins, and this build has no plugin host.',
  );
  // The old decorative empty marks are gone: every pane is rows now.
  expect(document.querySelectorAll('#destination-settings [data-empty-mark]').length).toStrictEqual(0);
  expect(document.querySelectorAll('#destination-settings [data-empty-card]').length).toStrictEqual(0);
  // The jobs list is rows, not a plugin table: no version column, no R2 badge, no load bit.
  expect(document.querySelectorAll('#settings-extensions #settings-plugin-slots').length).toStrictEqual(1);
  expect(document.querySelector('[data-slot-head]')).toBeNull();
  expect(document.querySelector('[data-slot-version]')).toBeNull();
  expect(document.querySelector('#settings-extensions')?.textContent?.includes('1.0.0')).toStrictEqual(false);
  expect(document.querySelector('#settings-extensions')?.textContent?.includes('Not loaded')).toStrictEqual(false);
  expect(document.querySelector('#settings-extensions')?.textContent?.includes('R2')).toStrictEqual(false);
  expect(document.querySelector('#settings-extensions')?.textContent?.includes('WebAssembly')).toStrictEqual(false);
  expect(extensionJobs()).toStrictEqual(jobsWhileCoversAreServed());
  // About is a definition list: muted label, primary value, one row per fact.
  expect(document.querySelector('#settings-about-facts')?.getAttribute('data-settings-facts')).toStrictEqual('1');
  expect(document.querySelector('[data-settings-fact="data"] [data-fact-label]')?.textContent).toStrictEqual('Library');
  expect(document.querySelector('[data-settings-fact="data"] [data-fact-value]')?.textContent).toStrictEqual(
    'Demo data',
  );
  expect(document.querySelector('[data-settings-fact="address"] [data-fact-label]')?.textContent).toStrictEqual(
    'Address',
  );
  expect(document.querySelector('[data-settings-fact="address"] [data-fact-value]')?.textContent).toStrictEqual(
    'loopback',
  );
  expect(document.querySelector('[data-settings-fact="version"] [data-fact-label]')?.textContent).toStrictEqual(
    'Version',
  );
  expect(document.querySelector('[data-settings-fact="version"] [data-fact-value]')?.textContent).toStrictEqual('demo');
  expect(document.querySelector('[data-settings-privacy]')?.textContent).toStrictEqual(
    'History and loves stay on this profile; this demo has no server yet.',
  );
});

test('playback and connected services list what is coming as rows with an honest status, never a dead control', () => {
  renderSettings('compact');
  const rows = (pane: string) =>
    [...document.querySelectorAll(`${pane} [data-settings-row]`)].map((row) => [
      row.getAttribute('data-settings-row'),
      row.getAttribute('data-row-state'),
      row.querySelector('[data-settings-row-label]')?.textContent,
      row.querySelector('[data-settings-row-hint]')?.textContent,
      row.querySelector('[data-settings-row-status]')?.textContent,
    ]);
  expect(rows('#settings-playback')).toStrictEqual([
    [
      'levelling',
      'unavailable',
      'Volume levelling',
      'Plays tracks at a consistent loudness, from the tags in your files.',
      'Not available yet',
    ],
    [
      'crossfade',
      'unavailable',
      'Crossfade',
      'Blends the end of one track into the start of the next.',
      'Not available yet',
    ],
    [
      'output',
      'unavailable',
      'Output device',
      'Chooses the speakers or headphones this device plays through.',
      'Not available yet',
    ],
  ]);
  expect(rows('#settings-connected')).toStrictEqual([
    [
      'scrobble',
      'unavailable',
      'Scrobbling',
      'Sends what you play to a listening-history service you link yourself.',
      'Not available yet',
    ],
    ['lyrics', 'unavailable', 'Lyrics lookup', 'Finds lyrics for tracks whose files have none.', 'Not available yet'],
  ]);
  // Nothing in these panes can be pressed, toggled or focused: a setting that
  // is not wired shows its status as words.
  for (const pane of ['#settings-playback', '#settings-connected']) {
    expect(document.querySelectorAll(`${pane} [tabindex]`).length).toStrictEqual(0);
    expect(
      document.querySelectorAll(
        `${pane} [role="switch"], ${pane} [role="checkbox"], ${pane} [role="button"], ${pane} input`,
      ).length,
    ).toStrictEqual(0);
  }
});

function extensionJobs(): (string | null)[][] {
  return [...document.querySelectorAll('#settings-plugin-slots [data-settings-row]')].map((row) => [
    row.getAttribute('data-settings-row'),
    row.getAttribute('data-job-status'),
    row.querySelector('[data-slot-title]')?.textContent ?? null,
    row.querySelector('[data-job-detail]')?.textContent ?? null,
    row.querySelector('[data-settings-row-status]')?.textContent ?? null,
  ]);
}

function jobsWhileCoversAreServed(): (string | null)[][] {
  return [
    ['metadata-provider', 'on', 'Metadata and artwork', 'Built into this library host. Cover Art Archive.', 'On'],
    ['lyrics-provider', 'not-in-build', 'Lyrics lookup', null, 'Not in this build'],
    ['search-provider', 'not-in-build', 'Catalogue search', null, 'Not in this build'],
    ['scrobbler', 'not-in-build', 'Scrobblers', null, 'Not in this build'],
    [
      'theme-pack',
      'not-a-plugin',
      'Themes',
      'Not a separate plugin. Themes are the settings appearance control.',
      null,
    ],
    ['home-row', 'not-a-plugin', 'Home rows', 'Not a separate plugin.', null],
  ];
}

test('with no plugin slots to list, the extensions pane keeps its statement and draws no empty table', () => {
  render(
    <Settings
      messages={destinationMessages()}
      shellMessages={shellMessages()}
      theme="dark"
      onThemeChange={() => {}}
      width="compact"
    />,
  );
  expect(document.querySelector('#settings-extensions [data-empty-state="extensions"]')?.textContent).toStrictEqual(
    'These jobs belong to this library. They are not plugins, and this build has no plugin host.',
  );
  expect(document.querySelector('#settings-plugin-slots')).toBeNull();
  expect(document.querySelector('[data-slot-head]')).toBeNull();
  expect(document.querySelectorAll('#settings-extensions [data-settings-row]').length).toStrictEqual(0);
  expect(document.querySelector('#settings-extensions [data-settings-badge]')).toBeNull();
});

test('when the host is not serving covers, metadata stays built in and does not read On or Not loaded', () => {
  render(
    <Settings
      pluginSlots={pluginSlots(false)}
      messages={destinationMessages()}
      shellMessages={shellMessages()}
      theme="dark"
      onThemeChange={() => {}}
      width="compact"
    />,
  );
  expect(extensionJobs()).toStrictEqual([
    [
      'metadata-provider',
      'not-serving',
      'Metadata and artwork',
      'Built into this library host. Cover Art Archive.',
      null,
    ],
    ['lyrics-provider', 'not-in-build', 'Lyrics lookup', null, 'Not in this build'],
    ['search-provider', 'not-in-build', 'Catalogue search', null, 'Not in this build'],
    ['scrobbler', 'not-in-build', 'Scrobblers', null, 'Not in this build'],
    [
      'theme-pack',
      'not-a-plugin',
      'Themes',
      'Not a separate plugin. Themes are the settings appearance control.',
      null,
    ],
    ['home-row', 'not-a-plugin', 'Home rows', 'Not a separate plugin.', null],
  ]);
  expect(document.querySelector('#settings-extensions')?.textContent?.includes('Not loaded')).toStrictEqual(false);
  expect(document.querySelector('#settings-extensions')?.textContent?.includes('1.0.0')).toStrictEqual(false);
  expect(document.querySelector('[data-slot-version]')).toBeNull();
  expect(document.querySelector('#settings-extensions [data-settings-badge]')).toBeNull();
});

test('a route section of extensions shows that pane, and a later route section follows it', () => {
  const view = render(
    <Settings
      section="extensions"
      pluginSlots={pluginSlots(true)}
      messages={destinationMessages()}
      shellMessages={shellMessages()}
      theme="dark"
      onThemeChange={() => {}}
      width="wide"
    />,
  );
  expect(document.querySelector('#settings-extensions')).not.toBeNull();
  expect(document.querySelector('#settings-appearance')).toBeNull();
  expect(document.querySelector('#settings-nav-extensions')?.getAttribute('aria-selected')).toStrictEqual('true');
  expect(extensionJobs()).toStrictEqual(jobsWhileCoversAreServed());
  fireEvent.click(screen.getByRole('tab', { name: 'Appearance' }));
  expect(document.querySelector('#settings-appearance')).not.toBeNull();
  expect(document.querySelector('#settings-extensions')).toBeNull();
  view.rerender(
    <Settings
      section="privacy"
      pluginSlots={pluginSlots(true)}
      messages={destinationMessages()}
      shellMessages={shellMessages()}
      theme="dark"
      onThemeChange={() => {}}
      width="wide"
    />,
  );
  expect(document.querySelector('#settings-privacy')).not.toBeNull();
  expect(document.querySelector('#settings-appearance')).toBeNull();
  view.rerender(
    <Settings
      pluginSlots={pluginSlots(true)}
      messages={destinationMessages()}
      shellMessages={shellMessages()}
      theme="dark"
      onThemeChange={() => {}}
      width="wide"
    />,
  );
  expect(document.querySelector('#settings-appearance')).not.toBeNull();
  expect(document.querySelector('#settings-privacy')).toBeNull();
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

test('the theme is one radio group of five preview cards, with the current theme checked', () => {
  renderSettings('compact');
  const group = screen.getByRole('radiogroup', { name: 'Theme' });
  expect(group.id).toStrictEqual('settings-theme-preview');
  const cards = [...group.querySelectorAll('[data-theme-swatch]')];
  expect(
    cards.map((card) => [
      card.id,
      card.getAttribute('data-theme-swatch'),
      card.getAttribute('role'),
      card.getAttribute('aria-label'),
      card.getAttribute('aria-checked'),
      card.getAttribute('data-selected'),
      // Roving tabindex: the checked card is the group's one tab stop.
      card.getAttribute('tabindex'),
      card.querySelector('[data-swatch-name]')?.textContent,
    ]),
  ).toStrictEqual([
    ['settings-theme-swatch-system', 'system', 'radio', 'System', 'false', '0', '-1', 'System'],
    ['settings-theme-swatch-dark', 'dark', 'radio', 'Dark', 'true', '1', '0', 'Dark'],
    ['settings-theme-swatch-light', 'light', 'radio', 'Light', 'false', '0', '-1', 'Light'],
    ['settings-theme-swatch-oled', 'oled', 'radio', 'OLED', 'false', '0', '-1', 'OLED'],
    [
      'settings-theme-swatch-high-contrast',
      'high-contrast',
      'radio',
      'High contrast',
      'false',
      '0',
      '-1',
      'High contrast',
    ],
  ]);
  // The group explains itself in words from the catalogue.
  expect(document.querySelector('#settings-appearance [data-settings-group-title]')?.textContent).toStrictEqual(
    'Theme',
  );
  expect(document.querySelector('#settings-appearance [data-settings-group-hint]')?.textContent).toStrictEqual(
    'System follows this device, with Dark as the fallback. OLED uses true black, and High contrast strengthens every edge.',
  );
});

test('with System chosen, the system card is the group\u2019s one tab stop', () => {
  render(
    <Settings
      messages={destinationMessages()}
      shellMessages={shellMessages()}
      theme="system"
      onThemeChange={() => {}}
      width="compact"
    />,
  );
  expect(screen.getByRole('radio', { name: 'System' }).getAttribute('aria-checked')).toStrictEqual('true');
  expect(screen.getByRole('radio', { name: 'System' }).getAttribute('tabindex')).toStrictEqual('0');
  expect(screen.getByRole('radio', { name: 'Dark' }).getAttribute('aria-checked')).toStrictEqual('false');
  expect(screen.getByRole('radio', { name: 'Dark' }).getAttribute('tabindex')).toStrictEqual('-1');
});

test('a card is chosen by pointer, Enter or Space and reports the theme to the shell', () => {
  const picked: ThemeId[] = [];
  renderSettings('compact', (theme) => {
    picked.push(theme);
  });
  fireEvent.click(screen.getByRole('radio', { name: 'Light' }));
  fireEvent.keyDown(screen.getByRole('radio', { name: 'OLED' }), { key: 'Enter' });
  fireEvent.keyDown(screen.getByRole('radio', { name: 'High contrast' }), { key: ' ' });
  // Keys that are not a choice do nothing.
  fireEvent.keyDown(screen.getByRole('radio', { name: 'High contrast' }), { key: 'Tab' });
  fireEvent.keyDown(screen.getByRole('radio', { name: 'Light' }), { key: 'a' });
  expect(picked).toStrictEqual(['light', 'oled', 'high-contrast']);
  expect(screen.getAllByRole('radio', { name: 'Dark' }).length).toStrictEqual(1);
});

test('arrow keys move the choice and the focus through the radio group, wrapping at the ends', () => {
  const picked: ThemeId[] = [];
  renderSettings('compact', (theme) => {
    picked.push(theme);
  });
  const dark = screen.getByRole('radio', { name: 'Dark' });
  // The harness keeps the theme at dark, so each arrow steps from dark.
  fireEvent.keyDown(dark, { key: 'ArrowRight' });
  expect(document.activeElement?.id).toStrictEqual('settings-theme-swatch-light');
  fireEvent.keyDown(dark, { key: 'ArrowLeft' });
  expect(document.activeElement?.id).toStrictEqual('settings-theme-swatch-system');
  fireEvent.keyDown(dark, { key: 'ArrowDown' });
  expect(document.activeElement?.id).toStrictEqual('settings-theme-swatch-light');
  fireEvent.keyDown(dark, { key: 'ArrowUp' });
  expect(document.activeElement?.id).toStrictEqual('settings-theme-swatch-system');
  expect(picked).toStrictEqual(['light', 'system', 'light', 'system']);
  // Home is not a radio-group key here: nothing is chosen and focus stays.
  fireEvent.keyDown(dark, { key: 'Home' });
  expect(picked.length).toStrictEqual(4);
  expect(document.activeElement?.id).toStrictEqual('settings-theme-swatch-system');
});

test('the preview cards ride along on the side layout and stacked layouts alike', () => {
  renderSettings('expanded');
  expect(document.querySelector('#settings-appearance #settings-theme-preview')?.getAttribute('role')).toStrictEqual(
    'radiogroup',
  );
  expect(document.querySelectorAll('#settings-theme-preview [data-theme-swatch]').length).toStrictEqual(5);
});

// Verifies: design-language §3 (brass is scarce), §8 (focus rings), §9 (reduced motion), SEC-API-044 (no url/data)
test('area-settings.css keeps the steel-and-brass contract for settings', async () => {
  const css = await readFile(join(here, '../../../../../apps/demo/public/area-settings.css'), 'utf8');
  expect(css.length).toBeGreaterThan(0);
  // Release badges are steel tags; brass is kept for where you are.
  expect(css.includes("[data-settings-badge='R1']")).toBe(true);
  expect(css.includes("[data-settings-badge='R2']")).toBe(true);
  const badges = css.slice(css.indexOf('[data-settings-badge]'), css.indexOf('/* —— Section list'));
  expect(badges.length).toBeGreaterThan(0);
  expect(badges.includes('--gm-accent')).toBe(false);
  // Every theme choice has its swatch drawn from static token values; brass outlines the selected one.
  for (const id of ['system', 'dark', 'light', 'oled', 'high-contrast']) {
    expect(css.includes(`[data-theme-swatch='${id}']`)).toBe(true);
  }
  // The system card shows both faces honestly: a dark half and a light half.
  const systemSwatch = css.slice(css.indexOf("[data-theme-swatch='system'] [data-swatch-stage]"));
  expect(systemSwatch.includes('linear-gradient(90deg')).toBe(true);
  expect(css.includes("[data-theme-swatch][data-selected='1']")).toBe(true);
  // A loaded plugin slot is the one place a status colour earns its keep;
  // "Not loaded" stays steel because nothing is wrong.
  expect(css.includes('var(--gm-status-success)')).toBe(true);
  // Keyboard focus rings and reduced-motion handling exist.
  expect(css.includes('outline: 2px solid var(--gm-focus-ring)')).toBe(true);
  expect(css.includes('prefers-reduced-motion: reduce')).toBe(true);
  // Layout pass: panes are flat and their content sits in raised row groups;
  // panes enter on the standard curve and swatches lift on hover — all dying
  // instantly under reduced motion.
  expect(css.includes('[data-settings-rows]')).toBe(true);
  expect(css.includes('[data-settings-row] + [data-settings-row]')).toBe(true);
  expect(css.includes('gm-settings-pane-in')).toBe(true);
  expect(css.includes('translateY(-2px)')).toBe(true);
  expect(css.includes('animation: none')).toBe(true);
  // The layout answers the width of its own column, not the window: the
  // sidebar and the queue are resizable, so no rule may assume a fixed width.
  expect(css.includes('container-type: inline-size')).toBe(true);
  expect(css.includes('@container')).toBe(true);
  expect(css.includes('vw')).toBe(false);
  // Every hexagon is the shared nut; no surface redraws it.
  expect(css.includes('clip-path: var(--gm-nut)')).toBe(true);
  expect(css.includes('polygon(')).toBe(false);
  // Machined radii only — never pills, never blur.
  expect(css.includes('999px')).toBe(false);
  expect(css.includes('backdrop-filter')).toBe(false);
  // About reads as a definition list with tabular figures on the values.
  expect(css.includes('[data-fact-label]')).toBe(true);
  expect(css.includes('[data-fact-value]')).toBe(true);
  expect(css.includes('tabular-nums')).toBe(true);
  // No external or inline assets: stylesheets stay inside the CSP (SEC-API-044).
  expect(css.includes('url(')).toBe(false);
  expect(css.includes('data:')).toBe(false);
  // Hostile corpus strings never appear in this surface.
  expect(css.toLowerCase().includes('hostile')).toBe(false);
});
