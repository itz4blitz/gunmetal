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
  shellThemeSystem: string;
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
  resizeSidebar: string;
  resizeQueue: string;
  mute: string;
  unmute: string;
  /* 2026-10-07 player pass: shuffle/repeat states, honest buffering and
     playback-error copy, and the queue-line remove action. Append-only. */
  playerShuffle: string;
  playerRepeat: string;
  playerRepeatAll: string;
  playerRepeatOne: string;
  playerBuffering: string;
  playerPlaybackFailed: string;
  playerRemove: string;
  /* The Store door. An extension is a record, not a download. */
  navStore: string;
  /* The destination pin. Its name carries the state it will move to. */
  pinToSidebar: string;
  unpinFromSidebar: string;
  /* The Watch door. It is listed only when the host passes a watch area. */
  navWatch: string;
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
    shellThemeSystem: 'System',
    demoData: 'Fixture library',
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
    resizeSidebar: 'Resize sidebar',
    resizeQueue: 'Resize queue',
    mute: 'Mute',
    unmute: 'Unmute',
    /* 2026-10-07 player pass: shuffle/repeat states, honest buffering and
       playback-error copy, and the queue-line remove action. Append-only. */
    playerShuffle: 'Shuffle',
    playerRepeat: 'Repeat',
    playerRepeatAll: 'Repeat all',
    playerRepeatOne: 'Repeat one',
    playerBuffering: 'Buffering…',
    playerPlaybackFailed: 'This track could not be played.',
    playerRemove: 'Remove from queue',
    /* The Store door. An extension is a record, not a download. */
    navStore: 'Store',
    /* The destination pin. Its name carries the state it will move to. */
    pinToSidebar: 'Pins the page to the sidebar',
    unpinFromSidebar: 'Unpins the page from the sidebar',
    /* The Watch door. It is listed only when the host passes a watch area. */
    navWatch: 'Watch',
  };
}
