import { expect, test } from 'vitest';
import { catalogue } from './catalogue.ts';

test('the catalogue returns trusted shell and destination strings', () => {
  expect(catalogue()).toStrictEqual({
    shell: {
      wordmark: 'Gunmetal',
      navHome: 'Home',
      navSearch: 'Search',
      navLibrary: 'Library',
      navSettings: 'Settings',
      playerEmpty: 'Nothing is playing',
      themeLabel: 'Theme',
      themeDark: 'Dark',
      themeLight: 'Light',
      themeOled: 'OLED',
      themeHighContrast: 'High contrast',
      demoData: 'Demo data',
      primaryNav: 'Primary',
      playerRegion: 'Now playing',
      rightPane: 'Queue',
    },
    destinations: {
      homeHeadline: 'Home',
      searchHeadline: 'Search',
      libraryHeadline: 'Library',
      settingsHeadline: 'Settings',
      notFoundHeadline: 'Not found',
    },
  });
});
