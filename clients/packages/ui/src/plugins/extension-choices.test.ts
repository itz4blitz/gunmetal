import { describe, expect, test } from 'vitest';
import {
  defaultChoice,
  installChoice,
  parseChoices,
  serializeChoices,
  setChoiceValue,
  settingsFor,
  uninstallChoice,
} from './extension-choices.ts';

const ids = [
  { id: 'cover-art', status: 'on' as const },
  { id: 'lyrics', status: 'not-in-build' as const },
  { id: 'url-style', status: 'on' as const },
];

describe('extension choices', () => {
  test('a job this server runs starts installed, and one it does not starts off', () => {
    expect(defaultChoice('cover-art', 'on')).toStrictEqual({
      installed: true,
      values: { archive: 'on', portraits: 'on' },
    });
    expect(defaultChoice('lyrics', 'not-in-build')).toStrictEqual({ installed: false, values: {} });
    expect(settingsFor('lyrics')).toStrictEqual([]);
  });

  test('install and uninstall change only the saved choice', () => {
    const start = parseChoices(null, ids);
    const installed = installChoice(start, 'lyrics', 'not-in-build');
    expect(installed.lyrics?.installed).toStrictEqual(true);
    expect(installed['cover-art']?.installed).toStrictEqual(true);
    const removed = uninstallChoice(installed, 'cover-art', 'on');
    expect(removed['cover-art']?.installed).toStrictEqual(false);
    expect(removed.lyrics?.installed).toStrictEqual(true);
  });

  test('a setting accepts only a declared option, and a bad save falls back', () => {
    const start = parseChoices(null, ids);
    const next = setChoiceValue(start, 'url-style', 'on', 'style', 'id');
    expect(next['url-style']?.values.style).toStrictEqual('id');
    expect(setChoiceValue(next, 'url-style', 'on', 'style', 'path')).toStrictEqual(next);
    expect(setChoiceValue(next, 'url-style', 'on', 'missing', 'id')).toStrictEqual(next);
    expect(installChoice({}, 'lyrics', 'not-in-build').lyrics).toStrictEqual({ installed: true, values: {} });
    expect(uninstallChoice({}, 'cover-art', 'on')['cover-art']?.installed).toStrictEqual(false);
    expect(setChoiceValue({}, 'url-style', 'on', 'style', 'id')['url-style']?.values.style).toStrictEqual('id');
    expect(parseChoices('[]', ids)['cover-art']).toStrictEqual(defaultChoice('cover-art', 'on'));
    expect(parseChoices('{"cover-art":[]}', ids)['cover-art']).toStrictEqual(defaultChoice('cover-art', 'on'));
    expect(parseChoices('{"cover-art":{"values":[]}}', ids)['cover-art']).toStrictEqual(defaultChoice('cover-art', 'on'));
    const raw = serializeChoices(next);
    expect(parseChoices(raw, ids)['url-style']?.values.style).toStrictEqual('id');
    expect(parseChoices('{', ids)['cover-art']).toStrictEqual(defaultChoice('cover-art', 'on'));
    expect(parseChoices('{"cover-art":{"installed":"yes","values":{"archive":"maybe"}}}', ids)['cover-art']).toStrictEqual(
      defaultChoice('cover-art', 'on'),
    );
  });
});
