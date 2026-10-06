export function screens(): readonly { name: string; path: string }[] {
  return [
    { name: 'shell', path: '/' },
    { name: 'unsupported-browser', path: '/unsupported.html' },
  ];
}
