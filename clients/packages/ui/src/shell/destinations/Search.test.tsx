import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { demoLibrary } from '../../../../fake-server/src/catalogue.ts';
import { destinationMessages } from '../../messages/en/destinations.ts';
import { Search } from './Search.tsx';

afterEach(cleanup);

const here = dirname(fileURLToPath(import.meta.url));

function renderSearch() {
  return render(
    <Search
      messages={destinationMessages()}
      library={demoLibrary()}
      onOpenAlbum={vi.fn()}
      onPlayAlbum={vi.fn()}
      onPlayTrack={vi.fn()}
    />,
  );
}

function typeQuery(value: string): void {
  fireEvent.change(screen.getByLabelText('Search albums and tracks'), { target: { value } });
}

test('local lookup finds Cylinders by artist, SPDX, attribution, source and track title', () => {
  renderSearch();
  typeQuery('zabriskie');
  expect(document.querySelector('#album-tile-demo-album-09')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Cylinders' })).toBeTruthy();
  expect(document.querySelector('#search-plugin-notice')?.textContent).toStrictEqual(
    'A signed catalogue plugin can find releases outside this library. None is loaded.',
  );

  typeQuery('cc-by-4.0');
  expect(document.querySelector('#album-tile-demo-album-09')).toBeTruthy();
  expect(document.querySelectorAll('[data-album-tile]').length).toStrictEqual(1);

  typeQuery('Cylinders by Chris Zabriskie');
  expect(document.querySelector('#album-tile-demo-album-09')).toBeTruthy();

  typeQuery('chriszabriskie.com');
  expect(document.querySelector('#album-tile-demo-album-09')).toBeTruthy();

  typeQuery('Cylinder Seven');
  expect(document.querySelector('#track-row-demo-track-09-07')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Cylinder Seven' })).toBeTruthy();
});

// Verifies: SEC-HIS-024, SEC-API-044
test('search lookup stays on the local fixture set and does not fetch a third party', async () => {
  renderSearch();
  typeQuery('Cylinders');
  expect(document.querySelector('#search-demo-notice')?.textContent).toStrictEqual(
    'Demo-local filter — not CorePort search',
  );
  expect(document.querySelector('#search-plugin-notice')?.textContent).toStrictEqual(
    'A signed catalogue plugin can find releases outside this library. None is loaded.',
  );
  const source = await readFile(join(here, 'Search.tsx'), 'utf8');
  expect(source.includes('fetch(')).toStrictEqual(false);
  expect(source.includes('https://')).toStrictEqual(false);
  expect(source.includes('jamendo')).toStrictEqual(false);
  expect(source.includes('archive.org')).toStrictEqual(false);
  expect(source.includes('WebAssembly')).toStrictEqual(false);
});
