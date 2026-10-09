import { describe, expect, test } from 'vitest';
import { parseStoreCatalog, type StoreListing } from './store-catalog.ts';

const listed: StoreListing = {
  id: 'desk-lamp',
  title: 'Desk lamp',
  version: '1.0.0',
  plane: 'client',
  slot: 'theme-pack',
  status: 'not-in-build',
  summary: 'Would add a lamp colour as data.',
  detail: ['The record is data. A merge lists it. It does not run.'],
  grants: ['theme:apply'],
};

function catalog(extensions: readonly StoreListing[]): string {
  return JSON.stringify({
    extensions,
    id: 'gunmetal.extensions',
    version: '1',
  });
}

describe('parseStoreCatalog', () => {
  test('a merged record is listed whole, including an id this build did not ship', () => {
    expect(parseStoreCatalog(catalog([listed]))).toStrictEqual([listed]);
  });

  test('a catalog that is not the store index is refused', () => {
    expect(parseStoreCatalog('not json')).toBeUndefined();
    expect(parseStoreCatalog('[]')).toBeUndefined();
    expect(parseStoreCatalog(JSON.stringify({ id: 'other', version: '1', extensions: [listed] }))).toBeUndefined();
    expect(parseStoreCatalog(JSON.stringify({ id: 'gunmetal.extensions', version: '2', extensions: [listed] }))).toBeUndefined();
    expect(
      parseStoreCatalog(JSON.stringify({ id: 'gunmetal.extensions', version: '1', extensions: [listed], extra: true })),
    ).toBeUndefined();
    expect(parseStoreCatalog(JSON.stringify({ id: 'gunmetal.extensions', version: '1', extensions: [] }))).toBeUndefined();
    expect(parseStoreCatalog(catalog([{ ...listed, id: 'Bad' }]))).toBeUndefined();
    expect(parseStoreCatalog(catalog([{ ...listed, grants: ['*'] }]))).toBeUndefined();
    expect(parseStoreCatalog(catalog([{ ...listed, status: 'installed' as 'on' }]))).toBeUndefined();
    expect(parseStoreCatalog(catalog([listed, listed]))).toBeUndefined();
  });
});
