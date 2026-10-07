import { createHash } from 'node:crypto';
import { expect, test } from 'vitest';
import { contentSecurityPolicy, htmlHeaders } from './headers.ts';

const policy = [
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

// Verifies: SEC-API-044
test('the content security policy is SEC-API-044 plus the D-74 hash of an empty style element', () => {
  expect(contentSecurityPolicy()).toStrictEqual(policy);
  expect(createHash('sha256').update('').digest('base64')).toStrictEqual(
    '47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=',
  );
});

// Verifies: SEC-API-044, SEC-CLI-004
test('HTML responses send the policy and nosniff', () => {
  expect(htmlHeaders()).toStrictEqual({
    'Content-Security-Policy': policy,
    'X-Content-Type-Options': 'nosniff',
  });
});
