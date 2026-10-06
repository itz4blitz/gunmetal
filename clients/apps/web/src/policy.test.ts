import { expect, test } from 'vitest';
import { acceptsScriptUrl, installGunmetalLoader, scriptUrl } from './policy.ts';

const origin = 'https://gunmetal.example';

function decision(url: string): boolean {
  return acceptsScriptUrl(url, origin);
}

// Verifies: SEC-API-045, SEC-API-049
test('only same-origin URLs under the asset prefix pass', () => {
  const allowed = [
    `${origin}/assets/index.js`,
    `${origin}/assets/chunks/player.js`,
    '/assets/index.js',
    'assets/index.js',
  ];
  const refused = [
    `${origin}/other.js`,
    `${origin}/asset/index.js`,
    `${origin}/assets`,
    `${origin}/Assets/index.js`,
    'https://evil.example/assets/index.js',
    'http://gunmetal.example/assets/index.js',
    'https://gunmetal.example:4443/assets/index.js',
    '//evil.example/assets/index.js',
    ['java', 'script:alert(1)'].join(''),
    'data:text/javascript,alert(1)',
    'blob:https://gunmetal.example/assets/index.js',
    '',
    'not a url \x00',
  ];
  expect(allowed.map(decision)).toStrictEqual([true, true, true, true]);
  expect(refused.map(decision)).toStrictEqual(refused.map(() => false));
});

// Verifies: SEC-API-045
test('a resolved parent path that leaves the asset prefix is refused', () => {
  expect(decision(`${origin}/assets/../secret.js`)).toStrictEqual(false);
  expect(decision('/assets/../secret.js')).toStrictEqual(false);
});

// Verifies: SEC-API-045
test('the policy function returns the URL or null', () => {
  expect(scriptUrl(`${origin}/assets/index.js`, origin)).toStrictEqual(`${origin}/assets/index.js`);
  expect(scriptUrl(`${origin}/secret.js`, origin)).toStrictEqual(null);
  expect(scriptUrl('http://[', origin)).toStrictEqual(null);
  expect(acceptsScriptUrl('http://[', origin)).toStrictEqual(false);
});

// Verifies: SEC-API-045
test('every random path under the prefix on this origin is accepted and every other origin is refused', () => {
  const names = ['a.js', 'b-c.js', '0', 'chunk/nested/file.js'];
  expect(names.map((name) => decision(`${origin}/assets/${name}`))).toStrictEqual([true, true, true, true]);
  expect(names.map((name) => decision(`https://other.example/assets/${name}`))).toStrictEqual([
    false,
    false,
    false,
    false,
  ]);
});

// Verifies: SEC-API-045
test('the one Trusted Types policy is gunmetal-loader and is skipped when the engine has none', () => {
  const created: { name: string; url: string | null }[] = [];
  installGunmetalLoader(undefined, () => 'x');
  expect(created).toStrictEqual([]);
  installGunmetalLoader(
    {
      createPolicy: (name, rules) => {
        created.push({ name, url: rules.createScriptURL(`${origin}/assets/index.js`) });
      },
    },
    (url) => scriptUrl(url, origin),
  );
  expect(created).toStrictEqual([{ name: 'gunmetal-loader', url: `${origin}/assets/index.js` }]);
});
