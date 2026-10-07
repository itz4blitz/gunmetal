export type ShellMessages = {
  wordmark: string;
  skipToContent: string;
  skipToPlayer: string;
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
  play: string;
  pause: string;
  previous: string;
  next: string;
  queue: string;
  volume: string;
  queueEmpty: string;
  queueHeading: string;
  queueClose: string;
  progress: string;
  openFullPlayer: string;
  playerFullRegion: string;
  playerClose: string;
  lyrics: string;
  playingFrom: string;
  lyricsUnavailable: string;
};

export function shellMessages(): ShellMessages {
  return {
    wordmark: 'Gunmetal',
    skipToContent: 'Skip to content',
    skipToPlayer: 'Skip to player',
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
    play: 'Play',
    pause: 'Pause',
    previous: 'Previous',
    next: 'Next',
    queue: 'Queue',
    volume: 'Volume',
    queueEmpty: 'Queue is empty',
    queueHeading: 'Up next',
    queueClose: 'Close queue',
    progress: 'Progress',
    openFullPlayer: 'Open full player',
    playerFullRegion: 'Full player',
    playerClose: 'Close',
    lyrics: 'Lyrics',
    playingFrom: 'Playing from',
    lyricsUnavailable: 'This file has no lyrics.',
  };
}
