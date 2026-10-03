# 7. Identity and sessions

Date: 2026-10-03
Status: proposed. Drafted by WP-125 for the owner's acceptance. Its input
is the owner's answer to register decision D-06 on 2026-10-02: no
passwords, TOTP or emailed codes; passkeys, OIDC and approval from a
signed-in phone, with the recovery ladder; the recommendation accepted
([decisions](../decisions.md#owner-answers-2026-10-02)). That answer
settles the model, not this text. The owner accepts or edits the record
when reviewing the wave-0 pull request into `main` (D-01), and this line
then says so, with the date. Extends decisions 6 and 7 of
[record 1](0001-architecture.md) without changing them.

This is the identity architecture record that SEC-STD-006 requires before
any server code stores a user.

## Context

Record 1 commits to no central account, passkeys and OIDC, device-bound
keys, random IDs, per-object authorisation and short-lived signed stream
URLs. The [security baseline](../security/README.md) designs the rest of
the identity model and is written as if there are no passwords. SEC-STD-006
asks one record to settle that question, to fix one table of session
lifetimes, and to say where identity data lives, before the identity store
(WP-046) or any other package that stores a user may start.

Passwords are how media-server accounts fall: the plex.tv breaches of 2022
and 2025, Navidrome's September 2026 brute-force advisory, and Emby's 2023
compromise through passwordless sign-in for "local" clients
([identity and access](../security/identity-and-access.md#summary)). A
password fallback would become the weakest path, and ASVS 6.1.3 requires
every path to be equally strong.

The owner also adopted the smaller R1 of the register (D-10). OIDC
sign-in, music share links and a second administrator arrive in point
release R1.2. The release placements below follow that.

## Decisions

### 1. No passwords

Gunmetal offers no account passwords, security questions, emailed or
texted codes or links, and no TOTP, for sign-in or for recovery, in any
release (SEC-IAM-025). ACC-052, ACC-053 and API-AUTH-05 are No.
SEC-TM-013, SEC-HIS-045 and SEC-STD-009 stay withdrawn. Bringing any of
them back needs a new record that supersedes this decision, and
SEC-STD-009's text returns with it.

A secret that a person chooses is never an account credential. R1 has
none: the only one in the baseline, the optional share-link password,
ships with music share links in R1.2 (SEC-API-097, SEC-STD-008) and is
hashed as [record 9](0009-cryptography.md) says. Backups need no
passphrase, because the recovery kit holds a key (SEC-PRV-040), so
SEC-STD-024 does not bind in R1. The app passwords that legacy music apps
get from R2 are random keys the server generates, scoped below
administrator, never account credentials (SEC-IAM-083 to SEC-IAM-085).

### 2. Principals

Every request resolves to exactly one principal of a closed set, or is
refused; a request with credentials for more than one principal is refused
(SEC-IAM-002). Credentials, accounts and profiles are separate entities
from the first release (SEC-IAM-001).

| Principal | Authenticates with | Ceiling | From |
|---|---|---|---|
| Owner, exactly one (SEC-IAM-003) | Passkey; recovery codes and host recovery enrol a new passkey | Everything, including the owner-only actions in decision 6 | R1 |
| Administrator | Passkey; OIDC from R1.2, which never satisfies a fresh-uv action (SEC-IAM-107) | The administrator preset; never an owner-only action (SEC-IAM-075) | R1.2 (ACC-040); until then the owner is the only administrator |
| Member | Passkey; a paired browser; OIDC from R1.2; a device key from R2 | Granted libraries | R1 |
| Guest | Passkey enrolled through an invitation; OIDC from R1.2 | Only the libraries the invitation names (SEC-IAM-080) | R1 |
| Device: paired browser | A non-extractable Web Crypto key (SEC-IAM-108) | The account's capabilities minus every administrator capability, device approval and account-security change | R1 |
| Device: native personal or household device | A non-exportable device key (SEC-IAM-048) | Capped by device class and key protection (SEC-IAM-049) | R2 |
| Managed profile | No credential; reached from a household device or a guardian's account | Granted libraries through a content policy | R2 |
| API client | API key or app password | A declared scope, never administrator (SEC-IAM-083) | R2 |
| Plugin | Its owner-approved manifest | The manifest, never an identity capability (SEC-IAM-086) | R2 |
| Peer server | Not before a federation record (SEC-IAM-081) | Named data flows only | Later |
| Anonymous link holder | A share-link secret | One object (SEC-API-097) | R1.2 |

A request with no valid credential is anonymous and reaches only the
reviewed public routes (SEC-IAM-067). Checks test capabilities, never role
names; roles are presets of capabilities (SEC-IAM-074), and the effective
capabilities are the intersection of the account's capabilities, the
device ceiling, the session state and the token scope
([identity design, section 3](../security/identity-and-access.md#3-capabilities-presets-and-ceilings)).
Authorisation is one deny-by-default pure function in the core
(SEC-IAM-068).

### 3. Sign-in methods

- **Passkeys (R1).** WebAuthn Level 3, discoverable credentials, user
  verification required, attestation "none", and a relying-party ID fixed
  at the claim to the configured origin's host (SEC-IAM-018 to
  SEC-IAM-021). The algorithms accepted are those of record 9: ES256 and
  EdDSA. Sign-in is usernameless and does not reveal whether an account or
  credential exists (SEC-IAM-022). An account may hold several passkeys,
  and the owner must always hold at least one (SEC-IAM-023, SEC-IAM-024,
  SEC-IAM-107).
- **Browser pairing (R1).** A browser that cannot use a passkey is
  approved from one of the person's signed-in personal-class devices
  (ACC-062) under SEC-IAM-056 to SEC-IAM-060. The approval always starts
  on the approving device, so nothing an outsider does can make a prompt
  appear (SEC-STD-027). The paired browser is a limited-class device that
  can play but never administer, approve other devices or change account
  security, and renews its session only by signing a fresh server
  challenge (SEC-IAM-108).
- **OIDC (R1.2).** The authorisation code flow with PKCE, state and nonce,
  the server as a confidential client, identities keyed only by (issuer,
  subject), auto-registration off by default, claims never conferring the
  owner role, and administrator claim mapping off until the owner turns
  it on per provider (SEC-IAM-026 to SEC-IAM-036, SEC-STD-025). An OIDC
  sign-in alone never satisfies an owner-only or fresh-uv action
  (SEC-IAM-107).
- **Device keys and TV device authorisation (R2)** follow SEC-IAM-048 to
  SEC-IAM-055 and the client security record that SEC-STD-039 requires.
- **Not built without a record of its own:** federation between servers
  (SEC-IAM-081), LDAP or any directory sign-in (SEC-STD-010), sign-in from
  proxy identity headers, Tailscale's included (SEC-NET-023), and
  delegated access for third-party apps; Gunmetal is not an OAuth
  authorisation server or an OpenID Provider (SEC-STD-026).

Every authentication pathway goes through the one credential verifier,
with one pathway inventory and one limiter (SEC-TM-014, WP-064).

### 4. Claiming a server

A server ships with no account, password, token or key (SEC-IAM-005).
Until it is claimed it serves only the claim page, its assets and a health
check (SEC-IAM-006). The claim code has 128 bits, is shown only through
host-side channels, lasts 24 hours and is never rotated by failed attempts
(SEC-IAM-007). It is accepted only from loopback or from a secure context
on a configured origin (SEC-IAM-008). The claim consumes the code, creates
the owner and enrols the owner's first passkey in one transaction, after
which setup is gone for good (SEC-IAM-009, SEC-OPS-006).

The relying-party ID is fixed at the claim, so the server's HTTPS path
([record 8](0008-https-and-naming.md)) is chosen before it.

### 5. One table of session lifetimes

The single table is the security parameters table in
[threat-model.md](../security/threat-model.md#security-parameters)
(SEC-TM-072). This record adopts it unchanged. Its session rows, copied
as they stand on 2026-10-03, are:

| Key | Value | Owner |
|---|---|---|
| session.browser | 7 days idle, 30 days absolute; opaque 256-bit token stored hashed | SEC-IAM-041, SEC-IAM-037 |
| session.paired_browser | As session.browser; limited class; renewed by a Web Crypto key signature | SEC-IAM-108 |
| session.admin | Separate `__Host-` cookie, SameSite=Strict; 15 minutes idle, 1 hour absolute | SEC-IAM-041 |
| step_up.fresh_uv | User verification within 5 minutes; route tags none, elevated, fresh-uv | SEC-IAM-041 |
| token.native_access | 10 minutes, sender-constrained, no refresh tokens | SEC-IAM-050 |
| cookie.session | `__Host-gm_session`; Secure, HttpOnly, SameSite=Lax, Path=/, no Domain | SEC-API-032 |
| capability.stream | Item duration plus 10 minutes, 4 hours at most, refreshed transparently | SEC-API-027 |
| capability.artwork | 1 hour, aligned to a time bucket | SEC-API-027 |

If the parameters table changes, it wins over this copy. The rules around
it:

- First-party session tokens are opaque random values, stored on the
  server only as keyed hashes (HMAC-SHA-256 under the session-hash key of
  record 9, which is both the SHA-256 hash of SEC-IAM-037 and the keyed
  hash of SEC-OPS-016), and checked by lookup on every request. There are
  no first-party JWTs.
- A new token is issued at sign-in, at elevation and at every profile
  switch, and the previous one stops working (SEC-IAM-038).
- Revocation bites on the next request, and open streams and sockets close
  within 5 seconds (SEC-IAM-043, SEC-TM-028).
- The owner may shorten these lifetimes but never lengthen them (ACC-079).
- Media and artwork are fetched only by capability URLs bound to a live
  session and re-checked on every request, never by cookie (SEC-API-026 to
  SEC-API-029).
- Household devices (R2) stay enrolled until revoked and go dormant after
  30 days unused (household_device, SEC-IAM-109). That is the long
  playback-session deviation the baseline accepts (threat-model OD-8).

Three lifetimes have no row in the parameters table yet (register D-81):
the 30-minute idle end of a browser marked shared (SEC-CLI-010), the
lifetime of an administrator's recovery link (SEC-IAM-091 says only
"short-lived"), and the lifetime of the session a recovery code opens.
This record does not invent them. The baseline's owner adds the rows
before WP-062, WP-106 and the client build them.

### 6. Host-equivalent actions

These actions carry the fresh-uv tag (SEC-IAM-041), are audit-logged and
announced to every administrator, and are never satisfied by an OIDC
sign-in alone (SEC-TM-017, SEC-IAM-107).

| Action | Who | From |
|---|---|---|
| Trusted proxies and whether each is a private overlay or public | Owner | R1 |
| Posture and remote administration | Owner | R1 |
| TLS and naming settings: certificates, the ACME domain and its DNS credential | Owner | R1 |
| Egress policy: the update check, metadata providers (R1.1), network grants | Owner | R1 |
| Backup download and restore | Owner | R1 |
| Ownership transfer, confirmed by both people (SEC-IAM-003) | Owner | R1 |
| Rotating every server key (SEC-OPS-018) | Owner | R1 |
| Security settings: origins, session lifetimes (shorter only), rate limits | Owner | R1 |
| Creating, promoting, demoting or recovering administrators | Owner | R1.2 |
| Configuring an OIDC provider and its claim mapping | Owner | R1.2 |
| Plugin installs and grants; enabling adapters | Owner | R2 |
| Adding or removing library roots; browsing the file system to choose one | Owner or administrator | R1 |

Some powers never reach the network at all: owner recovery, reading the
claim code, and the host-only settings that never enter a backup
(developer mode, external program paths, listen sockets; SEC-OPS-041).

### 7. Recovery

Each rung is used only when the one above it has failed (SEC-IAM-089 to
SEC-IAM-092, SEC-IAM-106).

1. **Another passkey.** Synced passkeys survive a lost phone, and the
   interface keeps asking while an account holds only one.
2. **Recovery codes.** Ten single-use codes of 80 bits, stored as peppered
   hashes, offered to the owner and administrators when they enrol and
   available to everyone under Account > Recovery. A code opens a session
   that can only enrol a new passkey. For the owner they are strongly
   prompted but skippable, because host recovery exists (identity OD-12);
   the owner's codes are printed in the recovery kit with the backup key
   and the fingerprint of the server's identity key (SEC-PRV-040,
   [record 10](0010-backup-archives.md)).
3. **The identity provider**, for OIDC accounts (R1.2).
4. **An administrator's recovery link.** Administrators issue them for
   members and guests, the owner for administrators, and nobody for the
   owner. A link is redeemed in person or on a device the person already
   approved.
5. **Host recovery, for the owner only.** `gunmetal admin recover` on the
   host talks to the server over a local socket only the service user can
   open, prints an enrolment link valid for 15 minutes, and alerts every
   administrator (SEC-IAM-092, SEC-OPS-009). The command takes the name
   ADM-034 and the plan's command-line registry already use, which, once
   the owner accepts this record, settles register D-33.

A passkey enrolled through a recovery code or an administrator's link
starts the recovery hold: 72 hours by default, 24 to 72 at the owner's
choice. During the hold it cannot remove other credentials, elevate,
export history or create invitations; every existing device can end the
hold with one tap; and after an administrator's link the person's history
stays hidden from the new passkey until the hold ends (recovery.hold,
SEC-IAM-106). Recovery never goes through email, SMS, security questions
or a vendor (SEC-IAM-025).

### 8. Where identity data lives

- **The identity store** is `durable/identity.db`, an SQLite file in the
  durable store that [record 3](0003-durable-user-state.md) defines,
  opened only through the one connection opener in `gunmetal-fs` (WP-126).
  It holds accounts, credentials' public material, session token hashes,
  devices, grants, invitations, recovery-code hashes, settings and the
  public-ID mapping (register D-04). Cache rebuilds never touch it
  (SEC-IAM-004, SEC-TM-051).
- **Keys and other secrets** are 0600 files in the 0700 `secrets/`
  directory (SEC-OPS-012) and never leave the secrets crate (record 9).
  Secrets the server replays to others are sealed in the identity store
  under the vault key (SEC-OPS-017).
- **The security log** is the audit log in `durable/audit/`, separate from
  listening history and from diagnostic logs (SEC-OPS-020).
- **Backups** include all of the above, encrypted and signed
  ([record 10](0010-backup-archives.md), SEC-IAM-105).
- **Browsers** keep no session or API token in web storage (SEC-IAM-017,
  SEC-API-032).

## Consequences

- Once the owner accepts this record, the identity store (WP-046) and
  every package that stores a user may build to it, together with record
  3. Until then SEC-STD-006 keeps them waiting. WP-062, WP-063, WP-064,
  WP-080, WP-081, WP-106 and WP-120 follow it. OIDC (WP-096) and share
  links (WP-134) are R1.2.
- A person with no passkey-capable device and no phone needs a hardware
  security key or help from someone in the household. D-06's
  recommendation states that trade-off, and the owner accepted the
  recommendation.
- Changing the server's origin breaks every passkey, so members re-enrol
  as record 8 describes (SEC-NET-072).
- The docs lint (WP-127) fails while SEC-IAM-025 and any live password or
  TOTP requirement or feature row are both present (SEC-STD-006).
- Found while checking this record, for the baseline's owner (this
  package does not edit `docs/security/`): the gap analysis in
  `standards-coverage.md` still describes the password conflict as open
  (its summary item 1, the "Passwords and TOTP" row of its conflict table,
  and coverage rows for ASVS 6.2, ASVS 6.7 and A07 that list SEC-TM-013
  among the citing requirements), and an incident row in
  `rival-security-history.md` cites the withdrawn SEC-HIS-045. None is a
  live requirement, but SEC-STD-006 asks every file to agree. The three
  missing parameter rows in decision 5 are the other open item.

## Requirement check

Review record, dated 2026-10-03. This is the author's check, written by
the coding agent working on WP-125; no person has reviewed it yet. The
package's pull request merges into `wave-0` through the integrator agent
once the gate passes, with no human review (D-01). The owner's review of
the wave-0 pull request into `main` confirms or edits this check, and
only then does it stand as the dated review record for SEC-STD-006.

| Requirement | What it asks | Result |
|---|---|---|
| SEC-STD-006 | One record decides whether account passwords and TOTP exist in R1 | Met: decision 1, no passwords or TOTP in any release |
| SEC-STD-006 | It fixes one session-lifetime table, the parameters table, with native access tokens of 10 minutes under SEC-IAM-050 | Met: decision 5 adopts the parameters table; token.native_access is 10 minutes |
| SEC-STD-006 | Every file in `docs/security/` and `docs/features/` agrees with it | Met for every requirement and feature row: ACC-052 and ACC-053 are No; SEC-TM-013, SEC-HIS-045 and SEC-STD-009 are withdrawn and cited only as redirects or in the control-ownership table; no live requirement offers a password, TOTP, an emailed code or a security question. The stale narrative listed under Consequences remains for the baseline's owner |
| SEC-STD-006 | Requirements it withdraws are marked withdrawn and no live requirement cites them | Met: this record withdraws nothing new; the rows withdrawn on 2026-10-02 are marked |
