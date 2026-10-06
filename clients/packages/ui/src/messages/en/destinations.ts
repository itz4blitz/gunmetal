export type DestinationMessages = {
  homeHeadline: string;
  searchHeadline: string;
  libraryHeadline: string;
  settingsHeadline: string;
  notFoundHeadline: string;
};

export function destinationMessages(): DestinationMessages {
  return {
    homeHeadline: 'Home',
    searchHeadline: 'Search',
    libraryHeadline: 'Library',
    settingsHeadline: 'Settings',
    notFoundHeadline: 'Not found',
  };
}
