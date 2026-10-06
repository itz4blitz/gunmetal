export type DestinationMessages = {
  homeHeadline: string;
  searchHeadline: string;
  libraryHeadline: string;
  settingsHeadline: string;
  notFoundHeadline: string;
  recentlyAdded: string;
  recentlyPlayed: string;
  continueListening: string;
  loved: string;
  emptyContinue: string;
  emptyLoved: string;
  emptyRecentlyPlayed: string;
  emptyRecentlyAdded: string;
  tabAlbums: string;
  tabArtists: string;
  tabTracks: string;
  play: string;
  open: string;
  seeAll: string;
  playAlbum: string;
  backToLibrary: string;
  tracksHeading: string;
  discsHeading: string;
  searchPlaceholder: string;
  searchRecentHeading: string;
  searchRecentEmpty: string;
  searchTypeFilter: string;
  searchDemoLocalNotice: string;
  searchNoHits: string;
  settingsAppearance: string;
  settingsDemoData: string;
  settingsDemoDataBody: string;
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
    recentlyPlayed: 'Recently played',
    continueListening: 'Continue listening',
    loved: 'Loved',
    emptyContinue: 'Nothing to continue yet',
    emptyLoved: 'No loved tracks yet',
    emptyRecentlyPlayed: 'Nothing played yet',
    emptyRecentlyAdded: 'No albums added yet',
    tabAlbums: 'Albums',
    tabArtists: 'Artists',
    tabTracks: 'Tracks',
    play: 'Play',
    open: 'Open',
    seeAll: 'See all',
    playAlbum: 'Play album',
    backToLibrary: 'Back',
    tracksHeading: 'Tracks',
    discsHeading: 'Discs',
    searchPlaceholder: 'Search albums and tracks',
    searchRecentHeading: 'Recent searches',
    searchRecentEmpty: 'No recent searches',
    searchTypeFilter: 'Result types',
    searchDemoLocalNotice: 'Demo-local filter — not CorePort search',
    searchNoHits: "No matches for this query in Music",
    settingsAppearance: 'Appearance',
    settingsDemoData: 'Demo data',
    settingsDemoDataBody:
      'Albums and tracks here come from the Stage-A fixture catalogue. They are not loaded from a server.',
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
