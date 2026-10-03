# 3. Durable user state

Date: 2026-10-03
Status: accepted. The owner delegated this decision and accepted its
recommendation ([register decision D-05](../decisions.md#owner-answers-2026-10-02),
2026-10-02). Extends decision 5 of [record 1](0001-architecture.md), which
stays as written.

## Context

Record 1, decision 5, makes SQLite a rebuildable cache and names watch
history as the only irreplaceable data, kept in an append-only, exportable
log. Music in the first release (record 2) and the security baseline both
need more than that:

- Almost every write in the first release is something a rescan cannot
  recreate: the queue, playlists, loves, ratings, dismissals, Home layouts,
  saved rules, settings, the household's corrections to artists and
  albums, and the date an album first arrived. If any of it lives only in
  the cache, "rebuild instead of repair" (ADM-077) and "upgrade from any
  version" (ADM-058) are false ([api-needs](../plan/api-needs.md#ui-requirements-the-architecture-makes-hard-or-impossible),
  item 1; [feature map](../features/README.md#open-decisions-for-the-project-owner),
  decision 1).
- Accounts, credentials' public keys, devices, roles, grants,
  invitations and shares must live in durable storage that cache rebuilds
  never touch and that every backup includes, and a rebuild must never let
  a restricted profile see what its policy hides (SEC-IAM-004,
  SEC-TM-051; threat TM-T60 at boundary TB10).
- A person can erase one play, a range or all of their history. The data
  must leave every store, the log included, within the deadline
  SEC-PRV-049 sets, and stay gone when an older backup is restored
  (SEC-PRV-049, SEC-PRV-050, SEC-PRV-052). A log that is append-only
  without exception cannot do that, and a masking "removal event" does not
  meet the baseline (D-05).
- Backups leave the machine, so what they hold must be the minimum and
  must be encrypted (threat TM-T59; SEC-TM-052, SEC-IAM-105).

This record takes these inputs as settled:

- **Public IDs** are 128 random bits from the CSPRNG, minted once and kept
  in the identity store's public-ID mapping, never derived from content,
  paths or names (D-04; SEC-HIS-012, SEC-API-023).
- **Ratings** (D-05): loves on everything; five stars with half steps,
  switched on when ratings are imported; tracks, albums and artists rated
  directly, never derived from one another; strictly per person.
- **History events** carry only the fields SEC-PRV-002 allows, so a play
  records no source context; "Continue listening" uses the context IDs the
  queue already stores (D-05; MUS-050, LAT-009).
- **The crate layout** (D-02): the durable stores live in
  `gunmetal-durable`; every data-directory file goes through the
  data-root handle and every SQLite file through the one connection opener
  in `gunmetal-fs` (WP-126); protocol and record bodies are encoded with
  `serde` and `postcard` in the core.
- **R1 servers are Linux only**, and untrusted files are parsed in a
  jailed worker (D-09). This record relies on Linux rename and `fsync`
  semantics, and on the worker for restore.
- **The identity model** (no passwords or TOTP, passkeys, OIDC, browser
  pairing, sessions and their lifetimes) is record 7's; the archive inside
  an encrypted backup is record 10's (D-06, D-08).

## Decisions

### 1. What is durable, and where it lives

The rule: **anything a rescan of the media files cannot recreate is
durable state, and durable state never lives only in the cache.**
Everything else is derived, and may be thrown away and recomputed.

| Store | Path in the data directory | Holds | Format | Changed by | In backups |
|---|---|---|---|---|---|
| **Identity store** | `durable/identity.db` | Who exists and what each may do; server settings; the public-ID mapping (section 2) | SQLite, WAL, `synchronous=FULL` | Transactions; schema parts before R1, numbered migrations after | Yes, without volatile rows |
| **User log** | `durable/log/<stream>/<yyyy-mm>.seg` | What people and the household authored (sections 3 to 5) | Append-only segments of framed, checksummed records | Appends; erasure is the only rewrite | Yes |
| **Erasure ledger** | `durable/erasure/` | Selectors of everything erased: IDs and clock values, never content (section 8) | Framed records, as the log | Appends, by the log's writer task | No; a restore never replaces it |
| **Uploads** | `durable/uploads/` | Images people uploaded, as the server's re-encoded copies named by content hash | Files | Create and delete | Yes |
| **Staged imports** | `durable/staged/` | History an administrator imported for another person, until that person accepts or declines (API-SET-11) | As the log | Appends; erased on decline, or on the retention schedule if never answered | No |
| **Audit log** | `durable/audit/` | Security events, in the format of [operations section 3](../security/operations-and-incident-response.md#3-the-audit-log) (WP-069) | JSON lines, hash-chained, signed checkpoints | Appends; retention prunes with signed checkpoints | Yes, with the latest checkpoint |
| **Derived-data store** | `derived/derived.db` | Analysis and provider results keyed by content identity, producer kind and producer version (ADM-141) | SQLite | New keys; never migrated | Only when the owner chooses |
| **Cache** | `cache/library.db` | Everything else: the catalogue, projections of the log, the change log, scan and health state | SQLite, WAL | Rebuilt; never migrated | Never |

Secrets and keys stay in `secrets/` (operations section 2, WP-047), and
host-only settings stay in the configuration file, which a backup never
holds (SEC-OPS-041). Neither is this record's subject.

**Which durable store.** A fact goes to the identity store when the
server uses it to decide who may do what, and to the user log when a
person or the household authored it. The privacy choices the server
enforces (whether admins see the titles a person plays, MUS-235; how long
history is kept, MUS-234; consent to outbound sharing, SEC-PRV-033) are
policy inputs, so they live in the identity store even though a person
sets them: the authorisation layer and the purge job read them without
the cache, migrations are bound not to change them (SEC-PRV-023), and
erasing history cannot erase them. Preferences only clients act on live
in the person's stream (section 5). Both stores are durable, backed up
and exported; LAT-007's "never only a SQLite row" is read as "never only
in the rebuildable cache".

### 2. The identity store

**What it holds in R1.**

- **The server**: the claim state, which must agree with the
  `secrets/claimed` marker; the relying-party ID fixed at setup
  (SEC-IAM-018); every setting changed through the server (name and
  sign-in message, network and sign-in settings, egress grants, the
  update-check answer and the highest feed versions seen, alert rules,
  stream limits, backup settings); and the sealed ciphertexts of replayed
  third-party secrets, which only the secrets crate's vault opens
  (SEC-OPS-017).
- **Libraries**: kind, roots, flags, root settings and artist-splitting
  rules. They live here because grants refer to them and a rebuild needs
  them before it can scan.
- **People**: accounts, profiles, credentials (public material only:
  passkey public keys with their flags and counters, OIDC links keyed by
  issuer and subject, paired browsers' public keys), devices,
  recovery-code hashes and recovery holds, pending account deletions, the
  enforced privacy choices of section 1, and alerts and notices waiting
  for delivery. Credentials, accounts and profiles are separate entities
  (SEC-IAM-001). There is no password, TOTP secret or plaintext token
  (SEC-IAM-025).
- **Access**: grants, invitations and share links, each holding a hash of
  its secret, never the secret. Share-link counters hold counts, never
  addresses (SEC-PRV-003).
- **The public-ID mapping**: for each library item, its content identity,
  its public ID and the time the server first saw it, which is the
  album's "date added" that a better copy never resets (DIS-038); for
  each stored object (account, profile, device, playlist, share), its
  public ID.
- **Volatile rows**: sessions (token hashes, revocation epochs, elevation
  and user-verification times, the private-session flag, and the source
  address of a live session, removed when it ends, SEC-PRV-003), and any
  short-lived ceremony state an owning package keeps here rather than in
  memory (WebAuthn challenges, pairing requests, single-use tickets,
  export download handles). Volatile rows survive a restart, are never
  written into a backup and are gone after a restore. Each schema part
  says whether its rows are volatile.

**Rules.**

- It is opened only through the one connection opener, with
  `synchronous=FULL` and `secure_delete=ON`, and runs SQL only as static
  queries with bound parameters (WP-126; SEC-API-066, SEC-TM-039,
  SEC-PRV-050).
- Every column declares one data class, checked against the live schema
  at open (SEC-TM-050, SEC-PRV-001; WP-122).
- Every reader that returns a user-visible object takes a `Permit` from
  the policy; the pre-principal lookups are the written exceptions
  (SEC-TM-024, SEC-API-010; WP-046, WP-065).
- A single-use secret is consumed by one conditional update inside one
  transaction (SEC-STD-029).
- **Before R1 ships**, the schema is built from named parts and compared
  by digest, as the cache is; a development build that finds a different
  digest refuses to start and says to delete the development data
  directory. **At the R1 release** the parts freeze into migration
  `0001`, and every later change is a numbered migration whose number the
  integrator assigns at merge.
- **Migrations** follow [operations section 8](../security/operations-and-incident-response.md#8-upgrades-migrations-and-rollback):
  a snapshot with the SQLite online backup API, `integrity_check` on it,
  the migration in one transaction, the invariants (exactly one owner; the
  claim marker agrees; every security setting is valid), commit, then the
  audit event (SEC-OPS-048). No migration makes an installation less strict
  or changes a person's privacy choice (SEC-OPS-049, SEC-PRV-023). An older
  binary refuses a newer file and names the snapshot to restore; it never
  drops identity, audit or security settings in order to start
  (SEC-OPS-051).

### 3. The user log: streams and segments

- **One stream per profile**, in `durable/log/p-<profile ID>/`, holds that
  profile's Activity data.
- **One household stream**, in `durable/log/household/`, holds the
  household's curation: artist merges, splits and aliases (MUS-007), album
  merges and splits (LIB-058), release-type and explicit overrides
  (MUS-010, MUS-047), answers to the review queue (LIB-099) and playlists
  imported for the whole household (API-SET-11). Only an administrator
  acting through the server writes to it, online and in server order. It
  never holds one person's Activity data. Its entries are keyed by content
  identity, so they survive rescans, moves and rebuilds (LIB-179,
  LIB-029). Curation an administrator made stays after their account is
  deleted, because it belongs to the household.
- Directory and file names are built from internal random IDs and
  constants through the data-root handle, never from names, titles or
  paths (SEC-HIS-015, SEC-HIS-016, SEC-MED-033).
- A stream exists only for a profile the identity store holds, and is
  created by that profile's first record. Erasing a profile removes its
  whole directory (section 8).
- **Segments are monthly** by the server's UTC clock at the time of
  appending: `<yyyy-mm>.seg`. A play made offline last month and uploaded
  today goes into this month's segment; its own clock says when it
  happened. Each segment starts with a header naming the stream, the month
  and the segment format version.
- The data directory is on a local Linux filesystem; the server refuses a
  network filesystem (ADM-079), so rename and `fsync` mean what this
  record assumes.

### 4. Records and the event envelope

**Framing.** WP-035 fixes the bytes; this record fixes what is there.

- Each record is a length, a CRC-32C checksum over the rest, the record
  format version and the payload. The reader checks the declared length
  against a fixed cap before reading the payload, and the writer refuses a
  record above it. WP-035 sets the cap as a constant documented next to
  the framing code, as each format package does for its own parser
  constants.
- Storage is boundary TB10, and what comes back from it is untrusted: a
  damaged disk or a restored backup can hold anything. Segment decoding is
  a core parser under the parsing contract and SEC-MED-001, with a fuzz
  harness like every other parser.
- **A torn tail** (a record cut off at the end of the newest segment) was
  never acknowledged (section 6), so startup cuts the segment back to the
  last whole record and reports what it cut. That removes no acknowledged
  data and is not a rewrite.
- **Damage elsewhere** is never repaired in place. Replay skips the damaged
  range and resynchronises at the next good record, and the startup page
  and `gunmetal doctor` report the stream, the byte range and the sequence
  numbers lost, so the owner can fill the gap from a backup (ADM-078).

**The envelope.** WP-034 fixes the types; the payload is the envelope
encoded with `postcard`.

| Field | What it is |
|---|---|
| Sequence number | Stamped by the writer, per stream, strictly increasing and never reused. Erasure leaves gaps. Projections and damage reports refer to records by it. |
| Event ID | 128 random bits. An event a client authored carries the ID the client generated, so a retry is idempotent; a record the server authors gets its ID from the minting function (WP-047). |
| Clock | A hybrid logical clock: wall time in milliseconds and a logical counter. |
| Device ID | The device that authored the event (or the import batch, section 5). |
| Profile ID | The stream's profile, or the household. |
| Type, version and skippable flag | The record type, the schema version of that type, and whether an older binary may skip it (section 9). |
| Body | The typed body as length-delimited bytes, so a reader that does not know the type can keep it whole. |

Every field of every body type declares one data class (SEC-TM-050,
SEC-PRV-001).

### 5. What a stream holds

Three kinds of record, as [api-needs](../plan/api-needs.md#how-changes-flow-from-the-device-to-the-server)
sets out, each with its merge rule from [the conflict table](../plan/api-needs.md#how-conflicts-resolve):

1. **Events.**
   - In a profile's stream: play and skip, carrying exactly the fields
     of SEC-PRV-002; love and unlove, on any item including playlists;
     rating, in half steps from half a star to five, or cleared, on a
     track, album or artist (the event exists from the first log version,
     so the point release that ships ratings needs no format change);
     dismiss and its undo (hide and snooze join as new types in R2,
     API-LOG-05).
   - A typed position (LAT-006): a place in one item, held as a time
     offset in milliseconds, a text locator in the Readium style (the
     resource and the progression within it, never the text around it),
     a page of a total, or a percentage. Beyond the profile, the item and
     the place, it carries only the device and the time. It is Activity
     data and merges by the latest clock per item and version, the
     conflict table's rule for resume points. R1 defines the type in the
     first log version and writes none: music resumes from the queue
     document's position (below). The first media that resume outside a
     queue (video resume points in R2, then books) write it with no
     format change.
   - Imported listens (ADM-042) are play events whose device ID is the
     import batch's ID, and the batch's own record (service, date, counts)
     sits in the same stream, appended before the batch's first play or
     line. The person sees where they came from, scrobblers never send
     them back, and removing the import is an erasure of that batch
     (section 8). Unmatched lines of an import are a separate record type,
     not history events, because they keep the titles and artists the
     matcher needs until they match or the batch is removed. Each line's
     envelope clock is the listen time it carries, so every history
     selector and the retention purge remove it as they remove a play
     (section 8).
   - In the household stream: the curation entries of section 3, each
     naming the administrator who made it, which only admin views show
     (LIB-178's rule).
2. **Operations on versioned documents**: the queue (one per profile, with
   its lanes, contexts, active device and position; MUS-122, LAT-009),
   manual playlists, Home layout and pins, and rule trees. Each operation
   names the document and the version it was built on; the server orders
   operations, assigns versions and writes them. **Every 100 operations a
   document gets a full snapshot record**, so replay starts at the latest
   snapshot. Position updates are not individual records: the playing
   device reports at state changes and periodically, and the server writes
   at most one position record per document per batch window (API-QUE-04,
   ADM-080). Playlist entries have their own IDs and refer to tracks by
   content identity; an entry whose track is gone keeps its last known
   title (API-PL-08).
3. **Settings records**: one record per key, with a person or a device
   scope, replaced whole (CLI-030, ACC-012). Only preferences clients act
   on; enforced privacy choices are in the identity store (section 1).

**Never in the log**: anything from a private session, because the server
refuses an event tagged with one (SEC-PRV-024); addresses, user agents,
locations or free text in a history event (SEC-PRV-002); search terms
(SEC-PRV-004); secrets of any kind.

### 6. Writing

- **One writer.** One writer task per data directory owns every segment
  file, and nothing else opens one for writing (WP-068).
- **The server writes its own bytes.** It never stores bytes a client
  sent. It decodes a request into typed values and validates them as
  untrusted input (SEC-HIS-033), takes the profile and device from the
  credential (SEC-API-013), resolves every public ID through the mapping to
  a content identity, refuses the whole write if any item is not visible
  to the profile (SEC-API-012), and then encodes the record itself.
- **The erasure ledger first.** Before appending, the writer checks every
  incoming event against the erasure ledger (section 8), which it holds
  in memory: every selector of the event's stream, by event ID, by clock
  range or up-to clock, and by import batch. An event a selector covers
  is acknowledged as a success and not stored, so the device dequeues it
  and an erased record never comes back, whether it is a retry whose
  first acknowledgement was lost or a play queued offline inside an
  erased range (CLI-093). This is how "the erasure
  removes the play whatever order events arrive in" holds
  ([the conflict table](../plan/api-needs.md#how-conflicts-resolve)). A
  range or up-to selector covers an imported listen only when the
  listen's batch record was appended before the selector, because an
  import accepted after an erasure is history the person chose to add;
  the same comparison applies when the ledger is applied again after a
  restore, so the same data always gives the same state. The ledger is
  appended by the same writer task, so no event slips between a
  selector's append and the check. A device whose clock runs behind can
  lose a play it made just after an erasure; the rule errs on the side of
  the erasure. WP-068 makes this check, and its tests include a retry of
  an erased event and a play from an erased range arriving after the
  erasure, each acknowledged and leaving no record in the segments or the
  cache.
- **Idempotency.** For an event no selector covers, the key is (stream,
  event ID). The same ID with the same body succeeds without a second
  record; the same ID with a different body is refused as a conflict, so a
  retry can never overwrite.
- **Clocks.** The server refuses a client event whose wall time is ahead
  of its own clock by more than a fixed skew bound, so a device cannot win
  every "latest wins" by lying about the time, and never advances its own
  clock past that bound from a client's value. WP-034 sets the bound as a
  core constant, and the server package that enforces it registers it in
  the limits register (SEC-STD-030).
- **Durability.** A write is acknowledged only once it is on disk. The
  writer appends a batch, calls `fdatasync` on the segment, then
  acknowledges every write in the batch (group commit). A new segment is
  created exclusively with its header, and the file and its directory are
  synced before its first acknowledgement. The identity store commits with
  `synchronous=FULL`. A write that fails is refused with a typed error;
  plays queued on a device stay queued (CLI-093).
- **One store per action.** A user action writes one durable store. Where
  a flow touches both (account deletion, restore), the identity store is
  the authority and the log follows idempotently from it: account deletion
  marks the account in the identity store, and the erasure job removes the
  stream, recording its ledger entry first so a crash resumes (section 8).

### 7. Reading and replay

- **Readers take a `Permit`.** Every reader that returns a person's
  records takes a `Permit` for that stream (SEC-TM-024, SEC-API-010;
  WP-068). The one exception is the `Permit`-free replay
  (`replay_into`), which yields records only into the projection builders
  modules register and returns nothing a handler can serve. Only the
  startup module calls it, for the rebuild and the projections' catch-up
  (WP-095), and it is on WP-065's written list.
- **One set of rules, not one reader.** The cache's projections, the
  rebuild, export and import share WP-034's merge functions and the
  projection code, so they agree on what a stream means. They do not
  share a reader: export reads a stream only through the `Permit`-taking
  reader, with a `Permit` for that stream, and import writes only through
  the writer (section 6). A document starts from its latest snapshot;
  events merge by WP-034's rules, which are commutative, associative and
  idempotent (plays are a union by event ID; loves, ratings, dismissals,
  positions and settings take the latest clock, with ties broken by
  device ID and then event ID).
- **Projections catch up.** The cache records, per stream, the last
  sequence number each projection applied, in the same transaction as the
  projection's change. At startup every projection catches up from there,
  so a crash between an acknowledged append and its projection loses
  nothing. Projections reach devices through the cache's change log
  (WP-066), like any other change.
- **Unknown records are kept.** Replay passes them through, backups copy
  them, exports include them as opaque records with their type name, and
  no writer ever drops one.

### 8. Erasure, the one sanctioned rewrite

The log is append-only except for erasure, which is the only operation
that removes or rewrites acknowledged records.

**Selectors.** An erasure names what it removes without holding any of
it. The selector goes into the ledger and stays on the server. Devices
receive only the IDs of the events it removed (step 2), so a tombstone
identifies erased events only by ID, as SEC-PRV-052 requires.

| Selector | Removes |
|---|---|
| (stream, event ID) | One history record |
| (stream, from clock, to clock) | A person's history in a time range |
| (stream, up to clock) | All of a person's history so far |
| (stream, import batch ID) | One import: its plays, its unmatched lines and its batch record |
| (stream) | A whole profile, after an account's deletion grace period (SEC-PRV-051) |

**History** is what the event, range and up-to selectors remove, matched
by each record's envelope clock: play and skip events; typed positions
(LAT-006), because a place in an item says what the person read, watched
or heard, and when; and unmatched import lines, whose clock is the listen
time they carry (section 5). Everything else (loves, ratings, dismissals,
settings, and documents, the queue and its position included) is not
history; it leaves with the whole profile. When an erasure leaves an import batch with no plays and no
unmatched lines, the job also erases its batch record with the batch's
selector. A person's history-retention choice (MUS-234, DIS-187) runs
through the same pipeline: the daily purge job erases history older
than the chosen period with a range selector (WP-138, WP-133), so it
removes unmatched lines and positions as well as plays.

There is no removal event: D-05 rules out a masking one, so the "remove
play" body in WP-034's list is not a log type. The property its merge
test proved, that a removal wins whatever order events arrive in, is now
kept by the writer's ledger check (section 6). The selector type and its
match against an envelope are pure core code beside WP-034's merge rules,
so a core property test can prove that any interleaving of appends and
erasures ends in the state of the filtered log.

**Steps** ([privacy section 10](../security/privacy-and-data-protection.md#10-deletion-and-erasure);
WP-068, WP-133):

1. In the writer task, append the selector to the erasure ledger and sync
   it. From here the erasure is promised, and the writer drops every
   incoming event the selector covers (section 6).
2. In one cache transaction, drop the erased records from the
   projections, so they leave every view before the job finishes, and
   append to the profile's sync feed tombstones listing the erased event
   IDs, in chunks that fit the sync frame cap. Every device purges them on
   its next sync (API-SYNC-10, SEC-PRV-052). Because both happen in one
   transaction, a crash cannot lose the IDs before step 3 removes them
   from the log. A whole profile gets no tombstone: its devices were
   revoked when the account was disabled, and a revoked device deletes
   its copy (SEC-CLI-009).
3. In the writer task, rewrite each affected segment without the erased
   records: write a new file beside it, sync it, rename it over the old
   one and sync the directory (the data-root handle's atomic replace). For
   a whole profile, remove the stream's directory. Surviving records keep
   their sequence numbers.
4. Delete derived rows in the cache and the derived-data store under
   `secure_delete`, then checkpoint and truncate the WAL (SEC-PRV-050), and
   recompute derived values from the filtered log; an incremental erasure
   and a rebuild from the filtered log give the same state (SEC-PRV-049).
5. For a whole profile, delete its staged imports in `durable/staged/`,
   and each upload it referenced that no other row still references.
   Uploads are named by content hash, so two profiles can share one. In
   R1 the only references are profile pictures in the identity store
   (API-USR-01); a release that adds others (playlist images, R2) counts
   them too.
6. For a whole profile, delete the account's identity-store rows under
   `secure_delete`, then run `PRAGMA wal_checkpoint(TRUNCATE)` on the
   identity store, so no earlier WAL frame keeps the profile's name, its
   devices' names or its recovery holds (SEC-PRV-050). The audit log keeps
   only its own tombstone for the account, under its own rules ([operations section 3](../security/operations-and-incident-response.md#3-the-audit-log)).

Every step is idempotent, and the job resumes from the ledger after a
crash and finishes within the deadline of SEC-PRV-049.

**What the erasure tests show** (WP-068 for the log, WP-133 for the
job): a retry of an erased event and an offline play inside an erased
range are both acknowledged and leave no record; a range, an up-to
erasure and the retention purge each remove unmatched import lines and
positions as well as plays; an import accepted after an erasure keeps its
plays; a tombstone, compared with a literal encoding, holds only event
IDs; and after a whole profile's erasure a raw-byte scan of the data
directory, `identity.db` and its `-wal` file included, finds neither the
profile's name nor a device's name, its picture and staged imports are
gone, and a picture another profile shares stays.

**The ledger** holds only stream, event and batch IDs and clock values,
each entry's own clock included, never content. The writer reads it on
every append (section 6). It is not in backups and a restore never
replaces it; after any restore every entry is applied again before the
server serves
(SEC-PRV-049). It is kept for the life of the data directory, so a
restore of any older backup, including a copy kept off the host, is
covered. Backups taken before an erasure still hold the data until they
expire (SEC-PRV-041), and the deletion screen says so with the date
(SEC-PRV-051).

### 9. Versions and compatibility

- **The log changes only additively.** A new record type, or a new
  optional field in a newer version of a type, is allowed. Changing what
  an existing type means is not; that is a new type.
- A type is marked **skippable** when an older binary can ignore it
  without misreading anything else, and release notes say which releases
  added only skippable types (ADM-059, ADM-060).
- **An older binary** that meets an unknown type keeps and ignores it when
  it is skippable, and otherwise refuses to start and names the
  pre-upgrade snapshot to restore; it does the same for a newer segment or
  record format version (SEC-OPS-051).
- The identity store migrates (section 2); the cache never does (section
  10). So any newer binary reads any older log (ADM-058), and an older
  binary starts on newer data only when everything it does not understand
  is skippable.

### 10. Rebuilding the cache

The cache is rebuilt when its schema digest differs from the binary's
(after an upgrade, or when an older binary meets a newer cache), when the
owner asks (API-LIB-06, `gunmetal rebuild`), and after a restore. In order
(WP-095):

1. The rebuild never touches the identity store, the user log, the
   ledger, uploads, staged imports or the audit log (SEC-IAM-004).
2. The new cache gets a new generation ID, so a device holding a cursor
   from the old one is told to take a fresh snapshot (WP-066).
3. The household stream is replayed first, so every curation override,
   manual explicit marks included, is in place before any item appears.
4. Each profile stream is replayed into the projections.
5. A full scan runs, and items appear as its batches commit, with the
   overrides applied. The derived-data store is reused, so nothing is
   decoded or fetched twice (ADM-141).

Grants, roles and restrictions come from the identity store throughout,
never from the cache, and an item reaches a profile only once it is
committed with every attribute that profile's policy reads. A rebuild can
therefore show less while it runs, never more (SEC-TM-051).

### 11. One backup path

Both stores go into one archive (WP-090):

- **A consistent point.** The writer pauses between batches, records each
  segment's length, and the identity store's online backup starts from a
  read transaction taken in the same pause (SEC-OPS-041); then writing
  resumes. Segments are copied up to the recorded lengths while appends
  continue. An erasure rewrite during the copy replaces the file by
  rename, and the copy keeps reading the file it opened, which is the
  state at the consistent point.
- **Contents**: the identity store without volatile rows, every stream,
  the audit log with its latest signed checkpoint (SEC-OPS-024), uploads,
  the configuration without host-only settings, and the server keys only
  in the wrapped form the secrets crate exports; the derived-data store
  only when the owner chooses. Never the cache, the ledger, staged
  imports, snapshots or scratch files.
- **Protection**: encrypted in age v1 to the server's backup key and the
  owner's recovery key, and signed (SEC-OPS-042, SEC-OPS-043, SEC-PRV-039,
  SEC-IAM-105, SEC-TM-052), with the container inside the encryption that
  record 10 defines.

Pre-migration snapshots (section 2) are separate: local copies with the
same protection as the live data ([operations section 7](../security/operations-and-incident-response.md#7-backups-and-restore)).

### 12. Restore

Restore (WP-109) treats a backup as untrusted input. It verifies the
signature and the format versions before changing anything (SEC-TM-052),
opens the archive and every SQLite file in it read-only in the jailed
worker (SEC-STD-031; record 10 for SEC-HIS-019), and lets data reach the
server only as typed rows and records. It replaces the identity store,
the log and the uploads as a whole, each upload decoded and re-encoded in
the worker as a new upload is (API-USR-01). It then applies the ledger
again, rotates every key and holds restored devices and credentials for
review (SEC-OPS-044; no session survives, because none was in the
backup), and rebuilds the cache (section 10).

### 13. One export path

Export reads a stream only through the `Permit`-taking reader, with a
`Permit` for that stream, never through the rebuild's `Permit`-free
replay. It builds its documents with the same merge functions and
projection code as the rebuild (section 7; SEC-TM-024, SEC-API-010;
[privacy section 9](../security/privacy-and-data-protection.md#9-export)):

- **A person's export** (API-USR-05; SEC-PRV-047, SEC-PRV-048) is their
  own stream, read with a `Permit` for it, in the documented formats,
  plus their own identity-store rows and their own audit records, and
  nothing about anyone else.
- **The owner's server export** (ADM-074) holds settings without secrets,
  library roots, the household stream and household data, and the owner's
  own data, each stream read with the owner's `Permit` for it; never
  another adult's stream (SEC-PRV-025).
- Importing a native export into a fresh server appends the same events
  with the same event IDs through the writer (section 6), so a repeated
  import changes nothing and the round trip of SEC-PRV-047 holds.

### 14. Content identity and public IDs

- The log refers to library items only by their content identity
  (LIB-028; SEC-PRV-002), never by path, title or public ID, so history,
  playlists and curation follow a recording through moves, renames and
  rebuilds (LIB-029).
- Clients never see content identities, only public IDs, and the public-ID
  mapping in the identity store connects the two. Each ID is minted once,
  by the secrets crate's minting function (WP-047), and kept, so a rebuild
  reissues none (D-04; INT-008).
- If a later release changes the content-identity rules, it migrates the
  mapping under the identity store's migration rules (old identity to new,
  same public ID). The log is never rewritten for it.

## Not decided here

- The byte layouts of the segment header, the record framing and the
  envelope (WP-035, WP-034), and the values of the record size cap
  (WP-035) and the clock skew bound (WP-034).
- The identity model and session lifetimes (record 7), cryptography
  (record 9), and the archive inside a backup (record 10).
- Retention periods (SEC-PRV-005; WP-138).
- Sync payload budgets for history, lyrics and seek indexes (work-packages
  owner decision 15).
- Compaction of superseded document operations. R1 has none. If the
  benchmark (WP-115) shows the log growing too fast, compaction would be a
  second sanctioned rewrite and needs its own record.

## Review record

Date: 2026-10-03. Reviewed by the author of WP-002, before a maintainer
reviews the pull request.

**What was checked.** That every R1 row in [api-needs](../plan/api-needs.md)
marked "Feed", or that writes user data, has a home in this record, and
that none is homed only in the cache. The table lists every R1 capability
row once, the ones that store nothing included, so the check is complete.

**Requirements reviewed, design part only:** SEC-IAM-004 and SEC-TM-051.
The integration tests that prove them belong to WP-046 and WP-095. This
record names them without a `Verifies:` line, so that the traceability
check cannot count a design review as their proof.

**Revised the same day** after the package review. The writer now checks
every append against the erasure ledger, so an erased play cannot return
by a retry or a late offline upload (section 6). Unmatched import lines
and typed positions (LAT-006) are history for erasure and retention
(sections 5 and 8). A whole profile's erasure now truncates the identity
store's WAL and removes its uploads and staged imports, and a restore
replaces uploads (sections 8 and 12). Tombstones list event IDs only, as
SEC-PRV-052 is written; clock ranges stay in the server's ledger
(section 8). Export reads through the `Permit`-taking reader, never the
rebuild's replay (sections 7 and 13).

| Capabilities | Where their data lives |
|---|---|
| API-SYS-01 to API-SYS-09 | Nothing stored. The startup page, the emergency page and the public facts read the stores' state. |
| API-SYS-10 | Nothing durable; single-use tickets are volatile rows or memory (section 2). |
| API-AUTH-01 | The claim code is in `secrets/` (WP-047, WP-080), not in a store here. |
| API-AUTH-02, API-AUTH-03 | Identity store: the claim state, the owner's account, profile and credential, recovery-code hashes, and the setup answers as server settings. The recovery key's public half is in `secrets/`. |
| API-AUTH-04, API-AUTH-06, API-AUTH-13 | Identity store: credentials (passkey counters and last use, OIDC links, paired browsers' keys) and devices; the OIDC client secret as a sealed vault row; pairing requests volatile. |
| API-AUTH-07 | Not user state: the limiter's counters are WP-064's; failures go to the audit log. |
| API-AUTH-08 to API-AUTH-10 | Identity store, volatile rows: sessions, epochs, elevation. |
| API-AUTH-11, API-AUTH-12, API-AUTH-15 | Identity store: recovery links as hashes, recovery holds, recovery-code hashes; events in the audit log. |
| API-USR-01 | Identity store (the profile's name; the picture's reference); `durable/uploads/` (the re-encoded picture, removed with the profile once no other row references it, section 8). |
| API-USR-02 | The profile's stream (preferences with person or device scope); identity store (the privacy choices the server enforces). |
| API-USR-03 | Identity store. |
| API-USR-04 | Audit log. |
| API-USR-05 | Reads both stores through the one export path (section 13); the archive is a single-use scratch file. |
| API-USR-06 | The profile's stream: imported plays under their batch ID, the batch record and unmatched lines (section 5). Unmatched lines are history for erasure: each carries its listen time as its clock, so a range or up-to erasure and the retention purge remove them as they remove plays (section 8). |
| API-USR-07 | Identity store, volatile: the private flag on the session. Nothing reaches the log. |
| API-USR-09 | Nothing stored. |
| API-USR-10 | Identity store (pending deletion), then the erasure of a whole profile (section 8). |
| API-DEV-01, API-DEV-05 | Identity store: devices, sessions, notices waiting for delivery. |
| API-DEV-02 | Identity store (the device's last sync time). The cursor stays on the device and is valid for one cache generation. |
| API-DEV-03 | Not durable user state: the uploaded report falls under the diagnostic class of the retention schedule. |
| API-SYNC-01 to API-SYNC-03 | Built from the cache, the identity store's grants and the projections of the profile's stream; the change log is in the cache. |
| API-SYNC-04 | The profile's stream, through its projections. |
| API-SYNC-05 | Derived: the generated sizes in the bounded derivative cache, with their metadata in the derived-data store (ADM-141). Nothing anyone authored. |
| API-SYNC-06, API-SYNC-07 | Derived; cache. |
| API-SYNC-10 | Erasure ledger (durable, on the server only); tombstones listing the erased event IDs in the cache's change log. A device that resnapshots after a rebuild receives a copy without the erased events. |
| API-SYNC-11 | Nothing stored. |
| API-LIB-01, API-LIB-03 to API-LIB-05, API-LIB-07 | Identity store: libraries, roots, root settings, grants, artist-splitting rules. A changed location keeps history because the log is keyed by content identity. |
| API-LIB-02 | Nothing stored. |
| API-LIB-06 | The rebuild (section 10). |
| API-CAT-01 to API-CAT-10 | Cache, derived from the files at scan; analysis and provider results in the derived-data store. |
| API-CAT-11 | Nothing stored. |
| API-CAT-12 | Household stream. |
| API-CAT-13 | Cache. |
| API-STR-01 to API-STR-05 | Nothing stored here; signing keys in `secrets/`, sessions volatile. |
| API-SES-01, API-SES-02 | Live playback sessions are in memory and end with the process; each admin read is recorded in the audit log (SEC-IAM-077). |
| API-SES-03 | The profile's stream: the active device in the queue document. |
| API-SES-04 | The profile's stream: play and skip events. |
| API-SES-08 | Identity store: the limits, as server settings. Leases are in memory. |
| API-QUE-01 to API-QUE-04 | The profile's stream: the queue document's operations, snapshots and coalesced positions. |
| API-QUE-05 | The profile's stream: a new playlist document. |
| API-PL-01, API-PL-02, API-PL-05, API-PL-08 | The profile's stream: playlist and rule-tree documents, pins and loves; a missing entry keeps its last known title. |
| API-PL-03 | The profile's stream, or the household stream for a household playlist; the export is a download. |
| API-PL-04 | Cache: read from the files, read-only. Their public IDs are in the mapping, and pins and loves on them are in the profile's stream. |
| API-LOG-01, API-LOG-02, API-LOG-05 | The profile's stream. |
| API-LOG-03 | The erasure of section 8: ledger, segment rewrite, tombstone. |
| API-LOG-04 | Cache: projections of the profile's stream. |
| API-HOME-01, API-HOME-02 | The profile's stream: the Home layout document and pins. |
| API-HOME-03 | Identity store: the first-seen time in the public-ID mapping. |
| API-HOME-04 | Derived, on the device or in the cache. |
| API-HOME-05 | Nothing on the server (SEC-PRV-004). |
| API-SHR-01, API-SHR-03 | Identity store: share links with secret hashes, limits and counters. |
| API-SHR-02 | Volatile: the share session. |
| API-SCAN-01 to API-SCAN-04 | Cache: scan and task state; nothing anyone authored. |
| API-SCAN-05 | Cache (the activity log, which is derived) and the audit log. |
| API-HLTH-01, API-HLTH-02, API-HLTH-05 | Cache: derived from the scan. |
| API-HLTH-03 | Cache (pending decisions, which a rescan recreates); household stream (the answers). |
| API-HLTH-04 | Cache. Identities, history and playlists stay keyed by content identity, so a file restored from the trash returns with everything (LIB-033). |
| API-ADM-01 to API-ADM-03 | Identity store: accounts, roles, invitations as hashes, and the account and credential that redemption creates. |
| API-SET-01 to API-SET-03, API-SET-06, API-SET-07, API-SET-09 | Identity store: settings and alert rules, with replayed secrets as sealed vault rows. Host-only settings are in the configuration file. |
| API-SET-04, API-SET-05 | The backup and restore paths (sections 11 and 12); archives in `backups/`. |
| API-SET-08 | Nothing durable; the bundle is a scratch file. |
| API-SET-10 | Nothing stored. |
| API-SET-11 | The importing administrator's stream, or the household stream for household playlists; another person's history waits in `durable/staged/` until that person accepts. |
| API-SET-13 | `secrets/`; the vault rows in the identity store are sealed again. |
| API-TOK-03 | Nothing stored. |

## Consequences

- WP-034, WP-035, WP-046 and WP-068, and through them most of R1, can
  start. The point releases R1.1 to R1.3 (D-10) add record types and
  identity tables additively; ratings and history imports need no format
  change when they ship.
- "Rebuild the cache" is honest: it loses nothing, because nothing durable
  lives only there (ADM-077). Every module that projects user data
  registers a rebuilder and tests its own replay (WP-085, WP-086, WP-087,
  WP-092, WP-093, WP-107).
- Backups stay small: the stores this record defines, without the cache
  and, by default, without derived data.
- Upgrades and rollbacks follow one rule set: the cache is discarded, the
  log is read additively, and the identity store migrates under a
  snapshot.
- Two durable stores are more work than one, and the erasure job must run
  inside the log's writer to rewrite a segment safely.
- One writer with group commit bounds write throughput; positions and
  plays are therefore batched, and the scan benchmark (WP-115) measures
  the log's size and write rate.
- The log grows without compaction in R1. Superseded queue and playlist
  operations stay until their profile is erased.
- The erasure ledger keeps IDs and clock values, never content, for the
  life of the data directory. The writer holds it in memory and checks
  every append against it, and it grows by one entry per erasure,
  including each daily retention purge.
- Erasing a large range sends devices one event ID per erased record, in
  chunks, rather than one short range; that is the price of tombstones
  that hold only IDs.
