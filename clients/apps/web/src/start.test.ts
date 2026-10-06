import { expect, test } from 'vitest';
import { start } from './start.ts';
import { scriptUrl } from './policy.ts';

function host(overrides: Partial<Parameters<typeof start>[0]> = {}) {
  const seen: { policy: string | null; navigated: boolean; rendered: boolean } = {
    policy: null,
    navigated: false,
    rendered: false,
  };
  const root = document.createElement('div');
  const input = {
    wasm: true,
    secure: true,
    origin: 'https://gunmetal.example',
    root,
    trustedTypes: {
      createPolicy: (_name: string, rules: { createScriptURL: (url: string) => string | null }) => {
        seen.policy = rules.createScriptURL('https://gunmetal.example/assets/index.js');
      },
    },
    navigateUnsupported: () => {
      seen.navigated = true;
    },
    render: () => {
      seen.rendered = true;
    },
    ...overrides,
  };
  return { seen, result: start(input), root };
}

// Verifies: SEC-API-045, SEC-API-052
test('a capable browser mounts the app after installing the loader policy', () => {
  const { seen, result } = host();
  expect({ result, ...seen }).toStrictEqual({
    result: 'app',
    policy: 'https://gunmetal.example/assets/index.js',
    navigated: false,
    rendered: true,
  });
});

// Verifies: SEC-API-052
test('missing WebAssembly shows the static page and does not run the app', () => {
  const { seen, result } = host({ wasm: false });
  expect({ result, ...seen }).toStrictEqual({
    result: 'unsupported',
    policy: 'https://gunmetal.example/assets/index.js',
    navigated: true,
    rendered: false,
  });
});

// Verifies: SEC-API-052
test('a cleartext non-secure context shows the static page and does not run the app', () => {
  const { seen, result } = host({ secure: false });
  expect({ result, navigated: seen.navigated, rendered: seen.rendered }).toStrictEqual({
    result: 'unsupported',
    navigated: true,
    rendered: false,
  });
});

test('a missing root does not navigate and does not render', () => {
  const { seen, result } = host({ root: null });
  expect({ result, navigated: seen.navigated, rendered: seen.rendered }).toStrictEqual({
    result: 'missing-root',
    navigated: false,
    rendered: false,
  });
});

test('an engine without Trusted Types still mounts when capable', () => {
  const { seen, result } = host({ trustedTypes: undefined });
  expect({ result, policy: seen.policy, rendered: seen.rendered }).toStrictEqual({
    result: 'app',
    policy: null,
    rendered: true,
  });
});

test('the installed policy refuses a foreign script URL', () => {
  expect(scriptUrl('https://evil.example/assets/index.js', 'https://gunmetal.example')).toStrictEqual(null);
});
