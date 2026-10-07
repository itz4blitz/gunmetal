import type { ClientPluginManifest, PluginConsent } from '../../../ports/src/plugins/types.ts';
import {
  grantsSatisfy,
  isClientGrant,
  isClientPluginSlot,
  requiredGrantForSlot,
} from '../../../ports/src/plugins/types.ts';
import type { PluginRecord } from '../../../ports/src/plugins/types.ts';
import type { ThemeId } from '../shell/theme.ts';

/**
 * A home row a plugin declares: typed data Gunmetal's clients draw on every platform
 * (INT-081), never injected script (INT-074). `recent` and `loved` rows are filled by the
 * client from the person's own history; a `custom` row names the albums it shows.
 */
export type HomeRowContribution =
  | { readonly id: string; readonly title: string; readonly kind: 'recent' }
  | { readonly id: string; readonly title: string; readonly kind: 'loved' }
  | { readonly id: string; readonly title: string; readonly kind: 'custom'; readonly albums: readonly string[] };

/** A theme pack a plugin declares: themes built on Gunmetal's own four themes, as data. */
export type ThemePackContribution = {
  readonly id: string;
  readonly title: string;
  readonly base: ThemeId;
};

/** What a plugin contributes, tagged by the slot it fills. The drawer of a slot reads its tag. */
export type ClientContribution =
  | { readonly slot: 'home-row'; readonly row: HomeRowContribution }
  | { readonly slot: 'theme-pack'; readonly theme: ThemePackContribution };

/** One client plugin: its manifest, its plain-data contribution, and its consent once granted. */
export type ClientPluginRecord = PluginRecord<ClientContribution>;

/** A registered plugin the person has consented to: the record plus the consent that enables it. */
export type EnabledClientPlugin = ClientPluginRecord & { readonly consent: PluginConsent };

/** Why the registry refused a record. Every reason is a data problem, never a runtime one. */
export type PluginRejectionReason =
  | 'bad-plane'
  | 'bad-id'
  | 'bad-version'
  | 'unknown-slot'
  | 'unknown-grant'
  | 'no-grant'
  | 'slot-mismatch'
  | 'duplicate-id';

/** The result of admitting one record: it is registered, or it is refused with a reason. */
export type PluginRegistration =
  | { readonly outcome: 'ok' }
  | { readonly outcome: 'rejected'; readonly reason: PluginRejectionReason; readonly id: string };

// Admission policy: ids are lowercase words joined by single hyphens; versions are three
// whole numbers separated by dots. Both shapes are pinned by the registry tests.
const PLUGIN_ID = /^[a-z0-9]+(-[a-z0-9]+)*$/;
const PLUGIN_VERSION = /^\d+\.\d+\.\d+$/;

let registered: ClientPluginRecord[] = [];

function reject(reason: PluginRejectionReason, id: string): PluginRegistration {
  return { outcome: 'rejected', reason, id };
}

/**
 * Admit one plugin into the registry. The manifest is untrusted input and is validated in a
 * fixed order: plane, id, version, slot, grants, slot grant, contribution slot, duplicate id.
 * Nothing is evaluated, nothing is fetched, nothing is imported: a contribution is plain
 * data the client draws when it is consented (ADR 22).
 */
export function registerClientPlugin(record: ClientPluginRecord): PluginRegistration {
  const manifest: ClientPluginManifest = record.manifest;
  if (manifest.plane !== 'client') {
    return reject('bad-plane', manifest.id);
  }
  if (!PLUGIN_ID.test(manifest.id)) {
    return reject('bad-id', manifest.id);
  }
  if (!PLUGIN_VERSION.test(manifest.version)) {
    return reject('bad-version', manifest.id);
  }
  if (!isClientPluginSlot(manifest.slot)) {
    return reject('unknown-slot', manifest.id);
  }
  if (!manifest.grants.every((grant) => isClientGrant(grant))) {
    return reject('unknown-grant', manifest.id);
  }
  if (!grantsSatisfy([requiredGrantForSlot(manifest.slot)], manifest.grants)) {
    return reject('no-grant', manifest.id);
  }
  if (record.contribution.slot !== manifest.slot) {
    return reject('slot-mismatch', manifest.id);
  }
  if (registered.some((entry) => entry.manifest.id === manifest.id)) {
    return reject('duplicate-id', manifest.id);
  }
  registered.push(record);
  return { outcome: 'ok' };
}

/** Everything registered, in registration order. The list is data; drawing it is a surface's job. */
export function clientPlugins(): readonly ClientPluginRecord[] {
  return registered;
}

/**
 * The registered plugins the person has consented to, in registration order, each with its
 * consent attached. Where several consents name one manifest, the earliest counts. An empty
 * consent ledger — R1's shipped state — enables nothing.
 */
export function enabledClientPlugins(consents: readonly PluginConsent[]): readonly EnabledClientPlugin[] {
  const enabled: EnabledClientPlugin[] = [];
  for (const record of registered) {
    const consent = consents.find((entry) => entry.manifestId === record.manifest.id);
    if (consent !== undefined) {
      enabled.push({ ...record, consent });
    }
  }
  return enabled;
}

/** Empty the registry. The composition root calls it before registering a build's plugins. */
export function clearClientPlugins(): void {
  registered = [];
}
