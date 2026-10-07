import { beforeEach, expect, test } from 'vitest';
import type { ClientPluginManifest } from '../../../ports/src/plugins/types.ts';
import {
  clearClientPlugins,
  clientPlugins,
  enabledClientPlugins,
  registerClientPlugin,
  type ClientPluginRecord,
} from './registry.ts';

function manifest(overrides: Partial<ClientPluginManifest>): ClientPluginManifest {
  return {
    id: 'vault-row',
    title: 'Vault',
    version: '1.0.0',
    plane: 'client',
    slot: 'home-row',
    grants: ['home-row:read'],
    ...overrides,
  };
}

function homeRowRecord(overrides?: {
  manifest?: Partial<ClientPluginManifest>;
  rowId?: string;
  rowTitle?: string;
  kind?: 'recent' | 'loved';
}): ClientPluginRecord {
  const row =
    overrides?.kind === undefined
      ? {
          id: overrides?.rowId ?? 'vault-row-1',
          title: overrides?.rowTitle ?? 'From the vault',
          kind: 'custom' as const,
          albums: ['demo-album-09'],
        }
      : { id: overrides?.rowId ?? 'vault-row-1', title: overrides?.rowTitle ?? 'From the vault', kind: overrides.kind };
  return { manifest: manifest(overrides?.manifest ?? {}), contribution: { slot: 'home-row', row } };
}

function themePackRecord(overrides?: {
  manifest?: Partial<ClientPluginManifest>;
  themeId?: string;
  base?: 'dark' | 'light' | 'oled' | 'high-contrast';
}): ClientPluginRecord {
  return {
    manifest: manifest({
      id: overrides?.manifest?.id ?? 'demo-themes',
      slot: 'theme-pack',
      grants: ['theme-pack:apply'],
      ...overrides?.manifest,
    }),
    contribution: {
      slot: 'theme-pack',
      // The registry reads plain data; the shape check belongs to the theme drawer, so the
      // record is written literally here and only valid packs are registered in these tests.
      theme: { id: overrides?.themeId ?? 'demo-themes-oled', title: 'Oled demo', base: overrides?.base ?? 'oled' },
    },
  };
}

beforeEach(() => {
  clearClientPlugins();
});

test('a valid home-row record registers, lists and is absent until consented', () => {
  expect(registerClientPlugin(homeRowRecord())).toStrictEqual({ outcome: 'ok' });
  expect(clientPlugins()).toStrictEqual([
    {
      manifest: {
        id: 'vault-row',
        title: 'Vault',
        version: '1.0.0',
        plane: 'client',
        slot: 'home-row',
        grants: ['home-row:read'],
      },
      contribution: {
        slot: 'home-row',
        row: { id: 'vault-row-1', title: 'From the vault', kind: 'custom', albums: ['demo-album-09'] },
      },
    },
  ]);
  expect(enabledClientPlugins([])).toStrictEqual([]);
  expect(enabledClientPlugins([{ manifestId: 'another-plugin', grantedAt: '2026-10-07T00:00:00Z' }])).toStrictEqual([]);
});

test('a valid theme-pack record registers beside a home-row record, in registration order', () => {
  expect(registerClientPlugin(homeRowRecord())).toStrictEqual({ outcome: 'ok' });
  expect(registerClientPlugin(themePackRecord())).toStrictEqual({ outcome: 'ok' });
  expect(clientPlugins().map((record) => record.manifest.id)).toStrictEqual(['vault-row', 'demo-themes']);
});

test('a recent or loved row registers like a custom row', () => {
  expect(registerClientPlugin(homeRowRecord({ kind: 'recent' }))).toStrictEqual({ outcome: 'ok' });
  expect(
    registerClientPlugin(
      homeRowRecord({ kind: 'loved', manifest: { id: 'loved-row-plugin' }, rowId: 'loved-row', rowTitle: 'Loved' }),
    ),
  ).toStrictEqual({
    outcome: 'ok',
  });
  expect(clientPlugins().map((record) => record.contribution)).toStrictEqual([
    { slot: 'home-row', row: { id: 'vault-row-1', title: 'From the vault', kind: 'recent' } },
    { slot: 'home-row', row: { id: 'loved-row', title: 'Loved', kind: 'loved' } },
  ]);
});

test('a manifest that is not plane client is rejected as bad-plane', () => {
  const record = homeRowRecord({ manifest: { plane: 'server' as never } });
  expect(registerClientPlugin(record)).toStrictEqual({ outcome: 'rejected', reason: 'bad-plane', id: 'vault-row' });
  expect(clientPlugins()).toStrictEqual([]);
});

test.each([
  ['', 'bad-id'],
  ['Vault-Row', 'bad-id'],
  ['-lead', 'bad-id'],
  ['trail-', 'bad-id'],
  ['dou--ble', 'bad-id'],
  ['spa ce', 'bad-id'],
  ['under_score', 'bad-id'],
  ['dot.name', 'bad-id'],
  ['v1.0.0', 'bad-version'],
  ['1.0', 'bad-version'],
  ['1.0.0.0', 'bad-version'],
  ['1.0.0-beta', 'bad-version'],
])('a manifest id or version outside the admitted shape is rejected (%s)', (value, reason) => {
  const id = reason === 'bad-version' ? 'vault-row' : value;
  const record = homeRowRecord({ manifest: { id, version: reason === 'bad-version' ? value : '1.0.0' } });
  expect(registerClientPlugin(record)).toStrictEqual({ outcome: 'rejected', reason, id });
  expect(clientPlugins()).toStrictEqual([]);
});

test('a valid id registers where a nearly identical one was rejected', () => {
  expect(registerClientPlugin(homeRowRecord({ manifest: { id: 'vault-row-2' } }))).toStrictEqual({ outcome: 'ok' });
});

test.each([['metadata-provider'], [''], ['home-row ']])(
  'a manifest slot that is not a client-plane slot is rejected as unknown-slot (%s)',
  (slot) => {
    const record = homeRowRecord({ manifest: { slot: slot as never } });
    expect(registerClientPlugin(record)).toStrictEqual({
      outcome: 'rejected',
      reason: 'unknown-slot',
      id: 'vault-row',
    });
  },
);

test('a manifest carrying a grant outside the closed set is rejected as unknown-grant', () => {
  const record = homeRowRecord({ manifest: { grants: ['home-row:read', 'library:write' as never] } });
  expect(registerClientPlugin(record)).toStrictEqual({ outcome: 'rejected', reason: 'unknown-grant', id: 'vault-row' });
});

test.each([
  ['empty grants', []],
  ['only the other slot grant', ['theme-pack:apply' as const]],
])('a home-row manifest without its slot grant is rejected as no-grant (%s)', (_name, grants) => {
  const record = homeRowRecord({ manifest: { grants } });
  expect(registerClientPlugin(record)).toStrictEqual({ outcome: 'rejected', reason: 'no-grant', id: 'vault-row' });
  expect(clientPlugins()).toStrictEqual([]);
});

test('a theme-pack manifest is rejected as no-grant when it only carries the home-row grant', () => {
  const record = themePackRecord({ manifest: { grants: ['home-row:read'] } });
  expect(registerClientPlugin(record)).toStrictEqual({ outcome: 'rejected', reason: 'no-grant', id: 'demo-themes' });
});

test('a contribution shaped for another slot than the manifest is rejected as slot-mismatch', () => {
  const record: ClientPluginRecord = {
    manifest: manifest({}),
    contribution: { slot: 'theme-pack', theme: { id: 'mismatch', title: 'Mismatch', base: 'dark' } },
  };
  expect(registerClientPlugin(record)).toStrictEqual({ outcome: 'rejected', reason: 'slot-mismatch', id: 'vault-row' });
});

test('a second record with an id already registered is rejected as duplicate-id', () => {
  expect(registerClientPlugin(homeRowRecord())).toStrictEqual({ outcome: 'ok' });
  expect(registerClientPlugin(homeRowRecord({ rowId: 'another-row' }))).toStrictEqual({
    outcome: 'rejected',
    reason: 'duplicate-id',
    id: 'vault-row',
  });
  expect(clientPlugins().length).toStrictEqual(1);
});

test('validation runs before the duplicate check: an invalid duplicate reads as its own reason', () => {
  expect(registerClientPlugin(homeRowRecord())).toStrictEqual({ outcome: 'ok' });
  expect(registerClientPlugin(homeRowRecord({ manifest: { version: '2.0' } }))).toStrictEqual({
    outcome: 'rejected',
    reason: 'bad-version',
    id: 'vault-row',
  });
});

test('consent enables exactly the registered records it names, keeping the first consent', () => {
  expect(registerClientPlugin(homeRowRecord())).toStrictEqual({ outcome: 'ok' });
  expect(registerClientPlugin(themePackRecord())).toStrictEqual({ outcome: 'ok' });
  const enabled = enabledClientPlugins([
    { manifestId: 'demo-themes', grantedAt: '2026-10-07T09:00:00Z' },
    { manifestId: 'unknown-plugin', grantedAt: '2026-10-07T09:01:00Z' },
    { manifestId: 'vault-row', grantedAt: '2026-10-07T09:02:00Z' },
    { manifestId: 'vault-row', grantedAt: '2026-10-07T09:03:00Z' },
  ]);
  expect(enabled).toStrictEqual([
    {
      manifest: {
        id: 'vault-row',
        title: 'Vault',
        version: '1.0.0',
        plane: 'client',
        slot: 'home-row',
        grants: ['home-row:read'],
      },
      contribution: {
        slot: 'home-row',
        row: { id: 'vault-row-1', title: 'From the vault', kind: 'custom', albums: ['demo-album-09'] },
      },
      consent: { manifestId: 'vault-row', grantedAt: '2026-10-07T09:02:00Z' },
    },
    {
      manifest: {
        id: 'demo-themes',
        title: 'Vault',
        version: '1.0.0',
        plane: 'client',
        slot: 'theme-pack',
        grants: ['theme-pack:apply'],
      },
      contribution: { slot: 'theme-pack', theme: { id: 'demo-themes-oled', title: 'Oled demo', base: 'oled' } },
      consent: { manifestId: 'demo-themes', grantedAt: '2026-10-07T09:00:00Z' },
    },
  ]);
});

test('an empty registry lists nothing and enables nothing, whatever the consents say', () => {
  expect(clientPlugins()).toStrictEqual([]);
  expect(enabledClientPlugins([{ manifestId: 'vault-row', grantedAt: '2026-10-07T09:00:00Z' }])).toStrictEqual([]);
});

test('clearing the registry empties it, and an id it held can register again', () => {
  expect(registerClientPlugin(homeRowRecord())).toStrictEqual({ outcome: 'ok' });
  clearClientPlugins();
  expect(clientPlugins()).toStrictEqual([]);
  expect(registerClientPlugin(homeRowRecord())).toStrictEqual({ outcome: 'ok' });
});
