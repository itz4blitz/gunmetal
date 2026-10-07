import { beforeEach, expect, test } from 'vitest';
import { clearClientPlugins, clientPlugins, enabledClientPlugins, registerClientPlugin } from './registry.ts';
import { fromTheVaultPlugin } from './demo-plugin.ts';

beforeEach(() => {
  clearClientPlugins();
});

test('the demo plugin is one home-row record of plain data', () => {
  expect(fromTheVaultPlugin).toStrictEqual({
    manifest: {
      id: 'from-the-vault',
      title: 'From the vault',
      version: '1.0.0',
      plane: 'client',
      slot: 'home-row',
      grants: ['home-row:read'],
    },
    contribution: {
      slot: 'home-row',
      row: {
        id: 'from-the-vault',
        title: 'From the vault',
        kind: 'custom',
        albums: ['demo-album-09', 'demo-album-12', 'demo-album-15'],
      },
    },
  });
});

test('the demo plugin is admitted by the registry and stays dark without consent', () => {
  expect(registerClientPlugin(fromTheVaultPlugin)).toStrictEqual({ outcome: 'ok' });
  expect(clientPlugins()).toStrictEqual([fromTheVaultPlugin]);
  expect(enabledClientPlugins([])).toStrictEqual([]);
  expect(enabledClientPlugins([{ manifestId: 'from-the-vault', grantedAt: '2026-10-07T10:00:00Z' }])).toStrictEqual([
    {
      manifest: {
        id: 'from-the-vault',
        title: 'From the vault',
        version: '1.0.0',
        plane: 'client',
        slot: 'home-row',
        grants: ['home-row:read'],
      },
      contribution: {
        slot: 'home-row',
        row: {
          id: 'from-the-vault',
          title: 'From the vault',
          kind: 'custom',
          albums: ['demo-album-09', 'demo-album-12', 'demo-album-15'],
        },
      },
      consent: { manifestId: 'from-the-vault', grantedAt: '2026-10-07T10:00:00Z' },
    },
  ]);
});

test('the demo row names albums the fake server ships', () => {
  const row = fromTheVaultPlugin.contribution.slot === 'home-row' ? fromTheVaultPlugin.contribution.row : undefined;
  expect(row).not.toStrictEqual(undefined);
  if (row !== undefined && row.kind === 'custom') {
    expect(row.albums.every((album) => /^demo-album-\d\d$/.test(album))).toStrictEqual(true);
  }
});
