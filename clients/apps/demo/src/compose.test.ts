import { expect, test } from 'vitest';
import { composeDemo } from './compose.ts';

test('the demo composition root injects fixture library and labels demo data', () => {
  const demo = composeDemo();
  expect(demo.showDemoLabel).toStrictEqual(true);
  expect(demo.library.kind).toStrictEqual('demo-fixtures');
  expect(demo.library.albums.map((album) => album.id)).toStrictEqual([
    'demo-album-01',
    'demo-album-02',
    'demo-album-03',
    'demo-album-04',
    'demo-album-05',
    'demo-album-06',
    'demo-album-07',
    'demo-album-08',
  ]);
});
