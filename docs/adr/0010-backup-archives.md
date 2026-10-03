# 10. Backup archives

Date: 2026-10-03
Status: proposed. Drafted by WP-125 for the owner's acceptance. Its input
is register decision D-08, which the owner delegated on 2026-10-02,
accepting its recommendation of "a small hand-written archive format"
inside the age envelope
([decisions](../decisions.md#owner-answers-2026-10-02)). That answer
settles the choice, not this format. The owner accepts or edits the
record when reviewing the wave-0 pull request into `main` (D-01), and
this line then says so, with the date.

## Context

The server must not extract an archive received from outside until an
architecture record allows it, and that record must require ignoring entry
paths, refusing symlink entries, and limiting entry count and unpacked size
(SEC-HIS-019). Restore (WP-109) has to read backups, which can arrive from
anywhere: an upload at setup, a second destination such as a NAS share, a
file the owner kept for years. Backups are hostile input like any other
(first principle 2).

The baseline already fixes the envelope. Every backup is encrypted in age
v1 to the server's backup key and to the owner's recovery key
(SEC-OPS-042, SEC-PRV-039) and signed with the server's backup-signing key;
restore verifies the signature and the encryption tags, shows which server
made the backup and when, and parses the archive with limits on total
size, entry count and expansion ratio, rejecting absolute paths, `..`,
links and duplicate entries (SEC-OPS-043). Any SQLite file in it is opened
only read-only in a jailed worker (SEC-STD-031). What remained open was
the container inside the envelope. Tar and zip bring names, links,
absolute paths and traversal with them
([operations guidance, section 7](../security/operations-and-incident-response.md#7-backups-and-restore)),
so the plan offered a hand-written format or the `tar` crate, and the
owner took the recommendation: hand-written.

## Decisions

### 1. What this record allows

The server may extract exactly one kind of archive: a Gunmetal backup in
the format below, and only through restore (WP-109). Every other archive
format stays forbidden. The `deny.toml` ban on archive-extraction crates
that WP-001 adds stays whole, and no archive crate is added. Any later need,
such as subtitle archives from providers (SEC-MED-055, R2) or comic books
(Later), needs a record of its own.

### 2. The file

A backup file is named by the server, `gunmetal-<server ID>-<UTC
time>.gmb`; restore ignores the name of an uploaded file. All integers are
big-endian.

| Offset | Size | Field |
|---|---|---|
| 0 | 8 | Magic: the ASCII bytes `GMBACKUP` |
| 8 | 2 | Format version: 1 |
| 10 | 2 | Header length H: exactly 336 in version 1 |
| 12 | H | The header, below |
| 12 + H | 64 | Ed25519 signature by `backup_signing` ([record 9](0009-cryptography.md)) over the label `gunmetal/v1/backup-header` followed by bytes 0 to 12 + H |
| 76 + H | the rest | The age v1 payload |

The header, version 1:

| Size | Field |
|---|---|
| 16 | The server's public ID |
| 8 | Creation time, Unix seconds, UTC |
| 32 | The `backup_signing` public key that made the signature |
| 32 | The server's `identity` public key ([record 9](0009-cryptography.md)) |
| 64 | The certificate of that `backup_signing` key: an Ed25519 signature by the `identity` key over the label `gunmetal/v1/backup-signing-key`, the server's public ID and the `backup_signing` public key (decision 6) |
| 8 | Sequence number of the latest signed audit checkpoint, which the writer asks the audit log to make just before the backup (SEC-OPS-024) |
| 32 | That checkpoint's head hash |
| 32 | The `audit_signing` public key that signed the checkpoint |
| 64 | The checkpoint's signature |
| 8 | Length of the age payload in bytes |
| 32 | SHA-256 of the age payload |
| 8 | Length of the decrypted payload in bytes |

The header is outside the encryption so that restore can say which server
made the backup and when before anything is decrypted (SEC-OPS-043). It
reveals the server's random ID, its public keys, the time, the sizes and
the audit head, and nothing about people or media.

### 3. The payload

The age payload has two recipients, `backup_recipient` and
`recovery_recipient` (SEC-OPS-042). Decrypted, it holds:

| Offset | Size | Field |
|---|---|---|
| 0 | 8 | Magic: the ASCII bytes `GMBKDATA` |
| 8 | 4 | Entry count N, from 1 to 65,536 |
| 12 | 64 × N | The entry table |
| 12 + 64N | the rest | The entry bodies, in table order, back to back |

Each entry in the table is 64 bytes:

| Size | Field |
|---|---|
| 2 | Kind, from the closed list below |
| 2 | Reserved; must be zero |
| 16 | Stream ID for a user-log segment; zero for every other kind |
| 4 | Number: a user-log segment's month, counted as year × 12 + month - 1, or an audit segment's sequence number; zero for every other kind |
| 8 | Body length in bytes |
| 32 | SHA-256 of the body |

The kinds in version 1:

| Code | Kind | How many | Restored as |
|---|---|---|---|
| 1 | Identity store (SQLite) | Exactly one | The identity store, after the jailed check (SEC-STD-031) |
| 2 | Configuration: the settings a backup carries, never the host-only ones (SEC-OPS-041) | Exactly one | Through the live validators; anything looser than the current settings is held for the owner (SEC-OPS-044) |
| 3 | Sealed server keys, the secrets crate's export (WP-090) | Exactly one | Opened only by the secrets crate; never written as a file |
| 4 | User-log segment | Any number | The segment for that stream and month |
| 5 | Audit-log segment | At least one | The audit segment with that sequence number |
| 6 | Audit address side store (SQLite) | Exactly one | The side store, after the jailed check |
| 7 | Derived-data store (SQLite) | None or one, only if the owner chose to include it (ADM-141) | The derived-data store, after the jailed check |

Where each kind lives on disk is decided by [record 3](0003-durable-user-state.md)
and the data-directory layout, not by the archive. There is no
compression in version 1, so the size of what is unpacked is exactly the
decrypted payload length that the signed header states.

### 4. The rules SEC-HIS-019 requires, and the rest

- **Entry paths are ignored, because entries have none.** No field is a
  name or a path. Restore builds each destination from the entry's kind,
  stream ID and number, as typed values, through the data-root handle's
  closed path constructors (WP-126), inside a fresh staging directory
  under `tmp/` (SEC-HIS-015, SEC-MED-033).
- **Symlink entries are refused, because no link kind exists.** An entry
  of any kind not in the list, a non-zero reserved field, a stream ID or
  number on a kind that has none, a missing stream ID on a user-log
  segment, a number out of range, or a kind present more or fewer times
  than the list allows refuses the whole archive. So does a duplicate
  entry: two entries with the same kind, stream ID and number.
- **Entry count and unpacked size are limited.** The count is at most
  65,536. The decrypted payload length in the signed header must equal
  12 + 64N plus the sum of the body lengths exactly, and the payload must
  decrypt to exactly that many bytes. Without compression the expansion
  ratio is at most one, so no entry can unpack to more than it occupies.
  Before anything is decrypted, restore checks that the stated length plus
  the free-space reserve fits on the disk (WP-097), and refuses an archive
  larger than the restore size limit in the limits register (SEC-STD-030).
- **Verify before parse.** Restore reads the fixed 12 bytes and the
  header. It decides whether to trust the header's `backup_signing` key
  by the rules of decision 6, and checks the signature before it
  interprets any other header field. Restore then checks the payload's
  length and SHA-256 against the signed header before age reads a byte of
  it, decrypts with `backup_recipient` or the owner's recovery key (which
  stays in memory only for the restore and is then zeroised), and
  releases no plaintext from a chunk before age has verified that chunk
  (SEC-STD-021). Each body is hashed as it is written
  to staging and compared with its table entry.
- **Nothing changes until everything checks.** Any failure deletes the
  staging directory and leaves the live data untouched (SEC-TM-052). Only
  after every body matches and every SQLite file has passed the jailed
  read-only check, with `trusted_schema` off, triggers and views refused,
  `cell_size_check` on, memory mapping off and `quick_check` passed
  (SEC-STD-031), does restore move the staged stores into place. It then
  checks that the live audit log extends the backup's checkpoint
  (SEC-OPS-024), rotates every symmetric key, invalidates every session
  and signed URL, and sends the owner the "review devices and access"
  alert (SEC-OPS-044).
- **Versions only move forward.** A server refuses a format version newer
  than the ones it knows (SEC-OPS-051). A new kind, field or compression
  scheme is a new format version, introduced by a record that supersedes
  this one; compression would also have to go through the one streaming
  decompression helper (SEC-MED-009).

### 5. Where the parsing happens

The outer framing, the header and the entry table are fixed-width
structures, read by one pure, sans-I/O function in the core under the
parse contract, with a fuzz harness and seeds (SEC-MED-001, SEC-MED-027,
SEC-MED-028). The writer (WP-090) and restore (WP-109) both use it, and
WP-090's verify-after-write reads every backup back through it (ADM-072).
That parsing happens in the server process. Apart from the first 12
bytes and the three fixed-width key fields that decision 6 checks against
its anchor, every byte it interprets has already been authenticated by a
key the server holds or the owner confirmed. The complex formats inside a
backup, the SQLite files, are opened only in the jailed worker
(SEC-STD-031), and the configuration only through the live validators.

### 6. The trust anchor for restore

The `backup_signing` key rotates with `root`, and `root` rotates at every
restore and every rotate-every-key action ([record 9](0009-cryptography.md)).
A fingerprint of that key printed once would soon be stale. On a fresh
host the server's own keys are all new, and there the header is the only
place that names the old key. Trusting a key because the header names it
would defeat the signature. So the anchor is the server's `identity` key.
The rotate-every-key action leaves it alone, and a restore brings back the
one in the backup with the server's other keys (record 9), so the
fingerprint the kit prints stays valid.

- **Certification.** When a `backup_signing` epoch begins, the secrets
  crate signs the label `gunmetal/v1/backup-signing-key`, the server's
  public ID and the epoch's public key with `identity`. It keeps the
  signature beside the epoch's public key, and the writer copies both
  keys and the signature into every header (decision 2).
- **The kit.** The recovery kit prints the `identity` key's fingerprint
  (record 9, decision 4) beside the backup recovery key (SEC-PRV-040).
  It stays valid until `identity` itself is replaced after a compromise,
  which needs a new kit page.
- **The same server.** Restore trusts the header's `backup_signing` key
  without asking only when it is one of this server's own current or
  earlier `backup_signing` public keys.
- **A fresh host, or another server's backup.** Restore otherwise refuses
  the backup unless it obtains an expected fingerprint from outside the
  file. That is either the fingerprint the owner types or scans from the
  recovery kit, or, when the restore uses the passkey-wrapped recovery key
  instead of the printed kit, the fingerprint inside that wrap's
  authenticated plaintext (record 9, decision 1). If the expected
  fingerprint equals the fingerprint of the header's `backup_signing`
  key, as SEC-OPS-043 words it, that key is trusted. If it equals the
  fingerprint of the header's `identity` key, the `backup_signing` key is
  trusted only once its certificate verifies under that `identity` key.
  Anything else refuses the archive.
- **Never from the file alone.** Restore shows the owner the fingerprints
  of the header's keys, beside which server made the backup and when
  (SEC-OPS-043). Nothing in the file can stand in for the expected
  fingerprint.

## Consequences

- Restore (WP-109) may extract backups, under these rules and no others.
  Backups (WP-090) write this format.
- No archive library enters the build, and the archive-extraction ban in
  `deny.toml` stays whole.
- The plan names no owner for the core module that holds the format's
  reader and writer. It should be one package's file, listed in the
  harness registry's parser-module list (WP-008); WP-090, which writes the
  first backup, is the natural owner, with WP-109 depending on it.
- Setup (WP-080) prints the `identity` fingerprint in the recovery kit,
  and WP-047 certifies each `backup_signing` epoch (decision 6).
- No document yet says where the passkey-wrapped recovery key is kept so
  that a fresh host can use it (SEC-PRV-040). Whatever WP-080 and WP-090
  choose, the wrap carries the `identity` fingerprint inside it (record 9,
  decision 1), and if it travels in the backup file it does so as a new
  format version.
- The restore size limit needs an entry in the limits register
  (`crates/gunmetal-server/limits.toml`, which WP-130's check reads)
  before WP-109 ships.
- Large libraries make large backups, because version 1 does not compress.
  Compression can come later as a new format version, through the one
  decompression helper.

## Requirement check

Review record, dated 2026-10-03. This is the author's check, written by
the coding agent working on WP-125; no person has reviewed it yet. The
package's pull request merges into `wave-0` through the integrator agent
once the gate passes, with no human review (D-01). The owner's review of
the wave-0 pull request into `main` confirms or edits this check, and only
then does it stand as the dated review record for SEC-HIS-019.

| Requirement | What it asks | Result |
|---|---|---|
| SEC-HIS-019 | No archive is extracted until an architecture record allows it | Met: decision 1 allows exactly one format, through one function |
| SEC-HIS-019 | The record requires ignoring entry paths | Met: decision 4; entries carry no name or path |
| SEC-HIS-019 | The record requires refusing symlink entries | Met: decision 4; no link kind exists and unknown kinds refuse the archive |
| SEC-HIS-019 | The record requires limiting entry count and unpacked size | Met: decision 4; at most 65,536 entries, an exact unpacked size stated in the signed header, an expansion ratio of at most one, a free-space check and a size limit |
| SEC-HIS-019 | The `cargo-deny` ban on archive-extraction crates in the server and core is lifted only by a reviewed record | Met: decision 1 lifts nothing; no archive crate is added, and the ban WP-001 writes stays whole |
| SEC-OPS-043 | Signature and encryption tags verified, the source shown, limits on size, count and expansion, no absolute paths, `..`, links or duplicates, foreign keys only with a typed fingerprint | Consistent: decisions 2 to 4 and 6; proved by WP-109's tests. A key this server never used is accepted only against a fingerprint from outside the file. That is the key's own, as the requirement words it, or that of the `identity` key that certified it, which the recovery kit prints and which stays valid across rotations and restores |
| SEC-OPS-042, SEC-PRV-039 | age v1 to both keys, the secret-store key only wrapped | Consistent: decision 3, kind 3 |
| SEC-STD-031 | Restored SQLite files opened read-only in a jailed worker | Consistent: decisions 4 and 5 |
