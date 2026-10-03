# 9. Cryptography: the allow-list and the inventory

Date: 2026-10-03
Status: accepted, through the owner's technical answers of 2026-10-03
([decision register](../decisions.md#technical-answers-to-wave-0s-package-questions-2026-10-03)),
on the input of register decision D-08, which the owner delegated on
2026-10-02, accepting its recommendation
([decisions](../decisions.md#owner-answers-2026-10-02)): aws-lc-rs if it
builds cleanly for the R1 targets, otherwise ring; ES256 and EdDSA for
passkeys; RS256 for OIDC only if the `rsa` crate passes review; `tough`
if its dependency tree passes cargo-vet. The rest of the allow-list, the
inventory and the post-quantum plan are the author's, accepted with the
key-table and version-byte fixes below.

## Context

The baseline asks for cryptography to come only from a reviewed
allow-list chosen in an architecture record (SEC-STD-019), for one place
in the code where cryptographic crates may be used and an inventory that
CI holds against it (SEC-STD-018), and for an inventory of every key on
the network paths (SEC-NET-057). Its first principle 8 is one door per
risk. The rustls provider, the AEAD nonce strategy (SEC-STD-020) and the
algorithms for passkeys, single sign-on and the update feed were left to
this record (register D-08).

The core is compiled into the web client and must not depend on the
secrets crate, and keys must never leave the secrets crate. The plan
therefore reads SEC-STD-018's "one `crypto` module" as one door in two
files (work-packages.md, owner decision 38, applied and accepted with the
remaining recommendations).

## Decisions

### 1. Where cryptography lives

- `crates/gunmetal-core/src/crypto.rs` (WP-122) holds SHA-256 and nothing
  else: the schema digest, content-identity windows, the audit hash chain
  and the backup payload hash.
- `crates/gunmetal-secrets/src/crypto/` (WP-047) holds every other
  algorithm, keyed or keyless, and the constructors of the rustls provider
  and of the server and client TLS configurations. The listener and the
  egress client receive ready-made configurations and never choose a
  provider or a verifier (SEC-NET-009).
- Keyed operations happen inside the secrets crate; other crates pass a
  message and get a tag, a signature or a sealed value back, and core
  functions take a `MacProvider` (WP-031). Verifications that need no
  secret are plain functions that take a public key.
- No other module in the workspace uses a cryptographic crate. WP-001's
  `clippy.toml` and `deny.toml` entries reject it everywhere else, and
  WP-047's xtask fails when the inventory in decision 5 and the functions
  in the two files disagree.
- **The web client** uses only the browser's WebAuthn and Web Crypto, and
  the core's SHA-256 through WASM; it ships no JavaScript cryptography
  library. Web Crypto supplies the paired browser's non-extractable ECDSA
  P-256 key (SEC-IAM-108) and, where the owner's authenticator supports the
  PRF extension, the wrap of the backup recovery key (SEC-PRV-040): a key
  derived with HKDF-SHA-256 from the PRF output and a fresh random salt,
  used for exactly one AES-256-GCM encryption. Its plaintext is the
  recovery key followed by the fingerprint of the server's `identity` key,
  so a restore that has only the owner's passkey also gets an authentic
  trust anchor ([record 10](0010-backup-archives.md), decision 6).
- **Native clients (R2)** are held to the same list; the client security
  record that SEC-STD-039 requires maps it onto each platform's keystore.

### 2. The allow-list

Only these implementations may perform cryptography. Anything else needs a
new record that supersedes this list before the crate is added, together
with the dependency checklist, cargo-deny and cargo-vet (SEC-SUP-024).

| Primitive | Implementation | Used for | From |
|---|---|---|---|
| Security randomness | `getrandom`, through one function (SEC-STD-022) | Every key, token, salt, nonce and public ID | R1 |
| SHA-256 | `sha2` (RustCrypto) | The core uses above; inside HMAC and HKDF; key fingerprints (decision 4) | R1 |
| HMAC-SHA-256 | `hmac` (RustCrypto) | Capability URLs, keyed hashes of stored secrets, the audit address commitment | R1 |
| HKDF-SHA-256 | `hkdf` (RustCrypto) | One key per purpose from the root secret (SEC-OPS-015) | R1 |
| XChaCha20-Poly1305 | `chacha20poly1305` (RustCrypto) | The vault for replayed secrets (SEC-OPS-017) | R1 |
| Ed25519 | `ed25519-dalek` | Audit checkpoints, backup signatures, the server identity key and its certificates of backup-signing keys, the update feed, EdDSA passkeys and (R1.2) EdDSA ID tokens | R1 |
| ECDSA P-256 with SHA-256 | `p256` (RustCrypto) | Verifying ES256 passkeys, paired-browser signatures and (R1.2) ES256 ID tokens; signing ACME requests and certificate requests with deterministic nonces | R1 |
| age v1 (X25519 recipients, ChaCha20-Poly1305) | `age`, or another age v1 implementation that passes the same review, built without its plugin and SSH-key features (which would start programs and add key types; feature names unverified) | Backups ([record 10](0010-backup-archives.md), SEC-OPS-042) | R1 |
| TLS | `rustls` with one provider (decision 3), `rustls-webpki`, and the bundled `webpki-roots` (SEC-NET-003) | HTTPS listener and outbound TLS, with the algorithms of decision 5's TLS table, RSA included, used only inside the provider | R1 |
| TUF 1.0 | `tough`, if its dependency tree passes cargo-vet and the checklist; otherwise a minimal client in WP-074 that verifies through the Ed25519 function above | The update and advisory feed (SEC-OPS-019, SEC-SUP-050) | R1 |
| Argon2id | `argon2` (RustCrypto), at or above 64 MiB, 3 passes and 4 lanes, with the parameters stored beside each hash (SEC-STD-024) | Share-link passwords (R1.2); profile PIN hashes (R2) | R1.2 |

Three conditions in that table:

- `tough` verifies signatures itself. If it is admitted, its signature
  backend is counted as an allow-listed implementation and must be the
  same one as the rustls provider, and it is called only from a
  `verify_feed` function in `gunmetal-secrets/src/crypto/`, so the door
  stays in two files.
- RSA is on the list only inside the TLS provider, for TLS. Both
  candidate providers verify RSA signatures, PKCS#1 v1.5 and PSS, and
  many WebPKI chains need them: Let's Encrypt's ISRG Root X1 and its R
  intermediates are RSA keys. That covers every outbound TLS connection
  and the chain check of the owner's own certificate (SEC-NET-003). The
  provider also signs TLS handshakes with an RSA key when the owner
  supplies an RSA certificate. These operations are reachable only
  through rustls's handshake and certificate verification, which the
  configurations built in `gunmetal-secrets/src/crypto/` drive. No
  Gunmetal function calls an RSA operation, and no `rsa` crate is linked.
- RS256 outside TLS waits for a recorded review, as D-08 asks. That covers
  ID tokens (R1.2) and passkeys, and the provider's RSA verifier is not
  used for them before the review. The review covers the `rsa` crate,
  including whether its past timing advisory affects verification
  (unverified). It also covers calling the provider's own RSA
  verification function from the crypto module instead, which links no
  new implementation. The owner chooses between them. RS256 for passkeys
  comes only if that review passes and the household authenticators are
  found to need it (register D-08, plan decisions 9 and 10). Until then
  single sign-on works only with providers set to ES256 or EdDSA.

Never allowed: MD5 or SHA-1 for any security purpose (a value a format
carries, such as FLAC's MD5 signature, is opaque bytes in a non-security
type, and R1 links no MD5 or SHA-1 implementation); unauthenticated
encryption (ECB, CBC or CTR without a MAC, MAC-then-encrypt); RSA
encryption with PKCS#1 v1.5; AES-GCM with random nonces in Gunmetal's own
server code; OpenSSL or any C cryptography other than the one TLS
provider; JWT libraries; non-cryptographic or seeded generators outside
tests (SEC-STD-019, SEC-STD-022). The `ctr` ban in `deny.toml` keeps no
wrapper, because the chosen AEAD is not built on it. R2's WebSocket
channel will need SHA-1 for its handshake, which arrives with a record
amendment that adds the reviewed WebSocket crate as the only wrapper of
`sha1`.

### 3. The TLS provider

The provider is **aws-lc-rs**, if it builds cleanly; otherwise **ring**.
"Builds cleanly" means all of these, in CI, for both R1 server targets
(Linux x86-64 and Linux AArch64, as static binaries and in the container
image): it compiles; `cargo deny check` passes, including licences, where
`deny.toml` excludes the original OpenSSL licence (whether either
provider's licence expression still contains it is unverified) and the
build-script allow-list of SEC-SUP-026; and `cargo vet` passes. WP-047's
dependency request runs the check and records which branch applied. The
provider not chosen is banned in `deny.toml`, so only one is ever linked.
If neither passes, WP-047 stops and the owner decides; no package picks a
third provider or a licence exception on its own.

With aws-lc-rs, listeners offer the hybrid X25519MLKEM768 group and prefer
it (rustls's `prefer-post-quantum`) from R1. With ring, the hybrid group
waits for SEC-NET-065 in R2. Either way the listener allows only TLS 1.3,
or TLS 1.2 with ECDHE and an AEAD suite (SEC-NET-002); installs no
session-ticket encrypter, so no ticket key exists to inventory; and uses
no "dangerous" verifier configuration (SEC-NET-009).

Both providers bring C and assembly into the build. That is accepted as
the cost of a mature TLS stack; the hardening checks of SEC-STD-033 apply
to the result. A pure-Rust provider is revisited once one is proven.

`webpki-roots` is distributed under its own data licence (stated as
CDLA-Permissive-2.0, unverified), which is not on `deny.toml`'s licence
allow-list. The dependency request that adds it either adds that licence
after review or names another bundled root source.

### 4. Discipline

- **AEAD only.** The one AEAD in Gunmetal's own server code is
  XChaCha20-Poly1305 with 192-bit nonces drawn from the one CSPRNG
  function by one nonce function (SEC-STD-020). No key in R1 uses counter
  nonces. age chooses its own nonces (a fresh file key per backup) and TLS
  its own. The single AES-256-GCM use in the web client (decision 1) has
  one encryption per derived key.
- **One opaque error.** Every decryption, MAC check and signature check
  returns one error value that does not say which step failed, releases no
  plaintext before the tag verifies (age verifies each chunk of a backup
  before releasing it), and logs no key, ciphertext or failing input
  (SEC-STD-021).
- **Constant time.** Tags and secrets are compared only through the
  implementations' verification functions or the `Secret` type's
  constant-time comparison, never with `==` (SEC-OPS-013).
- **Domain separation and agility.** Every MAC, hash-chain and signature
  input starts with a fixed label of the form `gunmetal/v1/<purpose>`, and
  HKDF's info is `gunmetal/v1/<purpose>/<key id>`. Every token, ciphertext
  and signature Gunmetal constructs carries a version, so an algorithm can
  change without breaking stored data. For values the secrets crate stores
  or sends on their own (session tokens, capability tags, vault
  ciphertexts, audit checkpoints), that version is a single leading byte,
  as the baseline's agility rule asks
  ([operations, section 2](../security/operations-and-incident-response.md#2-secrets-and-keys)).
  Two formats version another way, recorded here so they do not silently
  disagree with [record 10](0010-backup-archives.md):
  - age v1 payloads (backups) and TLS records carry the version their
    specifications define;
  - the backup archive of record 10 puts a 2-byte format version in the
    signed header and does not also prefix the 64-byte Ed25519 signature,
    because the version is already in the signed bytes.
- **Hashes of stored secrets.** Random secrets the server stores only to
  check later (session tokens, invitations, pairing codes, recovery links,
  recovery codes) are kept as HMAC-SHA-256 under a key of their own, never
  in clear and never as a bare hash. Argon2id is only for secrets a person
  chooses.
- **Key sizes.** Every symmetric key has at least 256 bits from the OS
  CSPRNG (SEC-OPS-011).
- **Fingerprints.** A public key's fingerprint is SHA-256 over the label
  `gunmetal/v1/fingerprint` followed by the key. It is shown as 64
  hexadecimal digits in groups of four, and as a QR code where the medium
  allows. A typed fingerprint is compared in full, never by prefix.

### 5. The cryptographic inventory

This is the first inventory SEC-STD-018 and SEC-NET-057 ask for. WP-047's
xtask reads the first column of the first table, and every key wrapper
type in the code must name one of those rows; WP-136 publishes the tables
as CycloneDX cryptographic assets in each release's SBOM. Unless a row
says otherwise, its operations live in `gunmetal-secrets/src/crypto/`.

**Keys and secrets the server holds.** Lifetimes and sizes that the
security parameters table owns are cited by its key.

| Name | Algorithm | Purpose | Made | Lives in | Rotation | In backups | From | Quantum |
|---|---|---|---|---|---|---|---|---|
| `root` | 256-bit secret, HKDF-SHA-256 input | Source of every derived key (SEC-OPS-015) | First start, OS CSPRNG | `secrets/root.key`, 0600, or a systemd credential | The owner's rotate-every-key action (SEC-OPS-018), every restore (SEC-OPS-044), or after compromise | Sealed export (record 10) | R1 | Symmetric; no change needed |
| `url_signing` | HMAC-SHA-256, tag of at least 128 bits | Capability URLs for audio, artwork and lyrics (SEC-API-026) | HKDF from `root`, one key per key ID | Derived on demand; key IDs in `secrets/keys.json` | A new key ID daily; the previous one accepted only for capability.stream's longest lifetime; any key ID revocable at once (SEC-API-030) | No | R1 | Symmetric |
| `session_hash` | HMAC-SHA-256 | Stored form of browser, admin and paired-browser session tokens (SEC-IAM-037, SEC-OPS-016) | HKDF from `root` | Derived | With `root`; rotating it ends every session, as intended | No | R1 | Symmetric |
| `secret_hash` | HMAC-SHA-256, one key per purpose: invitation, pairing code, recovery link | Stored form of single-use secrets (SEC-IAM-078, SEC-IAM-056, SEC-IAM-091) | HKDF from `root` | Derived | With `root`; outstanding invitations, pairing codes and recovery links end | No | R1 | Symmetric |
| `recovery_pepper` | HMAC-SHA-256 | Peppered hashes of recovery codes (SEC-IAM-089) | HKDF from `root` | Derived; after a rotation the retired pepper is kept sealed by `vault`, for verification only, until every code hashed under it is used or regenerated | With `root` | Sealed export | R1 | Symmetric |
| `audit_address` | HMAC-SHA-256 | Commitment to an audit record's source address and salt (WP-069) | HKDF from `root` | Derived; a retired key is kept sealed until the addresses committed under it are removed at 90 days | With `root` | Sealed export | R1 | Symmetric |
| `vault` | XChaCha20-Poly1305, 192-bit random nonces, the record ID as associated data | Replayed third-party secrets (SEC-OPS-017): the DNS update credential (record 8), OIDC client secrets (R1.2), integration secrets (R2) | HKDF from `root` | Derived; ciphertexts in the identity store | With `root`; every sealed value is re-encrypted (SEC-OPS-018) | Ciphertexts, with the identity store | R1 | Symmetric |
| `audit_signing` | Ed25519 | Audit checkpoints and the signed records of pruning and address coarsening (SEC-OPS-023) | Seed by HKDF from `root` | Derived; the public key of every epoch in `secrets/keys.json` | With `root`; earlier public keys kept to verify earlier checkpoints | Public keys | R1 | Classical signature (decision 6) |
| `backup_signing` | Ed25519 | Backup headers (record 10, SEC-OPS-043) | Seed by HKDF from `root` | Derived; the public key of every epoch kept, each with its certificate from `identity` | With `root`. `identity` certifies each new epoch's public key when the epoch begins, and every backup carries that certificate (record 10, decision 6). Earlier public keys are kept to verify earlier backups | Public keys and certificates | R1 | Classical signature |
| `backup_recipient` | X25519, as an age recipient | The server's own recipient for every backup (SEC-OPS-042) | HKDF from `root` | Derived | With `root`; retained backups are re-encrypted to the new key | Through `root` | R1 | Classical key agreement (decision 6) |
| `recovery_recipient` | X25519, as an age recipient | The owner's recipient for every backup (SEC-OPS-042, SEC-PRV-040) | At setup, preferably in the owner's browser (operations OD-4) | Public key in `secrets/recovery.pub`; the private half in the recovery kit, on the server only wrapped by `root` if kept at all, and optionally wrapped in the browser under a passkey PRF key | When the owner makes a new kit | Public key | R1 | Classical key agreement |
| `identity` | Ed25519 | The server's identity key: shown in pairing QR codes (SEC-IAM-057), and the iroh endpoint key from R2. It is also the trust anchor for backups: it certifies each `backup_signing` epoch under the label `gunmetal/v1/backup-signing-key`, and the recovery kit prints its fingerprint (SEC-PRV-040; record 10, decision 6). The label keeps these signatures apart from the handshake signatures the key makes from R2 | First start, OS CSPRNG | `secrets/identity.key`, 0600 | Only after compromise, with a continuity statement signed by the old key (SEC-NET-062, R2). The rotate-every-key action leaves it alone, and a restore brings back the one in the backup. A new key needs a new kit page with its fingerprint | Sealed export, so clients reconnect without pairing (register D-61) | R1 | Classical signature |
| `tls` | Built-in ACME: ECDSA P-256. A supplied certificate: the owner's key, ECDSA P-256 or P-384, or RSA of at least 2048 bits; any other type is refused at load ([record 8](0008-https-and-naming.md), decision 5) | The HTTPS listener's certificate. The provider signs handshakes with it: ECDSA, or RSA-PSS (and RSA PKCS#1 v1.5 under TLS 1.2) | On the server for built-in ACME; the owner's file for a supplied certificate | `secrets/tls/`, 0600 (SEC-NET-006) | A new key at every renewal (built-in ACME); the owner's choice otherwise | No; re-issued | R1 | Classical signature; key exchange is the TLS group |
| `acme_account` | ECDSA P-256 (ES256 request signatures) | The ACME account for own-domain certificates (record 8) | On the server, OS CSPRNG | `secrets/tls/`, 0600 (SEC-NET-006) | ACME key rollover, only after compromise | No; a new account is registered | R1 | Classical signature |
| `claim_code` | 128-bit secret, compared in constant time | Claiming the server (claim_code, SEC-IAM-007) | OS CSPRNG while unclaimed | `secrets/claim-code`, 0600 | Single use; 24 hours | No | R1 | Not a key |
| `pin_pepper` | 256-bit secret, Argon2id's secret input | The pepper for profile PIN hashes (SEC-IAM-062) | HKDF from `root` | Derived | With `root` | Sealed export | R2 | Symmetric |
| `offline_grant` | Ed25519 | Offline playback grants (SEC-IAM-054) | Seed by HKDF from `root` | Derived | With `root` | Public keys | R2 | Classical signature |

Random values from the one CSPRNG function, with sizes and lifetimes in
the parameters table: session tokens (session.browser, session.admin),
invitation secrets (invitation), recovery codes (recovery.codes), pairing
codes (pairing.code), share-link secrets (share_link, R1.2) and public IDs
(identifier.bits). They are stored only as the keyed hashes above.

**Public keys and trust anchors the server verifies against.**

| Name | Algorithm | Source | Lives in | Rotation | From |
|---|---|---|---|---|---|
| Passkey public keys | ES256 (COSE -7), EdDSA (COSE -8) | The authenticator, at registration | The identity store | The person adds and removes them (SEC-IAM-023) | R1 |
| Paired-browser keys | ECDSA P-256 with SHA-256 | The browser's Web Crypto, at pairing | The identity store | Revoked or re-paired | R1 |
| WebPKI roots | RSA (2048 and 4096 bits, among them Let's Encrypt's ISRG Root X1) and ECDSA P-256 and P-384 (among them ISRG Root X2) | `webpki-roots`, compiled in | The binary | With dependency updates; new roots must arrive before CAs use them | R1 |
| Certificate chains and TLS handshakes: every outbound peer's, and the owner's own chain (SEC-NET-003) | The provider's verification set: RSA PKCS#1 v1.5 and RSA-PSS with SHA-256, SHA-384 or SHA-512 on 2048- to 8192-bit keys; ECDSA P-256 and P-384 with SHA-256, SHA-384 or SHA-512 (aws-lc-rs; ring verifies these curves with SHA-256 and SHA-384 only); ECDSA P-521 (aws-lc-rs only); Ed25519 | The peer during the handshake, or the owner's certificate file | Memory | The peer's or the CA's | R1 |
| Update feed root | Ed25519 | The project's key ceremony, compiled in | The binary | Signed TUF root rotation (SEC-OPS-019) | R1 |
| OIDC provider keys | ES256, EdDSA; RS256 only after the review in decision 2 | The provider's JWKS, through the egress client | Memory | The provider's | R1.2 |
| Device keys, DPoP proofs | ECDSA P-256 | The device's keystore | The identity store | Revoked or re-enrolled | R2 |

**Algorithms used only inside the TLS provider.** These are rustls
0.23.45's defaults for each candidate provider. WP-047's dependency
request confirms them for the version it links, and the secrets crate
builds its configurations from them without adding any. No other code
reaches them.

| Use | Algorithms | From |
|---|---|---|
| Key exchange | X25519MLKEM768 (aws-lc-rs only, preferred; decision 3), X25519, ECDHE over P-256 and P-384 | R1 |
| Record protection | AES-256-GCM, AES-128-GCM and ChaCha20-Poly1305, under TLS 1.3, or under TLS 1.2 with ECDHE and an ECDSA or RSA server key | R1 |
| Key schedule and handshake hashes | SHA-256 and SHA-384, in HKDF (TLS 1.3) and the PRF (TLS 1.2) | R1 |
| Signature verification | The chain and handshake row of the table above, RSA included | R1 |
| Signing | The `tls` key: ECDSA P-256 or P-384; RSA-PSS, or RSA PKCS#1 v1.5 under TLS 1.2 | R1 |

**Project keys, held off every server.**

| Name | Algorithm | Held by | Rotation | From |
|---|---|---|---|---|
| TUF root, targets and advisory roles | Ed25519 on offline hardware keys, threshold 2 once two keyholders exist (SEC-SUP-049, SEC-STD-036); ECDSA P-256 if the chosen hardware cannot hold Ed25519 (unverified) | Distinct named people | Root rotation through signed metadata, rehearsed yearly (SEC-OPS-071) | R1 |
| TUF snapshot and timestamp roles | Ed25519 | The publishing workflow | Online, short-lived metadata | R1 |
| Release signatures | Sigstore keyless: a short-lived certificate for an ephemeral ECDSA P-256 key, logged in Rekor, with SLSA provenance (SEC-SUP-041 to SEC-SUP-043) | Nobody holds a long-lived key | Per release | R1 |
| Name-service zone and update keys | Not in R1 (record 8) | | | Later |
| Relay and edge keys | Ed25519 (iroh) | The project's relays and edge | Their own record | R2 |

Non-security checks are not inventoried as cryptography: CRC-32C frames
the user log against torn writes (WP-035) and protects nothing against an
attacker.

**Relation to the baseline's key table.** The operations guidance's
inventory
([operations, section 2](../security/operations-and-incident-response.md#2-secrets-and-keys))
is the first table this record extends. This inventory is the one
SEC-STD-018 and SEC-NET-057 ask for. Where they differ, this table wins;
the differences were previously unspoken:

| Baseline row | This inventory | How it differs |
|---|---|---|
| Root secret | `root` | Rotation also on every restore (SEC-OPS-044), which the baseline's "After restore: derived keys move to a new epoch" implies but does not list under Rotation |
| Stream-URL and CSRF keys | `url_signing` | Capability-URL signing only. CSRF is not a signing key: cookie SameSite and the `__Host-` prefix are the CSRF control (SEC-API-032 to SEC-API-035). SEC-OPS-015's parenthetical "CSRF" is not a separate row |
| Session tokens | `session_hash`, and the random values listed below the server-held table | The tokens are CSPRNG values stored as keyed hashes, not a signing key. The inventoried secret is the HMAC key |
| Server identity key (the iroh endpoint key) | `identity` | Also the backup trust anchor (record 10) and the pairing QR-code key (SEC-IAM-057). Iroh use remains R2 |
| TLS private key | `tls`, `acme_account` | Splits the ACME account key out; names the allowed key types |
| Server backup key | `backup_recipient` | Same role |
| Audit and backup signing keys | `audit_signing`, `backup_signing` | Split into two keys with separate labels, as SEC-OPS-015 requires one key per purpose |
| Recovery key | `recovery_recipient` | Adds the optional passkey-PRF wrap (SEC-PRV-040) |
| Third-party secrets | `vault` | XChaCha20-Poly1305 only. The baseline offers AES-256-GCM as well; decision 4 forbids AES-GCM with random nonces in Gunmetal's own server code |
| Setup (claim) code | `claim_code` | Single-use, 24 hours, never rotated by failed attempts (SEC-IAM-007). The baseline table's "every restart" does not apply |
| Project feed trust root | Update feed root (public-keys table) | Same role; it is a trust anchor, not a key the server holds |

Rows this inventory adds, which the baseline table does not name:
`secret_hash`, `recovery_pepper`, `audit_address`, `pin_pepper` (R2),
`offline_grant` (R2).

### 6. The post-quantum plan

SEC-STD-018 asks for a migration plan for the classical algorithms
Gunmetal relies on.

| What | Today | Risk | Plan |
|---|---|---|---|
| TLS key exchange | X25519, plus X25519MLKEM768 if the provider is aws-lc-rs | Traffic recorded now and decrypted later | The hybrid group in R1 with aws-lc-rs, otherwise in R2 (SEC-NET-065) |
| Backups that leave the host | age with X25519 recipients | A copied backup decrypted later | Move both recipients to a hybrid ML-KEM recipient type once the age implementation on this list supports one and passes review (support unverified); until then the docs say that off-host backups are protected by classical key agreement |
| TLS server authentication: certificate chains, handshake signatures and the `tls` key | RSA, ECDSA and Ed25519, inside the TLS provider | Forgery needs a quantum computer at connection time; recorded traffic gives nothing | Follow the WebPKI: post-quantum certificates (ML-DSA) once CAs, browsers and the provider support them, through dependency updates and `webpki-roots` |
| Passkeys, paired-browser keys, device keys | ES256 and EdDSA | Forgery needs a quantum computer at sign-in time; recorded traffic gives nothing | Follow WebAuthn and the platforms when post-quantum COSE algorithms ship |
| The update feed and release signatures | Ed25519 (TUF) and Sigstore | Forged updates | TUF root rotation can change key types; follow Sigstore's own migration |
| Audit checkpoints, backup signatures, the identity key and its certificates of backup-signing keys | Ed25519 | Forged local evidence or server identity | Add ML-DSA, alone or in a hybrid, when a reviewed pure-Rust implementation exists; the version on the value (a leading byte, or record 10's 2-byte format version for backup signatures) allows the switch |
| HMAC, HKDF, XChaCha20-Poly1305, SHA-256 | 256-bit keys and outputs | Grover's algorithm halves the margin, which stays sufficient | None |

## Consequences

- WP-047 builds the secrets crate's crypto module and the inventory
  xtask; WP-122 builds the core's SHA-256 file; WP-001 writes the lints
  and bans that keep cryptography in the two files; WP-136 publishes the
  inventory in the SBOM.
- WP-073 and WP-048 (TLS), WP-081 (passkeys), WP-090 and WP-109 (backups),
  WP-074 (the update feed) and WP-101 (ACME) wait for the provider check
  in decision 3 and use only the functions the crypto module exposes.
- Any new algorithm, key or implementation, including the ACME crate the
  plan has not chosen (plan decision 21) if it brings cryptography of its
  own, needs a record that supersedes this list before it lands.
- Single sign-on (R1.2) works only with ES256 or EdDSA providers until the
  RSA review is recorded.
- WP-047 certifies each `backup_signing` epoch with `identity`, and setup
  (WP-080) prints the `identity` fingerprint in the recovery kit (record
  10, decision 6).

## Requirement check

Review record, dated 2026-10-03, accepted with this follow-up on
2026-10-03. It stands as the dated review record for SEC-STD-018,
SEC-STD-019 and SEC-NET-057. The CI parts of each requirement are proved
by the packages named, not by this record.

| Requirement | What it asks | Result |
|---|---|---|
| SEC-STD-018 | An inventory with algorithm, key, purpose, library, location, rotation and post-quantum status | Met by decision 5; the library is decision 2's implementation for each algorithm, and the location is the "Lives in" column and decision 1 |
| SEC-STD-018 | CycloneDX cryptographic assets in the SBOM | WP-136, from decision 5 |
| SEC-STD-018 | A migration plan for the classical signatures relied on (passkeys, device keys, TUF and Sigstore) | Met by decision 6 |
| SEC-STD-018 | Cryptographic crates used only from one `crypto` module, and CI failing when the inventory and the code disagree | Decision 1, read as one door in two files (plan decision 38); the lints are WP-001's and the inventory check WP-047's |
| SEC-STD-019 | Cryptography only from a reviewed allow-list chosen in an architecture record | Met by decision 2, with the provider in decision 3 |
| SEC-STD-019 | AEAD only; no ECB, unauthenticated CBC or CTR, MAC-then-encrypt or RSA PKCS#1 v1.5 encryption | Met by decisions 2 and 4 |
| SEC-STD-019 | MD5 and SHA-1 only behind a non-security type | Met by decision 2; the compile-fail test is WP-047's |
| SEC-STD-019 | The same allow-list for native client code | Decision 1, for the web client now and the native clients through SEC-STD-039's record |
| SEC-NET-057 | Every key, certificate and algorithm on the network paths (TLS keys, the ACME account key, the iroh identity key, name-service registration, edge and relay keys), with where it lives, its lifetime and its rotation | Met by decision 5: `tls`, with every key type a supplied certificate may use; `acme_account`; `identity`; the WebPKI roots and the chain and handshake row, RSA included; the table of algorithms used only inside the TLS provider; and the relay and edge row. Name-service registration does not exist in R1 (record 8). The CI check that every key wrapper type appears in the inventory is WP-047's |
| SEC-STD-020, SEC-STD-021, SEC-STD-022, SEC-STD-024 | Nonce strategy, opaque errors, one randomness function, the Argon2id floor | Fixed by decisions 2 and 4; proved by WP-047's tests |
