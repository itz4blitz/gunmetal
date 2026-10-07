export type DestinationMessages = {
  homeHeadline: string;
  featured: string;
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
  playNext: string;
  addToQueue: string;
  goToAlbum: string;
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
  settingsPlayback: string;
  settingsPlaybackPlaceholder: string;
  settingsConnected: string;
  settingsConnectedEmpty: string;
  settingsExtensions: string;
  settingsExtensionsBody: string;
  settingsBadgeR1: string;
  settingsBadgeR2: string;
  settingsAbout: string;
  settingsAboutData: string;
  settingsAboutAddress: string;
  settingsAboutVersion: string;
  settingsPrivacy: string;
  settingsPrivacyBody: string;
  settingsNav: string;
  settingsSlotMetadata: string;
  settingsSlotLyrics: string;
  settingsSlotSearch: string;
  settingsSlotScrobble: string;
  settingsSlotTheme: string;
  settingsSlotHome: string;
  settingsSlotServer: string;
  settingsSlotClient: string;
  settingsSlotUnloaded: string;
  licenseLabel: string;
  searchPluginNotice: string;
  albumMissing: string;
  artistMissing: string;
  goToArtist: string;
  moreActions: string;
  contextMenu: string;
  lyrics: string;
  lyricsRegion: string;
  artistAlbumCount: string;
  trackFlagUnplayable: string;
  trackFlagDamaged: string;
  yearLabel: string;
  trackCountLabel: string;
  hostileAlbumLabel: string;
  hostileArtistLabel: string;
  artistCountLabel: string;
  searchResultCount: string;
  columnTitle: string;
  columnAlbum: string;
  columnTime: string;
  shuffle: string;
  shuffleUnavailable: string;
  playDisc: string;
  allSongsHeading: string;
  albumEyebrow: string;
  artistEyebrow: string;
  artistSongCount: string;
  columnNumber: string;
  moreByArtist: string;
  hostileTrackHidden: string;
  densityLabel: string;
  densityComfortable: string;
  densityCompact: string;
  searchClear: string;
  /* Appended for the 2026 settings pass (SUR-073): About definition-list
     labels and the Extensions plugin-table column heads. Append-only. */
  settingsAboutDataLabel: string;
  settingsAboutAddressLabel: string;
  settingsAboutVersionLabel: string;
  settingsSlotColumnSlot: string;
  settingsSlotColumnPlane: string;
  settingsSlotColumnState: string;
  /* Appended for the layout pass over Settings, Search and Library: setting
     rows with an honest status, the search idle and top-result states, and
     singular count nouns. Append-only. */
  settingsThemeHint: string;
  settingsUnavailable: string;
  settingsPlaybackLevelling: string;
  settingsPlaybackLevellingHint: string;
  settingsPlaybackCrossfade: string;
  settingsPlaybackCrossfadeHint: string;
  settingsPlaybackOutput: string;
  settingsPlaybackOutputHint: string;
  settingsConnectedScrobble: string;
  settingsConnectedScrobbleHint: string;
  settingsConnectedLyricsHint: string;
  searchHint: string;
  searchBrowseArtists: string;
  searchTopResult: string;
  searchKindTrack: string;
  searchNoHitsHint: string;
  albumCountOne: string;
  artistCountOne: string;
  trackCountOne: string;
  searchResultCountOne: string;
  libraryEmptyAlbums: string;
  libraryEmptyArtists: string;
  libraryEmptyTracks: string;
  settingsSlotLoaded: string;
};

export function destinationMessages(): DestinationMessages {
  return {
    homeHeadline: 'Home',
    featured: 'Featured',
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
    playNext: 'Play next',
    addToQueue: 'Add to queue',
    goToAlbum: 'Go to album',
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
    searchNoHits: 'No matches for this query in Music',
    settingsAppearance: 'Appearance',
    settingsPlayback: 'Playback',
    settingsPlaybackPlaceholder: 'Gain, crossfade and output arrive with CorePort (CP-020).',
    settingsConnected: 'Connected services',
    settingsConnectedEmpty: 'Scrobblers and lyrics lookup arrive as signed plugins in R2.',
    settingsExtensions: 'Extensions / Plugins',
    settingsExtensionsBody: 'Plugins run as WebAssembly with per-grant consent; none load in this build.',
    settingsBadgeR1: 'R1',
    settingsBadgeR2: 'R2',
    settingsAbout: 'About this connection',
    settingsAboutData: 'Demo data',
    settingsAboutAddress: 'loopback',
    settingsAboutVersion: 'demo',
    settingsPrivacy: 'Privacy',
    settingsPrivacyBody: 'History and loves stay on this profile; this demo has no server yet.',
    settingsNav: 'Settings sections',
    settingsSlotMetadata: 'Metadata and artwork',
    settingsSlotLyrics: 'Lyrics lookup',
    settingsSlotSearch: 'Catalogue search',
    settingsSlotScrobble: 'Scrobblers',
    settingsSlotTheme: 'Themes',
    settingsSlotHome: 'Home rows',
    settingsSlotServer: 'Server',
    settingsSlotClient: 'Client',
    settingsSlotUnloaded: 'Not loaded',
    licenseLabel: 'License',
    searchPluginNotice: 'A signed catalogue plugin can find releases outside this library. None is loaded.',
    albumMissing: 'That album is not in the demo library',
    artistMissing: 'That artist is not in the demo library',
    goToArtist: 'Go to artist',
    moreActions: 'More',
    contextMenu: 'Actions',
    lyrics: 'Lyrics',
    lyricsRegion: 'Lyrics',
    artistAlbumCount: 'albums',
    trackFlagUnplayable: 'Cannot play',
    trackFlagDamaged: 'Damaged',
    yearLabel: 'Year',
    trackCountLabel: 'tracks',
    hostileAlbumLabel: 'Hostile metadata (fixture)',
    hostileArtistLabel: 'Security corpus',
    artistCountLabel: 'artists',
    searchResultCount: 'results',
    columnTitle: 'Title',
    columnAlbum: 'Album',
    columnTime: 'Time',
    shuffle: 'Shuffle',
    shuffleUnavailable: 'Shuffle is not wired in this demo yet',
    playDisc: 'Play disc',
    allSongsHeading: 'All songs',
    albumEyebrow: 'Album',
    artistEyebrow: 'Artist',
    artistSongCount: 'songs',
    columnNumber: '#',
    moreByArtist: 'More by',
    hostileTrackHidden: 'Track title hidden (hostile metadata)',
    densityLabel: 'Row density',
    densityComfortable: 'Comfortable',
    densityCompact: 'Compact',
    searchClear: 'Clear search',
    /* 2026 settings pass (SUR-073). Append-only. */
    settingsAboutDataLabel: 'Library',
    settingsAboutAddressLabel: 'Address',
    settingsAboutVersionLabel: 'Version',
    settingsSlotColumnSlot: 'Extension',
    settingsSlotColumnPlane: 'Runs on',
    settingsSlotColumnState: 'Status',
    /* Layout pass over Settings, Search and Library. Append-only. */
    settingsThemeHint: 'Dark is the default. OLED uses true black, and High contrast strengthens every edge.',
    settingsUnavailable: 'Not available yet',
    settingsPlaybackLevelling: 'Volume levelling',
    settingsPlaybackLevellingHint: 'Plays tracks at a consistent loudness, from the tags in your files.',
    settingsPlaybackCrossfade: 'Crossfade',
    settingsPlaybackCrossfadeHint: 'Blends the end of one track into the start of the next.',
    settingsPlaybackOutput: 'Output device',
    settingsPlaybackOutputHint: 'Chooses the speakers or headphones this device plays through.',
    settingsConnectedScrobble: 'Scrobbling',
    settingsConnectedScrobbleHint: 'Sends what you play to a listening-history service you link yourself.',
    settingsConnectedLyricsHint: 'Finds lyrics for tracks whose files have none.',
    searchHint: 'Search by album, track or artist name.',
    searchBrowseArtists: 'Browse artists',
    searchTopResult: 'Top result',
    searchKindTrack: 'Track',
    searchNoHitsHint: 'Check the spelling, or try a shorter word.',
    albumCountOne: 'album',
    artistCountOne: 'artist',
    trackCountOne: 'track',
    searchResultCountOne: 'result',
    libraryEmptyAlbums: 'No albums in this library yet.',
    libraryEmptyArtists: 'No artists in this library yet.',
    libraryEmptyTracks: 'No tracks in this library yet.',
    settingsSlotLoaded: 'Loaded',
  };
}
