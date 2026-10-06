// The production policy the test server and the committed dev server send on every HTML response
// (SEC-API-044, SEC-CLI-004). style-src also names the SHA-256 of an empty style element, which is
// what React Native for Web inserts before it writes rules through the CSSOM (D-74).

export function contentSecurityPolicy(): string {
  return [
    "default-src 'none'",
    "script-src 'self' 'wasm-unsafe-eval'",
    "style-src 'self' 'sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU='",
    "img-src 'self' blob:",
    "media-src 'self' blob:",
    "font-src 'self'",
    "connect-src 'self'",
    "worker-src 'self'",
    "manifest-src 'self'",
    "base-uri 'none'",
    "form-action 'self'",
    "frame-ancestors 'none'",
    "object-src 'none'",
    "require-trusted-types-for 'script'",
    'trusted-types gunmetal-loader',
  ].join('; ');
}

export function htmlHeaders(): {
  'Content-Security-Policy': string;
  'X-Content-Type-Options': 'nosniff';
} {
  return {
    'Content-Security-Policy': contentSecurityPolicy(),
    'X-Content-Type-Options': 'nosniff',
  };
}
