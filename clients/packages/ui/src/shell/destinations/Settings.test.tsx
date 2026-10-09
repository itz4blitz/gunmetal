import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
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
    ['settings-about', 'about', 'title2'],
    ['settings-privacy', 'privacy', 'title2'],
  ]);
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toStrictEqual(5);
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
  expect(document.querySelectorAll('[data-settings-panel="1"]').length).toStrictEqual(5);
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
  ).toStrictEqual(['true', 'false', 'false', 'false', 'false']);
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
});

test('each stacked pane shows the catalogue body and no release badge', () => {
  const messages = destinationMessages();
  renderSettings('compact');
  // Release badges are not a product promise. No pane renders one.
  expect(document.querySelectorAll('#destination-settings [data-settings-badge]').length).toStrictEqual(0);
  expect(document.querySelector('[data-settings-placeholder="playback"]')?.textContent).toStrictEqual(
    messages.settingsPlaybackPlaceholder,
  );
  expect(document.querySelector('#settings-connected [data-empty-state="connected"]')?.textContent).toStrictEqual(
    messages.settingsConnectedEmpty,
  );
  // The old decorative empty marks are gone: every pane is rows now.
  expect(document.querySelectorAll('#destination-settings [data-empty-mark]').length).toStrictEqual(0);
  expect(document.querySelectorAll('#destination-settings [data-empty-card]').length).toStrictEqual(0);
  // About is a definition list: muted label, primary value, one row per fact.
  expect(document.querySelector('#settings-about-facts')?.getAttribute('data-settings-facts')).toStrictEqual('1');
  expect(document.querySelector('[data-settings-fact="data"] [data-fact-label]')?.textContent).toStrictEqual('Library');
  // Unset libraryFact keeps the catalogue sentence, whatever the message agent set it to.
  expect(document.querySelector('[data-settings-fact="data"] [data-fact-value]')?.textContent).toStrictEqual(
    messages.settingsAboutData,
  );
  expect(document.querySelector('[data-settings-fact="address"] [data-fact-label]')?.textContent).toStrictEqual(
    'Address',
  );
  expect(document.querySelector('[data-settings-fact="address"] [data-fact-value]')?.textContent).toStrictEqual(
    messages.settingsAboutAddress,
  );
  expect(document.querySelector('[data-settings-fact="version"] [data-fact-label]')?.textContent).toStrictEqual(
    'Version',
  );
  expect(document.querySelector('[data-settings-fact="version"] [data-fact-value]')?.textContent).toStrictEqual(
    messages.settingsAboutVersion,
  );
  expect(document.querySelector('[data-settings-privacy]')?.textContent).toStrictEqual(messages.settingsPrivacyBody);
});

test('a library fact replaces the about data message and hides the playback placeholder', () => {
  const messages = destinationMessages();
  render(
    <Settings
      libraryFact="36 albums · 10 artists"
      pluginSlots={pluginSlots(true)}
      messages={messages}
      shellMessages={shellMessages()}
      theme="dark"
      onThemeChange={() => {}}
      width="compact"
    />,
  );
  const about = document.querySelector('#settings-about');
  expect(about?.textContent?.includes('Demo data')).toStrictEqual(false);
  expect(about?.textContent).toStrictEqual(
    [
      messages.settingsAbout,
      messages.settingsAboutDataLabel,
      '36 albums · 10 artists',
      messages.settingsAboutAddressLabel,
      messages.settingsAboutAddress,
      messages.settingsAboutVersionLabel,
      messages.settingsAboutVersion,
    ].join(''),
  );
  expect(document.querySelector('[data-settings-fact="data"] [data-fact-value]')?.textContent).toStrictEqual(
    '36 albums · 10 artists',
  );
  expect(document.querySelector('[data-settings-fact="data"] [data-fact-value]')?.textContent).not.toStrictEqual(
    messages.settingsAboutData,
  );
  expect(document.querySelector('[data-settings-placeholder="playback"]')).toBeNull();
  expect(
    document.querySelector('#settings-playback')?.textContent?.includes(messages.settingsPlaybackPlaceholder),
  ).toStrictEqual(false);
  expect(document.querySelector('#settings-playback')?.textContent?.includes('Not connected')).toStrictEqual(false);
  expect(radioReport(screen.getByRole('radiogroup', { name: 'Volume levelling' }))).toStrictEqual([
    ['Off', 'Off', 'true', '0'],
    ['Track', 'Track', 'false', '-1'],
    ['Album', 'Album', 'false', '-1'],
  ]);
  expect(
    document.querySelector('#settings-playback [data-settings-row="levelling"] [data-settings-row-hint]')?.textContent,
  ).toStrictEqual('Plays tracks at a consistent loudness, from the tags in your files.');
  expect(document.querySelectorAll('#settings-playback [data-settings-row-status]').length).toStrictEqual(0);
  expect(document.querySelectorAll('#destination-settings [data-settings-badge]').length).toStrictEqual(0);
});

test('connected services stay unavailable, and playback settings are live radio groups', () => {
  renderSettings('compact');
  const rows = (pane: string) =>
    [...document.querySelectorAll(`${pane} [data-settings-row]`)].map((row) => [
      row.getAttribute('data-settings-row'),
      row.getAttribute('data-row-state'),
      row.querySelector('[data-settings-row-label]')?.textContent,
      row.querySelector('[data-settings-row-hint]')?.textContent,
      row.querySelector('[data-settings-row-status]')?.textContent ?? null,
    ]);
  expect(rows('#settings-playback')).toStrictEqual([
    [
      'levelling',
      null,
      'Volume levelling',
      'Plays tracks at a consistent loudness, from the tags in your files.',
      null,
    ],
    ['crossfade', null, 'Crossfade', 'Blends the end of one track into the start of the next.', null],
    ['output', null, 'Output device', 'Chooses the speakers or headphones this device plays through.', null],
  ]);
  expect(radioReport(screen.getByRole('radiogroup', { name: 'Volume levelling' }))).toStrictEqual([
    ['Off', 'Off', 'true', '0'],
    ['Track', 'Track', 'false', '-1'],
    ['Album', 'Album', 'false', '-1'],
  ]);
  expect(radioReport(screen.getByRole('radiogroup', { name: 'Crossfade' }))).toStrictEqual([
    ['Off', 'Off', 'true', '0'],
    ['2 seconds', '2 seconds', 'false', '-1'],
    ['4 seconds', '4 seconds', 'false', '-1'],
    ['6 seconds', '6 seconds', 'false', '-1'],
    ['8 seconds', '8 seconds', 'false', '-1'],
    ['12 seconds', '12 seconds', 'false', '-1'],
  ]);
  expect(radioReport(screen.getByRole('radiogroup', { name: 'Output device' }))).toStrictEqual([
    ['Default', 'Default', 'true', '0'],
  ]);
  // No callback is wired: choosing does not throw, and the controlled value stays.
  fireEvent.click(
    within(screen.getByRole('radiogroup', { name: 'Volume levelling' })).getByRole('radio', { name: 'Album' }),
  );
  fireEvent.click(
    within(screen.getByRole('radiogroup', { name: 'Crossfade' })).getByRole('radio', { name: '8 seconds' }),
  );
  fireEvent.click(
    within(screen.getByRole('radiogroup', { name: 'Output device' })).getByRole('radio', { name: 'Default' }),
  );
  expect(
    within(screen.getByRole('radiogroup', { name: 'Volume levelling' }))
      .getByRole('radio', { name: 'Off' })
      .getAttribute('aria-checked'),
  ).toStrictEqual('true');
  expect(
    within(screen.getByRole('radiogroup', { name: 'Crossfade' }))
      .getByRole('radio', { name: 'Off' })
      .getAttribute('aria-checked'),
  ).toStrictEqual('true');
  expect(
    within(screen.getByRole('radiogroup', { name: 'Output device' }))
      .getByRole('radio', { name: 'Default' })
      .getAttribute('aria-checked'),
  ).toStrictEqual('true');
  expect(rows('#settings-connected')).toStrictEqual([
    [
      'scrobble',
      'unavailable',
      'Scrobbling',
      'Sends what you play to a listening-history service you link yourself.',
      'Not connected',
    ],
    ['lyrics', 'unavailable', 'Lyrics lookup', 'Finds lyrics for tracks whose files have none.', 'Not connected'],
  ]);
  // A service that is not wired shows its status as words: nothing there can be pressed or focused.
  expect(document.querySelectorAll('#settings-connected [tabindex]').length).toStrictEqual(0);
  expect(
    document.querySelectorAll(
      '#settings-connected [role="switch"], #settings-connected [role="checkbox"], #settings-connected [role="button"], #settings-connected [role="radio"], #settings-connected input',
    ).length,
  ).toStrictEqual(0);
  expect(document.querySelector('#settings-playback')?.textContent?.includes('Not connected')).toStrictEqual(false);
});

test('playback volume levelling, crossfade and output device are radio groups that report the choice', () => {
  const messages = destinationMessages();
  const levellingPicks: ('off' | 'track' | 'album')[] = [];
  const fadePicks: (0 | 2 | 4 | 6 | 8 | 12)[] = [];
  const outputPicks: string[] = [];
  const outputs = [
    { id: 'speakers', label: 'Studio speakers' },
    { id: 'headphones', label: 'Headphones' },
  ];
  const tree = (next: {
    levelling: 'off' | 'track' | 'album';
    crossfadeSeconds: 0 | 2 | 4 | 6 | 8 | 12;
    sinkId: string;
    width?: 'compact' | 'wide';
    section?: 'playback';
  }) => (
    <Settings
      section={next.section}
      messages={messages}
      shellMessages={shellMessages()}
      theme="dark"
      onThemeChange={() => {}}
      width={next.width ?? 'compact'}
      levelling={next.levelling}
      onLevelling={(value) => {
        levellingPicks.push(value);
      }}
      crossfadeSeconds={next.crossfadeSeconds}
      onCrossfade={(value) => {
        fadePicks.push(value);
      }}
      outputs={outputs}
      sinkId={next.sinkId}
      onOutput={(id) => {
        outputPicks.push(id);
      }}
    />
  );
  const view = render(tree({ levelling: 'track', crossfadeSeconds: 4, sinkId: 'headphones' }));

  const levelling = screen.getByRole('radiogroup', { name: 'Volume levelling' });
  const crossfade = screen.getByRole('radiogroup', { name: 'Crossfade' });
  const output = screen.getByRole('radiogroup', { name: 'Output device' });
  expect(radioReport(levelling)).toStrictEqual([
    ['Off', 'Off', 'false', '-1'],
    ['Track', 'Track', 'true', '0'],
    ['Album', 'Album', 'false', '-1'],
  ]);
  expect(radioReport(crossfade)).toStrictEqual([
    ['Off', 'Off', 'false', '-1'],
    ['2 seconds', '2 seconds', 'false', '-1'],
    ['4 seconds', '4 seconds', 'true', '0'],
    ['6 seconds', '6 seconds', 'false', '-1'],
    ['8 seconds', '8 seconds', 'false', '-1'],
    ['12 seconds', '12 seconds', 'false', '-1'],
  ]);
  expect(radioReport(output)).toStrictEqual([
    ['Default', 'Default', 'false', '-1'],
    ['Studio speakers', 'Studio speakers', 'false', '-1'],
    ['Headphones', 'Headphones', 'true', '0'],
  ]);
  expect(within(levelling).getByRole('radio', { name: 'Off' }).textContent).toStrictEqual('Off');
  expect(within(levelling).getByRole('radio', { name: 'Track' }).textContent).toStrictEqual('Track');
  expect(within(levelling).getByRole('radio', { name: 'Album' }).textContent).toStrictEqual('Album');
  expect(within(crossfade).getByRole('radio', { name: '12 seconds' }).textContent).toStrictEqual('12 seconds');
  expect(within(output).getByRole('radio', { name: 'Default' }).textContent).toStrictEqual('Default');
  expect(
    document.querySelector('#settings-playback [data-settings-row="levelling"] [data-settings-row-hint]')?.textContent,
  ).toStrictEqual(messages.settingsPlaybackLevellingHint);
  expect(
    document.querySelector('#settings-playback [data-settings-row="crossfade"] [data-settings-row-hint]')?.textContent,
  ).toStrictEqual(messages.settingsPlaybackCrossfadeHint);
  expect(
    document.querySelector('#settings-playback [data-settings-row="output"] [data-settings-row-hint]')?.textContent,
  ).toStrictEqual(messages.settingsPlaybackOutputHint);
  for (const id of ['levelling', 'crossfade', 'output']) {
    expect(
      document
        .querySelector(`#settings-playback [data-settings-row="${id}"]`)
        ?.textContent?.includes('Not connected'),
    ).toStrictEqual(false);
    expect(
      document.querySelector(`#settings-playback [data-settings-row="${id}"] [data-settings-row-status]`),
    ).toBeNull();
  }
  expect([
    document.querySelector('#settings-connected [data-settings-row="lyrics"]')?.getAttribute('data-row-state'),
    document.querySelector('#settings-connected [data-settings-row="lyrics"] [data-settings-row-label]')?.textContent,
    document.querySelector('#settings-connected [data-settings-row="lyrics"] [data-settings-row-status]')?.textContent,
    document.querySelector('#settings-connected [data-settings-row="scrobble"]')?.getAttribute('data-row-state'),
    document.querySelector('#settings-connected [data-settings-row="scrobble"] [data-settings-row-status]')
      ?.textContent,
  ]).toStrictEqual(['unavailable', 'Lyrics lookup', 'Not connected', 'unavailable', 'Not connected']);
  expect(screen.queryByRole('radiogroup', { name: 'Lyrics lookup' })).toBeNull();
  expect(screen.queryByRole('radiogroup', { name: 'Scrobbling' })).toBeNull();

  fireEvent.click(within(levelling).getByRole('radio', { name: 'Album' }));
  fireEvent.click(within(levelling).getByRole('radio', { name: 'Off' }));
  fireEvent.keyDown(within(levelling).getByRole('radio', { name: 'Album' }), { key: 'Enter' });
  fireEvent.keyDown(within(levelling).getByRole('radio', { name: 'Track' }), { key: ' ' });
  fireEvent.keyDown(within(levelling).getByRole('radio', { name: 'Off' }), { key: 'Tab' });
  fireEvent.keyDown(within(levelling).getByRole('radio', { name: 'Off' }), { key: 'a' });
  fireEvent.keyDown(levelling, { key: 'ArrowRight' });
  expect(document.activeElement).toStrictEqual(within(levelling).getByRole('radio', { name: 'Album' }));
  fireEvent.keyDown(levelling, { key: 'ArrowDown' });
  fireEvent.keyDown(levelling, { key: 'ArrowLeft' });
  expect(document.activeElement).toStrictEqual(within(levelling).getByRole('radio', { name: 'Off' }));
  fireEvent.keyDown(levelling, { key: 'ArrowUp' });
  fireEvent.keyDown(levelling, { key: 'Home' });
  expect(levellingPicks).toStrictEqual(['album', 'off', 'album', 'track', 'album', 'album', 'off', 'off']);

  fireEvent.click(within(crossfade).getByRole('radio', { name: '12 seconds' }));
  fireEvent.click(within(crossfade).getByRole('radio', { name: 'Off' }));
  fireEvent.keyDown(crossfade, { key: 'ArrowRight' });
  expect(document.activeElement).toStrictEqual(within(crossfade).getByRole('radio', { name: '6 seconds' }));
  expect(fadePicks).toStrictEqual([12, 0, 6]);
  view.rerender(tree({ levelling: 'album', crossfadeSeconds: 12, sinkId: 'headphones' }));
  fireEvent.keyDown(screen.getByRole('radiogroup', { name: 'Crossfade' }), { key: 'ArrowRight' });
  expect(document.activeElement).toStrictEqual(
    within(screen.getByRole('radiogroup', { name: 'Crossfade' })).getByRole('radio', { name: 'Off' }),
  );
  view.rerender(tree({ levelling: 'off', crossfadeSeconds: 0, sinkId: 'headphones' }));
  fireEvent.keyDown(screen.getByRole('radiogroup', { name: 'Crossfade' }), { key: 'ArrowLeft' });
  expect(fadePicks).toStrictEqual([12, 0, 6, 0, 12]);
  fireEvent.keyDown(screen.getByRole('radiogroup', { name: 'Volume levelling' }), { key: 'ArrowLeft' });
  expect(levellingPicks[levellingPicks.length - 1]).toStrictEqual('album');

  fireEvent.click(
    within(screen.getByRole('radiogroup', { name: 'Output device' })).getByRole('radio', { name: 'Default' }),
  );
  fireEvent.click(
    within(screen.getByRole('radiogroup', { name: 'Output device' })).getByRole('radio', { name: 'Studio speakers' }),
  );
  fireEvent.keyDown(screen.getByRole('radiogroup', { name: 'Output device' }), { key: 'ArrowRight' });
  expect(document.activeElement).toStrictEqual(
    within(screen.getByRole('radiogroup', { name: 'Output device' })).getByRole('radio', { name: 'Default' }),
  );
  expect(outputPicks).toStrictEqual(['', 'speakers', '']);
  view.rerender(tree({ levelling: 'off', crossfadeSeconds: 0, sinkId: '' }));
  fireEvent.keyDown(screen.getByRole('radiogroup', { name: 'Output device' }), { key: 'ArrowLeft' });
  expect(outputPicks[outputPicks.length - 1]).toStrictEqual('headphones');
  view.rerender(tree({ levelling: 'off', crossfadeSeconds: 0, sinkId: 'unplugged' }));
  const unplugged = screen.getByRole('radiogroup', { name: 'Output device' });
  expect(radioReport(unplugged)).toStrictEqual([
    ['Default', 'Default', 'false', '0'],
    ['Studio speakers', 'Studio speakers', 'false', '-1'],
    ['Headphones', 'Headphones', 'false', '-1'],
  ]);
  const beforeMiss = outputPicks.length;
  fireEvent.keyDown(unplugged, { key: 'ArrowRight' });
  fireEvent.keyDown(unplugged, { key: 'ArrowUp' });
  expect(outputPicks.length).toStrictEqual(beforeMiss);
  fireEvent.keyDown(within(unplugged).getByRole('radio', { name: 'Studio speakers' }), { key: 'Enter' });
  expect(outputPicks[outputPicks.length - 1]).toStrictEqual('speakers');

  view.unmount();
  render(tree({ levelling: 'off', crossfadeSeconds: 0, sinkId: '', width: 'wide', section: 'playback' }));
  expect(document.querySelector('#settings-appearance')).toBeNull();
  expect(radioReport(screen.getByRole('radiogroup', { name: 'Volume levelling' }))).toStrictEqual([
    ['Off', 'Off', 'true', '0'],
    ['Track', 'Track', 'false', '-1'],
    ['Album', 'Album', 'false', '-1'],
  ]);
  expect(radioReport(screen.getByRole('radiogroup', { name: 'Output device' }))).toStrictEqual([
    ['Default', 'Default', 'true', '0'],
    ['Studio speakers', 'Studio speakers', 'false', '-1'],
    ['Headphones', 'Headphones', 'false', '-1'],
  ]);
  expect(document.querySelector('#settings-playback')?.textContent?.includes('Not connected')).toStrictEqual(false);
});

function radioReport(group: HTMLElement): (string | null)[][] {
  return within(group)
    .getAllByRole('radio')
    .map((radio) => [
      radio.getAttribute('aria-label'),
      radio.textContent,
      radio.getAttribute('aria-checked'),
      radio.getAttribute('tabindex'),
    ]);
}

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
