export type ShellMessages = {
  wordmark: string;
  navHome: string;
  navSearch: string;
  navLibrary: string;
  navSettings: string;
  playerEmpty: string;
  themeLabel: string;
  themeDark: string;
  themeLight: string;
  themeOled: string;
  themeHighContrast: string;
  demoData: string;
  primaryNav: string;
  playerRegion: string;
  rightPane: string;
};

export function shellMessages(): ShellMessages {
  return {
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
  };
}
