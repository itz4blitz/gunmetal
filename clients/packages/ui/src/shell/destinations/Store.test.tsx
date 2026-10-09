import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { catalogue } from '../../messages/catalogue.ts';
import { Store } from './Store.tsx';

afterEach(() => {
  cleanup();
  localStorage.removeItem('gunmetal.extension.choices');
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

function renderDetail(id: string, onOpenPath: (path: string) => void = () => {}) {
  return render(<Store messages={catalogue().destinations} selectedId={id} onOpenPath={onOpenPath} />);
}

test('a store record page shows its story, its state, and its action in the store', () => {
  renderDetail('cover-art');
  const page = document.querySelector('[data-store-detail="cover-art"]');
  expect(page).not.toBeNull();
  expect(screen.getByRole('heading', { name: 'Metadata and artwork' }).id).toStrictEqual('destination-headline');
  expect(page?.querySelector('[data-store-status]')?.textContent).toStrictEqual('On this server');
  expect(page?.querySelector('[data-store-summary]')?.textContent).toStrictEqual(
    'Fills missing album art and artist photos from MusicBrainz and Cover Art Archive.',
  );
  expect(page?.querySelector('[data-store-section-title]')?.textContent).toStrictEqual('What it does');
  expect([...(page?.querySelectorAll('[data-store-detail-line]') ?? [])].map((line) => line.textContent)).toStrictEqual(
    [
      'Runs inside this library host. It is not a downloaded package.',
      'Album art comes from embedded pictures first, then Cover Art Archive.',
      'Artist photos come from the Wikidata portrait on the MusicBrainz artist.',
    ],
  );
  expect(screen.getByRole('button', { name: 'Uninstall Metadata and artwork' })).not.toBeNull();
  expect(page?.querySelector('[data-store-action-detail]')?.getAttribute('data-store-action-detail')).toStrictEqual(
    'uninstall',
  );
  expect(page?.querySelector('[data-store-saved-note]')).toBeNull();
});

test('a record page carries the record own settings and saves a choice', () => {
  renderDetail('cover-art');
  const label = (settingId: string) =>
    document.querySelector(`[data-store-setting="${settingId}"] [data-store-setting-label]`)?.textContent;
  expect(label('archive')).toStrictEqual('Cover Art Archive');
  expect(label('portraits')).toStrictEqual('Artist photos');
  const archive = screen.getByRole('button', { name: 'Cover Art Archive Off' });
  const before = document.querySelector('[data-store-setting="archive"] [data-store-setting-option="on"]');
  expect(before?.getAttribute('data-store-setting-selected')).toStrictEqual('1');
  fireEvent.click(archive);
  expect(
    document
      .querySelector('[data-store-setting="archive"] [data-store-setting-option="off"]')
      ?.getAttribute('data-store-setting-selected'),
  ).toStrictEqual('1');
  expect(localStorage.getItem('gunmetal.extension.choices')).toContain('"archive":"off"');
  // A key the group does not offer changes nothing; Enter on a chip changes it.
  const on = screen.getByRole('button', { name: 'Cover Art Archive On' });
  fireEvent.keyDown(on, { key: 'Tab' });
  expect(localStorage.getItem('gunmetal.extension.choices')).toContain('"archive":"off"');
  fireEvent.keyDown(on, { key: 'Enter' });
  expect(localStorage.getItem('gunmetal.extension.choices')).toContain('"archive":"on"');
});

test('installing a record this build cannot run says Saved and can be undone', () => {
  renderDetail('lyrics');
  const page = document.querySelector('[data-store-detail="lyrics"]');
  expect(page?.querySelector('[data-store-status]')?.textContent).toStrictEqual('In the store');
  expect(page?.querySelector('[data-store-action-detail]')?.getAttribute('data-store-action-detail')).toStrictEqual(
    'install',
  );
  expect(page?.querySelector('[data-store-settings]')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Install Lyrics lookup' }));
  expect(page?.querySelector('[data-store-status]')?.textContent).toStrictEqual('Saved');
  expect(page?.querySelector('[data-store-saved-note]')?.textContent).toStrictEqual(
    'Saved. This server does not run it yet.',
  );
  expect(localStorage.getItem('gunmetal.extension.choices')).toContain('"lyrics":{"installed":true');
  fireEvent.click(screen.getByRole('button', { name: 'Uninstall Lyrics lookup' }));
  expect(page?.querySelector('[data-store-status]')?.textContent).toStrictEqual('In the store');
  expect(page?.querySelector('[data-store-saved-note]')).toBeNull();
  expect(localStorage.getItem('gunmetal.extension.choices')).toContain('"lyrics":{"installed":false');
  // Keyboard install works too: Enter, then Space on the way back.
  fireEvent.keyDown(screen.getByRole('button', { name: 'Install Lyrics lookup' }), { key: 'Enter' });
  expect(page?.querySelector('[data-store-status]')?.textContent).toStrictEqual('Saved');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Uninstall Lyrics lookup' }), { key: ' ' });
  expect(page?.querySelector('[data-store-status]')?.textContent).toStrictEqual('In the store');
});

test('uninstalling a record this server runs turns it Off', () => {
  renderDetail('url-style');
  const page = document.querySelector('[data-store-detail="url-style"]');
  expect(page?.querySelector('[data-store-status]')?.textContent).toStrictEqual('On this server');
  fireEvent.click(screen.getByRole('button', { name: 'Uninstall Address style' }));
  expect(page?.querySelector('[data-store-status]')?.textContent).toStrictEqual('Off');
  expect(localStorage.getItem('gunmetal.extension.choices')).toContain('"url-style":{"installed":false');
});

test('the address style record offers Name and Id, and Id is saved', () => {
  renderDetail('url-style');
  fireEvent.click(screen.getByRole('button', { name: 'Address style Id' }));
  expect(localStorage.getItem('gunmetal.extension.choices')).toContain('"style":"id"');
});

test('back to store is the store route, by pointer and by key', () => {
  const opened: string[] = [];
  renderDetail('lyrics', (path) => opened.push(path));
  fireEvent.click(screen.getByRole('button', { name: 'Back to store' }));
  expect(opened).toStrictEqual(['/store']);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back to store' }), { key: ' ' });
  expect(opened).toStrictEqual(['/store', '/store']);
  fireEvent.keyDown(screen.getByRole('button', { name: 'Back to store' }), { key: 'Tab' });
  expect(opened).toStrictEqual(['/store', '/store']);
});

test('a store record that is not known has no page', () => {
  render(<Store messages={catalogue().destinations} selectedId="desk-lamp" />);
  expect(document.querySelector('[data-store-detail]')).toBeNull();
  expect(screen.getByRole('heading', { name: 'Store' }).id).toStrictEqual('destination-headline');
});

test('the same-origin index replaces the built-in list when it answers', async () => {
  const listing = {
    id: 'desk-lamp',
    title: 'Desk lamp',
    version: '1.0.0',
    plane: 'client',
    slot: 'theme-pack',
    status: 'not-in-build',
    summary: 'Would add a lamp colour as data.',
    detail: ['A merge lists it.'],
    grants: ['theme:apply'],
  };
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => ({
      ok: true,
      text: async () => JSON.stringify({ id: 'gunmetal.extensions', version: '1', extensions: [listing] }),
    })),
  );
  const view = render(<Store messages={catalogue().destinations} />);
  expect(await screen.findByText('Desk lamp')).not.toBeNull();
  // The list now shows only the published record.
  expect(document.querySelector('[data-store-card="desk-lamp"]')).not.toBeNull();
  expect(document.querySelector('[data-store-card="lyrics"]')).toBeNull();
  view.unmount();
});

test('a published index that fails leaves the built-in list in place', async () => {
  const notOk = vi.fn(async () => ({ ok: false }));
  vi.stubGlobal('fetch', notOk);
  render(<Store messages={catalogue().destinations} />);
  expect(screen.getByText('Metadata and artwork')).not.toBeNull();
  await vi.waitFor(() => {
    expect(notOk).toHaveBeenCalled();
  });
  expect(document.querySelector('[data-store-card="lyrics"]')).not.toBeNull();
});

test('an unpublished record cannot be opened from the list', () => {
  const onOpenPath = vi.fn();
  const view = render(
    <Store
      messages={catalogue().destinations}
      listings={[
        {
          id: 'desk-lamp',
          title: 'Desk lamp',
          version: '1.0.0',
          plane: 'client',
          slot: 'theme-pack',
          status: 'not-in-build',
          summary: 'Would add a lamp colour as data.',
          detail: ['A merge lists it.'],
          grants: ['theme:apply'],
        },
      ]}
      onOpenPath={onOpenPath}
    />,
  );
  fireEvent.click(view.container.querySelector('[data-store-card="desk-lamp"]') as Element);
  expect(onOpenPath).not.toHaveBeenCalled();
  fireEvent.keyDown(screen.getByRole('button', { name: 'Install Desk lamp' }), { key: 'Enter' });
  expect(document.querySelector('[data-store-card="desk-lamp"] [data-store-status]')?.textContent).toStrictEqual(
    'Saved',
  );
});
