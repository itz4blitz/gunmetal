# 8. HTTPS and naming

Date: 2026-10-03
Status: accepted. The owner answered register decision D-07 on 2026-10-02,
and the answer differs from the register's recommendation: R1 gets HTTPS
through the owner's own domain (automatic certificates), a tailnet, or the
same machine; the project-run per-server name service is not in R1 and
moves to a later release. Remote access in R1 is the owner's own reverse
proxy or a tailnet; built-in remote access over iroh arrives in R2
([decisions](../decisions.md#owner-answers-2026-10-02)).

## Context

Passkeys work only in a secure context: HTTPS on a name, with a
certificate the browser trusts, or localhost. A WebAuthn relying-party ID
must be a domain, not an IP address, and Chrome refuses WebAuthn on pages
with certificate errors
([identity design, section 5](../security/identity-and-access.md#5-a-secure-context-is-a-prerequisite)).
No credential may cross cleartext (first principle 4).

The baseline recommended that the project run a per-server name service,
as plex.direct does, so that households without a domain get HTTPS by
default. The owner chose not to run one in R1. A project service would be
a soft central dependency that knows which servers exist; it needs its
zone on the Public Suffix List, Certificate Transparency monitoring, a
legal home for project services (register D-41) and a second keyholder
(register D-62) before it could launch. R1 is aimed at people who can
follow a recipe for a domain, a reverse proxy or a tailnet, and at the
browser on the server itself.

## Decisions

### 1. One cleartext rule

Over plain HTTP the server serves the web client and API only to loopback.
Every other peer gets a static redirect or help page that sets no cookie
and accepts no credential, whatever any setting says (SEC-NET-001). There
are no self-signed certificates and no "trust anyway" button, and when no
valid certificate is available the server never falls back to plaintext;
it says so on the console, in the log and on the help page (SEC-NET-005).
Every TLS listener negotiates only TLS 1.3, or TLS 1.2 with ECDHE and an
AEAD suite (SEC-NET-002), through rustls with the provider
[record 9](0009-cryptography.md) chooses.

### 2. Three ways to HTTPS in R1

Each path is documented with a copy-paste recipe and tested
(SEC-NET-013).

| Path | How | Where TLS ends | Test |
|---|---|---|---|
| The owner's own domain | One of: the server's built-in ACME client obtaining and renewing a certificate by DNS-01 (ACC-099, ADM-022); a certificate and key the owner supplies, reloaded when the files change (ACC-098); or the owner's reverse proxy (Caddy, nginx, Traefik, Cloudflare Tunnel) declared as a trusted proxy, public by default (SEC-NET-019) | The server, or the owner's proxy | CI: built-in ACME against Pebble with a test DNS server; each proxy run for real in front of the server (SEC-NET-022) |
| A tailnet | Tailscale Serve, or another overlay that issues HTTPS names, in front of a listener used only by the overlay and declared a private overlay (SEC-NET-019); or the server serving the tailnet name's certificate as an owner-supplied certificate | The overlay, or the server | Manual release checklist, because it needs a real tailnet |
| The same machine | `http://localhost:<port>` on the host itself, or through an SSH tunnel | Nowhere: loopback is a secure context | CI: a headless browser asserts a secure context and completes a passkey ceremony |

A certificate the server activates must validate, with its complete chain
and without fetching missing intermediates, against the bundled WebPKI
roots (SEC-NET-003). IP-address certificates do not help: public CAs issue
them only for public addresses.

### 3. The origin is chosen before the claim

The owner's first passkey is bound to the relying-party ID fixed at the
claim ([record 7](0007-identity-and-sessions.md), SEC-IAM-018), so the
install chooses its origin in the host-side configuration before the
claim. A passkey made on localhost works only on localhost: when a public
name is configured, the claim page sends the owner there before enrolment,
and it says plainly that the chosen name is permanent for passkeys
(SEC-NET-072).

### 4. Outbound traffic before the claim

Before the claim the server contacts nothing, except that an install
configured for built-in ACME on its own domain may reach the configured
certificate authority and publish the DNS-01 challenge (the egress
inventory's ACME row; SEC-TM-048, SEC-TM-075, SEC-OPS-007). Tailnet,
supplied-certificate, reverse-proxy and localhost installs contact nothing
at all before the claim.

The DNS-01 challenge is published by DNS-PERSIST-01 where the configured
CA offers it, so the server holds no DNS credential, and otherwise through
one DNS update interface the owner configures, whose credential is a
replayed secret sealed in the vault (SEC-OPS-017). That update destination
is not yet a row of the egress inventory. WP-101 proposes the interface,
and the baseline's owner adds its row before WP-101 ships.

### 5. Certificates and their keys

Renewal is automatic, follows ACME Renewal Information when the CA offers
it and otherwise happens by two-thirds of the certificate's lifetime, works
for lifetimes from 6 to 398 days, and keeps serving the current
certificate until its replacement validates (SEC-NET-004, SEC-TM-010). The
owner is alerted 30 and 7 days before expiry (SEC-NET-072). The TLS key
and the ACME account key are generated on the server from the CSPRNG,
kept as 0600 files under `secrets/tls/`, never logged and never sent to
anyone (SEC-NET-006); record 9 lists them in the cryptographic inventory.
HTTPS responses under a hostname carry `Strict-Transport-Security` of at
least one year, with `includeSubDomains` only when the owner confirms they
control every subdomain (SEC-API-038).

### 6. Hosts and links

The server answers 421 to a request whose Host is not one of its
configured names (SEC-NET-014), builds every absolute URL from the
configured canonical origin and never from Host or forwarding headers
(SEC-NET-015), and serves the web client itself, on the same origin as its
API (SEC-NET-058, SEC-IAM-014). There is no project-hosted web app.

### 7. Remote use in R1

- Through the owner's reverse proxy: requests get internet posture, with
  stricter limits, no setup, and no administration unless the owner turns
  remote administration on (SEC-NET-019, SEC-NET-045).
- Through a tailnet: a private overlay that proves itself on every
  request, or a direct tailnet address, which counts as local only when
  one of the server's own interfaces holds an address in the same overlay
  prefix (SEC-NET-019, SEC-NET-024).
- A forwarded port or a global IPv6 address reaches only the home-posture
  help page and raises an exposure alert (SEC-NET-024, SEC-NET-027). The
  server never asks the router for a port mapping for HTTP (SEC-NET-030).
- Built-in remote access over iroh, relays and the browser edge arrive in
  R2. This schedules record 1's decision 7; it does not change it.

### 8. No project name service in R1

R1 ships no part of the name service: no label registration, no naming or
Certificate Transparency monitoring purpose in the egress client, no
project zone. Before it is built, in whatever release the owner chooses,
it needs a record of its own that:

- amends record 1's decisions 7 (no central account) and 9 (Cloudflare
  hosts only the docs and the landing page), because it is a project
  service every default install would use;
- starts from the baseline's design: a 128-bit random label generated on
  the server and derived from nothing else, answers only for labels that
  encode local addresses, a CAA record bound to the server's own ACME
  account, the zone on the Public Suffix List before launch, Certificate
  Transparency monitoring, and zone keys kept offline under two-person
  control (SEC-NET-010 to SEC-NET-012, SEC-NET-069 to SEC-NET-071,
  SEC-HIS-061, SEC-STD-016);
- fixes the label scheme, which this record therefore does not; and
- names the release and settles the legal home (D-41) and the keyholders
  (D-62).

### 9. Changing the origin

Passkeys belong to the origin they were made on. Moving a household to a
new origin is a documented, tested migration that re-enrols members by
device-to-device approval or by recovery links under their rules
(SEC-NET-072, SEC-IAM-091, SEC-IAM-106), and a restore under a new address
warns before it changes anything (WP-109). A separate origin for the admin
interface (register D-24) is optional where the owner's certificate covers
a second name; R1 does not require it.

## Consequences

- R1's web client suits households that have, or can follow a recipe for,
  a domain, a reverse proxy or a tailnet, and anyone using the browser on
  the server itself. Others wait for the later name service or the R2
  native apps, which reach the server over iroh and pin its key rather
  than relying on a certificate.
- No project service exists in R1, and a fresh install contacts nothing
  unless its owner configured built-in ACME.
- WP-129 (the name service) and WP-135 (the naming client and CT
  monitoring) leave R1. WP-073, WP-080, WP-101 and WP-132 build to this
  record, and WP-101 drops its name-service branch.
- Documents that still describe the name service as R1, for their owners
  to realign (this package edits only its own records):
  - in `docs/security/`: the release-scope table and the egress
    inventory's naming and CT-monitoring rows in `threat-model.md`;
    SEC-NET-010 to SEC-NET-012 and SEC-NET-069 to SEC-NET-071, whose
    Release must move with their surface (SEC-TM-074); the wording of
    SEC-OPS-007, SEC-TM-048 and SEC-NET-032; the name-service zone in
    SEC-STD-016; network OD-1 and README decision 2;
  - in `docs/features/`: ADM-023 (R1 to a later release), ADM-021 and
    ACC-098, which offer the name service as a path;
  - in `docs/plan/`: WP-101's scope, WP-129, WP-135, owner decision 21
    and the waves table.
- SEC-OPS-007 names the name service as the only outbound contact allowed
  before the claim, and its verification expects none at all with
  own-domain naming, while the egress inventory, which owns egress
  defaults (SEC-TM-075), allows ACME before the claim for own-domain
  installs. Decision 4 follows the inventory, limited to an owner-configured
  CA and DNS update. The baseline's owner reconciles the two texts.

## Requirement check

Review record, dated 2026-10-03. Written by the record's author, a coding
agent working on WP-125, and confirmed by the human review of its pull
request (AGENTS.md). This record is the "HTTPS and naming" record of the
baseline's "add to the repository now" item 17; it does not by itself
verify a requirement, so this table checks that it agrees with the ones it
relies on.

| Requirement | Agreement |
|---|---|
| SEC-NET-001, SEC-NET-005 | Decision 1 restates the one cleartext rule by reference and adds no exception |
| SEC-NET-002, SEC-NET-003, SEC-NET-004, SEC-NET-006, SEC-TM-010 | Decisions 1, 2 and 5 |
| SEC-NET-013 | Decision 2 keeps all three paths, with the CI jobs and the manual Tailscale check the requirement names |
| SEC-IAM-008, SEC-IAM-018 | Decision 3: the claim happens on loopback or a configured HTTPS origin, and the relying-party ID is fixed then |
| SEC-OPS-007, SEC-TM-048, SEC-TM-075 | Decision 4 follows the egress inventory; the disagreement with SEC-OPS-007's wording is listed under Consequences |
| SEC-NET-010 to SEC-NET-012, SEC-NET-069 to SEC-NET-071 | Not applicable in R1, because their surface does not exist; decision 8 carries them into the name service's own record |
| SEC-NET-014, SEC-NET-015, SEC-NET-058, SEC-API-038 | Decisions 5 and 6 |
| SEC-NET-019, SEC-NET-024, SEC-NET-027, SEC-NET-030, SEC-NET-045 | Decision 7 |
