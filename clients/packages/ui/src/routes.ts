export type Route = {
  path: string;
  surface: string;
  needsSession: boolean;
  needsAdminSession: boolean;
};

// Closed list of secret-free paths (CP-009, SEC-CLI-025). Matched by exact comparison only.
// `/settings` is the appearance address; the shell replaces it with `/settings/appearance`.
export function routes(): readonly Route[] {
  return [
    { path: '/', surface: 'SUR-020', needsSession: true, needsAdminSession: false },
    { path: '/search', surface: 'SUR-032', needsSession: true, needsAdminSession: false },
    { path: '/library', surface: 'SUR-022', needsSession: true, needsAdminSession: false },
    { path: '/settings', surface: 'SUR-076', needsSession: true, needsAdminSession: false },
    { path: '/settings/appearance', surface: 'SUR-076', needsSession: true, needsAdminSession: false },
    { path: '/settings/playback', surface: 'SUR-074', needsSession: true, needsAdminSession: false },
    { path: '/settings/extensions', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
    { path: '/settings/about', surface: 'SUR-077', needsSession: true, needsAdminSession: false },
    { path: '/settings/privacy', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
  ];
}
