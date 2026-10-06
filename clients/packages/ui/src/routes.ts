export type Route = {
  path: string;
  surface: string;
  needsSession: boolean;
  needsAdminSession: boolean;
};

// Closed list of secret-free paths (CP-009, SEC-CLI-025). Matched by exact comparison only.
export function routes(): readonly Route[] {
  return [
    { path: '/', surface: 'SUR-020', needsSession: true, needsAdminSession: false },
    { path: '/search', surface: 'SUR-032', needsSession: true, needsAdminSession: false },
    { path: '/library', surface: 'SUR-022', needsSession: true, needsAdminSession: false },
    { path: '/settings', surface: 'SUR-073', needsSession: true, needsAdminSession: false },
  ];
}
