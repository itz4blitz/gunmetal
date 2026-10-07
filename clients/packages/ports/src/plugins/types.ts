// The client-plane plugin contract (INT-081, ADR 22): manifests, grants and consent as pure
// data. R1 loads nothing — a manifest is data a client draws, never code that runs (INT-054,
// INT-074). The registry that admits records of these types lives in packages/ui/src/plugins.

/** The two slots a client plugin may fill. New slots join this union, never bypass it. */
export type ClientPluginSlot = 'home-row' | 'theme-pack';

/**
 * A permission a client plugin can ask for. The union is closed and additive: an R2 record
 * adds new grants; an unknown grant in a manifest is untrusted input, not a permission.
 */
export type ClientGrant = 'home-row:read' | 'theme-pack:apply';

/** The declaration a client plugin carries. Every field is data; none is executable. */
export type ClientPluginManifest = {
  /** Lowercase, hyphen-separated registry id, unique per client build. */
  readonly id: string;
  /** The plugin's name as the person sees it. */
  readonly title: string;
  /** Three whole numbers separated by dots; the registry pins the shape. */
  readonly version: string;
  /** A client manifest always says 'client'; anything else is a server-plane concern. */
  readonly plane: 'client';
  /** The slot the plugin fills; the contribution must be shaped for the same slot. */
  readonly slot: ClientPluginSlot;
  /** The grants the plugin asks for. An absent grant means not granted (INT-055). */
  readonly grants: readonly ClientGrant[];
  /** Where the plugin's documentation lives, if anywhere. Never fetched by the client. */
  readonly homepage?: string;
};

/** One consent the person has given: this manifest was granted at this time. */
export type PluginConsent = {
  readonly manifestId: string;
  readonly grantedAt: string;
};

/** One registered plugin: its manifest, its declared contribution, and the consent once granted. */
export type PluginRecord<TContribution> = {
  readonly manifest: ClientPluginManifest;
  /** Plain data the client draws for the declared slot. Never a function, never a script. */
  readonly contribution: TContribution;
  /** Set when the person consented; absent means the plugin stays dark. */
  readonly consent?: PluginConsent;
};

const CLIENT_PLUGIN_SLOTS: readonly ClientPluginSlot[] = ['home-row', 'theme-pack'];

const CLIENT_GRANTS: readonly ClientGrant[] = ['home-row:read', 'theme-pack:apply'];

/** Whether an untrusted string names a known client-plane slot. */
export function isClientPluginSlot(value: string): value is ClientPluginSlot {
  return CLIENT_PLUGIN_SLOTS.includes(value as ClientPluginSlot);
}

/** Whether an untrusted string names a known client grant. */
export function isClientGrant(value: string): value is ClientGrant {
  return CLIENT_GRANTS.includes(value as ClientGrant);
}

/** The one grant a slot's manifest must carry: rows read the library, themes restyle the client. */
export function requiredGrantForSlot(slot: ClientPluginSlot): ClientGrant {
  return slot === 'home-row' ? 'home-row:read' : 'theme-pack:apply';
}

/**
 * Whether held grants cover every required grant. An empty requirement is satisfied by
 * anything; duplicates neither help nor hurt, because a grant is a membership, not a count.
 */
export function grantsSatisfy(required: readonly ClientGrant[], held: readonly ClientGrant[]): boolean {
  return required.every((grant) => held.includes(grant));
}
