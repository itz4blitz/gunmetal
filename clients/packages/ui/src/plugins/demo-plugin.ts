import type { ClientPluginRecord } from './registry.ts';

/**
 * The R1 demo plugin, "From the vault": one home row over albums the fake server ships
 * (demo-album-09, demo-album-12, demo-album-15). Plain data, like every client plugin
 * (INT-081): the registry stores this record, a home-row drawer would draw it, and nothing
 * in the client evaluates it — no script, no fetch, no runtime (ADR 22).
 */
export const fromTheVaultPlugin: ClientPluginRecord = {
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
};
