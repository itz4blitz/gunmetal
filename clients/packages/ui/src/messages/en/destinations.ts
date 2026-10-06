export type DestinationMessages = {
  homeHeadline: string;
  searchHeadline: string;
  libraryHeadline: string;
  settingsHeadline: string;
  notFoundHeadline: string;
  recentlyAdded: string;
  continueListening: string;
  loved: string;
  emptyContinue: string;
  emptyLoved: string;
  tabAlbums: string;
  tabArtists: string;
  tabTracks: string;
  play: string;
  playAlbum: string;
  backToLibrary: string;
  tracksHeading: string;
  discsHeading: string;
  searchPlaceholder: string;
  searchRecentEmpty: string;
  searchDemoLocalNotice: string;
  searchNoHits: string;
  settingsAppearance: string;
  settingsPlayback: string;
  settingsAbout: string;
  settingsPlaybackPlaceholder: string;
  settingsAboutBody: string;
  albumMissing: string;
  artistAlbumCount: string;
  trackFlagUnplayable: string;
  trackFlagDamaged: string;
  yearLabel: string;
  hostileAlbumLabel: string;
  hostileArtistLabel: string;
};

export function destinationMessages(): DestinationMessages {
  return {
    homeHeadline: 'Home',
    searchHeadline: 'Search',
    libraryHeadline: 'Library',
    settingsHeadline: 'Settings',
    notFoundHeadline: 'Not found',
    recentlyAdded: 'Recently added',
    continueListening: 'Continue listening',
    loved: 'Loved',
    emptyContinue: 'Nothing to continue yet',
    emptyLoved: 'No loved tracks yet',
    tabAlbums: 'Albums',
    tabArtists: 'Artists',
    tabTracks: 'Tracks',
    play: 'Play',
    playAlbum: 'Play album',
    backToLibrary: 'Back',
    tracksHeading: 'Tracks',
    discsHeading: 'Discs',
    searchPlaceholder: 'Search albums and tracks',
    searchRecentEmpty: 'No recent searches',
    searchDemoLocalNotice: 'Demo-local filter — not CorePort search',
    searchNoHits: "No matches for this query in Music",
    settingsAppearance: 'Appearance',
    settingsPlayback: 'Playback',
    settingsAbout: 'About',
    settingsPlaybackPlaceholder: 'Gain, crossfade and output arrive with CorePort (CP-020).',
    settingsAboutBody: 'This build shows fixture demo data only. It is not a live library.',
    albumMissing: 'That album is not in the demo library',
    artistAlbumCount: 'albums',
    trackFlagUnplayable: 'Cannot play',
    trackFlagDamaged: 'Damaged',
    yearLabel: 'Year',
    hostileAlbumLabel: 'Hostile metadata (fixture)',
    hostileArtistLabel: 'Security corpus',
  };
}
