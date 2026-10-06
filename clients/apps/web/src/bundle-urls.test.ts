import { expect, test } from 'vitest';
import { absoluteUrls } from './bundle-urls.ts';

test('a scan of a fixture bundle finds a planted absolute URL', () => {
  const fixture = 'import "./ok.js";\nimport "https://evil.example/tracker.js";\nconst x = "http://evil.example/p";\n';
  expect(absoluteUrls(fixture)).toStrictEqual(['https://evil.example/tracker.js', 'http://evil.example/p']);
});

test('a clean bundle holds no absolute URL', () => {
  expect(absoluteUrls('const path = "/assets/index.js";')).toStrictEqual([]);
});
