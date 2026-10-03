# User flows

Written on 2026-10-02. Status: draft for the project owner's review.

This file walks through the journeys that matter most in Gunmetal, one step
at a time, so that the screens the UI needs and the server work behind each
screen can be read off in the order a person meets them. It is built on the
[feature map](../features/README.md) and refers to features by ID. The
feature map is the source of truth: where this file and the map disagree
about what a feature does or when it ships, the map wins and this file is a
bug. Both stay inside the accepted architecture records,
[ADR 1](../adr/0001-architecture.md) and
[ADR 2](../adr/0002-music-is-first-class.md).

Nothing here is built yet. Speed figures are the map's proposed goals (open
decision 16), not measurements. Rival behaviour comes from the research in
`docs/research/` and the map's "Rivals today" cells, and carries
"(unverified)" wherever the research could not confirm it.

## How to read a flow

Every flow has the same parts.

- **Release.** The release in which the main path first works end to end on
  at least one client. When an earlier release offers a reduced version, or
  a later one changes the path, the release line says so and the flow has a
  section for it.
- **Who and where.** The person, the device and the client.
- **Starts and ends.** The state before the first step and after the last.
- **Steps.** A table whose columns, after the step number, are:
  - *The person* says what they do and what they see.
  - *Surface* names the screen or control, using the names in the map's UI
    surfaces column so the UI plan can collect them.
  - *The server* says what the server must do at that step. Some work runs
    in the shared Rust core on the device (WASM in browsers, UniFFI in
    native apps). The cell then says "on the device", because "no server
    call" is a requirement too.
  - *Features* lists the owning rows. A reference row appears only when it
    adds detail for that step.
- **When it goes wrong.** The branches a designer must draw and a tester
  must cover.
- **Rivals today.** Where Plex, Jellyfin and others stand, including where
  they are already good.
- **Other releases.** What arrives earlier or later, by ID.
- **Depends on.** Architecture records and the numbered open decisions in
  the [feature map README](../features/README.md#open-decisions-for-the-project-owner).

Two facts from the map shape every R1 flow.

1. **R1 clients are browsers only.** They are the web client and the
   installable web app (CLI-001, CLI-003), with a phone-width layout
   (CLI-149). Native Android, Android TV, Fire OS and desktop apps arrive in
   R2.
2. **Browsers keep their best features for secure contexts.** Passkeys,
   installation, offline loading and dependable local storage need HTTPS or
   localhost (CLI-150). Over plain HTTP on a LAN address, R1 is an
   online-only web player that signs in with a password. The flows mark the
   steps where this matters.

The flows lean on three terms the map defines. **The user log** holds
everything a person authors that a rescan cannot recreate: history, loves,
ratings, playlists, queue state, layouts and household curation. **The
identity store** holds accounts, credential public keys, device keys,
grants and invitations. **The synced library** is each profile's copy of
library metadata on each device. Both durable stores wait on ADR 3 (open
decision 1), so nearly every flow below depends on it.

## Flow index

Flows F01 to F12 are the journeys the project asked for, in the order
asked. F13 to F20 cover journeys that the first twelve kept running into:
upgrades, restores, a lost phone, a person's own record, video on a TV,
children, third-party apps and live TV.

| # | Flow | Release | Who | First client |
|---|---|---|---|---|
| [F01](#f01-first-run-server-setup) | First-run server setup | R1 | Owner | Host console, then the web client |
| [F02](#f02-adding-a-music-library-and-watching-it-scan) | Adding a music library and watching it scan | R1 | Owner or admin | Web client (admin) |
| [F03](#f03-signing-in-on-a-new-device) | Signing in on a new device | R1 for browsers; native apps in R2 | Any member | Web client |
| [F04](#f04-pairing-a-tv-by-scanning-a-code-with-a-phone) | Pairing a TV by scanning a code with a phone | R2 | Member with a signed-in phone | Android TV, Google TV or Fire OS app |
| [F05](#f05-playing-an-album-and-editing-the-queue) | Playing an album and editing the queue | R1 | Listener | Web client |
| [F06](#f06-building-a-playlist) | Building a playlist | R1 | Listener | Web client |
| [F07](#f07-downloading-for-offline-and-playing-on-a-plane) | Downloading for offline and playing on a plane | R2 | Listener or viewer | Android app |
| [F08](#f08-moving-playback-from-the-phone-to-another-device) | Moving playback from the phone to another device | R2; R1 offers "continue on this device" | Listener | Android app, desktop, TV, web |
| [F09](#f09-searching-across-music-and-video) | Searching across music and video | R1 for music; video in R2 | Anyone | Web client |
| [F10](#f10-sharing-a-library-with-a-friend) | Sharing a library with a friend | R1 through the owner's own address; no open ports in R2 | Owner and friend | Web client |
| [F11](#f11-migrating-from-plex-or-jellyfin) | Migrating from Plex or Jellyfin | R1 for export files and playlists; rival databases in R2 | Owner | Web client (admin) |
| [F12](#f12-recovering-from-a-failed-playback) | Recovering from a failed playback | R1 for music; video in R2 | Listener, then owner | Web client |
| [F13](#f13-upgrading-the-server-and-rolling-back) | Upgrading the server and rolling back | R1 | Owner | Host, startup page, admin |
| [F14](#f14-rebuilding-a-dead-server-from-a-backup) | Rebuilding a dead server from a backup | R1 | Owner | Host, welcome screen |
| [F15](#f15-losing-a-phone) | Losing a phone | R1 for sessions; downloads in R2 | Member, owner | Web client |
| [F16](#f16-keeping-your-own-record) | Keeping your own record | R1 | Listener | Web client |
| [F17](#f17-watching-a-film-on-the-tv) | Watching a film on the TV | R2 | Viewer | Android TV app |
| [F18](#f18-setting-up-a-childs-profile) | Setting up a child's profile | R2 | Parent | Web client, then TV |
| [F19](#f19-connecting-an-existing-music-app) | Connecting an existing music app | R2 | Listener, owner | A third-party Subsonic app |
| [F20](#f20-adding-an-iptv-playlist-and-watching-a-channel) | Adding an IPTV playlist and watching a channel | R3 | Owner, viewer | Web client (admin), TV app |

Journeys the project has placed in **Later** or **No** are listed in
[Journeys not designed here](#journeys-not-designed-here), with the reason.

---

## F01. First-run server setup

**Release: R1.**

**Who and where.** The owner, first at the host (a terminal, or `docker
logs` for a container), then in a browser.

**Starts** with a binary or container that has never run. **Ends** with an
owner account, setup closed for good, daily backups on and the first music
library scanning.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Installs the single binary, the container image or the OS service, with media mounted read-only. | Install docs; download page | Nothing yet. There is one self-contained binary per target, and container tags pin a version. | ADM-001, ADM-002, ADM-003, ADM-004, ADM-005, ADM-089 |
| 2 | Starts the server. | Console; Startup page | Checks the config against a typed schema and reports mistakes with their line. Refuses to run as root unless overridden, and refuses a data directory on a network filesystem. Keeps settings, log, cache and scratch space apart. Opens the HTTP listener before the database, so the startup page can say what is happening. | ADM-007, ADM-006, ADM-079, ADM-090, ADM-032 |
| 3 | Reads the one-time setup code in the console. | Console and log message | Generates a single-use code with an expiry and prints it to the console and to a file only the service user can read. Refuses every route except the welcome and status pages until the server is claimed. | ACC-001, ADM-018 |
| 4 | Opens the server's address in a browser and picks a language, region and time zone. | Welcome > Language | Serves the welcome page and stores the locale settings. | ADM-027 |
| 5 | Types the setup code. | Welcome > Setup code | Verifies the code under the shared guessing limiter. | ACC-001, ACC-063 |
| 6 | Chooses between setting up a new server and restoring from a backup. Restore continues in F14. | Welcome | Nothing beyond routing; both paths sit behind the setup code. | ADM-029 |
| 7 | Sees whether this page can use passkeys. Over HTTPS or on localhost it can. Over plain HTTP on a LAN address, a panel explains why not and offers what works: open the page on the server itself, or add a certificate for a domain the owner holds. | Welcome > "Why can't I add a passkey here?" panel; Welcome > HTTPS | Reports whether the request came over a secure origin. Given a certificate, starts the TLS listener and redirects to it. | ADM-021, CLI-150, ADM-022, ACC-098 |
| 8 | Creates the owner account: a name, then a passkey, or a password and a two-factor code where passkeys cannot work. Saves the recovery codes. Optionally links the household's own identity provider. | Welcome > Create admin | Creates the owner in the identity store. Registers the WebAuthn credential, or hashes the password under the strength policy and enrols the TOTP secret. Issues recovery codes. For OIDC, runs discovery with state, nonce and redirect allow-list checks. Starts a cookie session that page scripts cannot read. | ACC-002, ADM-019, ACC-050, ACC-052, ACC-053, ACC-057, ACC-124 |
| 9 | Names the server and, if wanted, writes a short message for the sign-in page. | Welcome (server name) | Stores two text settings that the sign-in page shows. | ADM-140 |
| 10 | Reviews privacy. Every feature that could reach the internet is listed, explained and off. Decides whether to allow the signed update check. | Welcome > Privacy | Starts the egress gate with no grants and registers each outbound feature, so the network activity page can account for every connection. Enables no metadata provider. | ADM-028, ACC-113, LIB-108, ADM-053, ADM-129 |
| 11 | Answers "Coming from another server?" by skipping, or by handing over Last.fm or ListenBrainz export files and M3U playlists (F11). | Welcome > Import | Should hold the import jobs until the first scan has built enough of the library to match against. | ADM-030, ADM-042, ADM-043 |
| 12 | Adds music folders, as F02 describes. | Welcome > Libraries | Browses only allowed base paths, checks access and storage type, and registers the roots in durable settings. | ADM-025, LIB-005, LIB-001 |
| 13 | Finishes and lands in the music library, which fills in while the scan runs. | Welcome > Done; Home; Admin > Dashboard | Removes the setup routes for good, starts the scan, switches on daily backups and shows the health summary. | ADM-020, ADM-031, LIB-021, ADM-065, ADM-109 |

**When it goes wrong.**

- The code has expired or was mistyped. The page says which, and the
  attempt counts against the limiter. The map does not say how a fresh code
  is issued ([G1](#gaps-and-questions-the-flows-expose)).
- The browser closes after the code is accepted but before the owner
  exists. The map does not say whether setup resumes (G1).
- There is no secure context and no domain. Setup never dead-ends: password
  plus two-factor works, and account settings ask for a passkey once HTTPS
  exists (ACC-002, ACC-055). Over plain HTTP that password crosses the home
  network unencrypted, and the panel should say so (G2).
- Someone else on the network reaches the welcome page first. Without the
  code they get nothing (ACC-001).
- The config has a mistake, the service runs as root, or the data directory
  is on NFS. The server refuses to start and says why, and `gunmetal doctor`
  lists fixes (ADM-007, ADM-006, ADM-079, ADM-123).
- The owner later loses every passkey and device. `gunmetal admin recover`
  on the host prints a single-use link (ACC-004, ADM-034).

**Rivals today.** Jellyfin's wizard is short, local and needs no outside
account, so Gunmetal is only at parity there. Plex's claim token for
headless installs is a clean flow, but it binds the server to a plex.tv
account. Immich's restore button on its welcome screen, which checks that
library folders are readable, is the model for step 6. Jellyfin 12.0 had to
close a hole that let its wizard be re-run, and Navidrome 0.64.2 fixed a
failed setup that left a server with no admin. Those are the reasons
ACC-001 and ADM-020 are rules rather than polish.

**Other releases.**

- R2: film and TV folders in the same step (ADM-026, LIB-002); setting up a
  headless server from the Android or TV app with a device key, which
  sidesteps the secure-context problem (ADM-035); declarative setup from a
  file (ADM-024); a next-steps checklist that ticks itself (ADM-033).
- Later: a per-server HTTPS name issued by gunmetal.tv (ADM-023, open
  decision 7); automatic certificates for the owner's domain (ACC-099).

**Depends on.** ADR 3 for the identity store (open decision 1); open
decision 7 (HTTPS names); open decision 15 (no telemetry).

---

## F02. Adding a music library and watching it scan

**Release: R1.**

**Who and where.** The owner or another admin (ACC-040) in the web client,
from Welcome > Libraries during F01 or from Admin > Libraries at any time.

**Starts** with folders of audio files the server can read. **Ends** with a
browsable, playable library on every signed-in device that may see it, a
health report, and background analysis running.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Chooses "Add library" and the Music kind. | Admin > Libraries; Welcome > Libraries | Creates a library record with its kind, roots and settings. R1 offers only music, but the record has room for other kinds, and a folder can be flagged as spoken word so audiobooks stay out of music. | LIB-001, LIB-004, LIB-011, LAT-010 |
| 2 | Picks one or more folders in the server's folder browser and sees, for each, whether it is readable, what kind of storage it is, and that nothing will be written to it. | Folder picker with live checks | Lists only allowed base paths, and only to admins. Checks read permission and storage type (local disk, network share, FUSE mount) and warns before an empty or unreadable folder. Stores each root once, so every item is a root plus a relative path. | ADM-025, LIB-003, LIB-007, ADM-089, LIB-015 |
| 3 | Optionally sets exclusions, how changes are detected and a safety-net schedule. | Library settings > Exclusions; "Watch for changes"; Library settings > Folder > Storage type; Library settings > Schedule | Stores a gitignore-style pattern list per root, a file watcher for local disks, a poll interval and read parallelism for shares and cloud drives, and a rescan schedule. | LIB-006, LIB-014, LIB-015, LIB-013 |
| 4 | Chooses who may see the library. | Admin > Users > Libraries, or a step in the add-library sheet | Writes library grants into each person's policy. The grants filter the synced library and every fetch. | ACC-037, MUS-027 |
| 5 | Saves, and the scan starts. | Admin > Dashboard scan card; Admin > Activity | Runs the scan on a bounded worker pool with a memory ceiling and no helper process per file. Reads headers and indexes only, within a per-file read budget. Visits recently modified folders first, commits in batches and publishes progress events. | LIB-012, LIB-020, LIB-019, LIB-021, LIB-022, ADM-093 |
| 6 | Watches the files found, the bytes read and an estimate of the time left. | Dashboard scan card; activity indicator in the admin header | Counts I/O per root, so the owner can see that the scan read headers, not whole files. | ADM-088, ADM-031 |
| 7 | Opens Home or the album grid while the scan runs, and albums appear batch by batch. | Home (scanning empty state); library views; progress banner | Sends each batch through the library change feed; on the device, the client applies the deltas to its synced copy. | LIB-018, CLI-022, MUS-208, DIS-004, MUS-043 |
| 8 | Plays an album that has already arrived, as F05 describes. | Album page; player | Serves bytes for any committed item under the usual signed URLs. | MUS-066, ACC-122 |
| 9 | Sees the library take shape from tags, not folders: credits, release groups, discs and artwork. | Artist page; album page | Builds albums, artists and credits from multi-value tags in every format, splits artist strings with an exception list, uses MusicBrainz IDs as identity, and records the reason for every grouping. Takes embedded and folder artwork, resized for each device class through a safe image path. Turns `.m3u` files in the folders into playlists. | LIB-045, MUS-034, MUS-035, MUS-036, LIB-098, LIB-134, LIB-135, LIB-142, LIB-143, LIB-192 |
| 10 | Reads the summary and the health report when the scan ends. | Admin > Activity; Library health > Problems; Library health > Tag problems; Library health > Sidecar problems; Admin > Review queue | Records parse errors with their location, tag problems with their reasons, and files the supported browsers cannot decode. Holds doubtful groupings, such as two same-titled albums, for a yes. | LIB-193, LIB-194, MUS-044, MUS-229, LIB-099, LIB-051, LIB-068 |
| 11 | Sees background analysis continue at low priority. | Admin > Tasks; Admin > Activity | Measures loudness for untagged tracks as a throttled, checkpointed job that survives restarts and yields to playback. Rebuilds the neighbour table that radio and "more like this" use. | MUS-086, LIB-024, ADM-095, DIS-060 |
| 12 | Later copies new albums into the folder, and they appear on their own. | Recently added row on Home | Picks up the change by watcher or poll and rereads only what changed. Treats a better copy of an existing file as an upgrade, not a new arrival. | LIB-014, LIB-015, LIB-017, LIB-030, DIS-035, DIS-036, DIS-038, MUS-059 |

**When it goes wrong.**

- A folder is unreadable or empty. The picker warns before saving
  (ADM-025).
- A drive or NAS goes offline during the scan or later. Its items grey out
  instead of disappearing, the root shows as offline, an alert goes out, and
  the trash never purges while a root is offline (LIB-032, ADM-108, ADM-116,
  LIB-033).
- Files are moved or renamed. Identity follows them, keeping plays, ratings
  and playlist positions (LIB-028, LIB-029). A file that vanished is listed
  with its last known path (LIB-034).
- A tag would split "AC/DC" into two artists. The exception list prevents
  it, and the split report shows any doubtful case (MUS-035, LIB-038).
- An admin wonders why a track landed on an album. "Why is this here?" and
  the file inspector show raw tags beside the interpreted values (LIB-098,
  ADM-125, LIB-195).
- In-process decoders for loudness are not approved before R1 (open
  decision 8). R1 then uses loudness tags plus a fallback gain, and step 11
  measures nothing (MUS-089).
- Nobody has decided who sees a new library by default
  ([G4](#gaps-and-questions-the-flows-expose)).

**Rivals today.** Navidrome already builds albums from tags, lets people
browse during the first scan and publishes expected scan times by library
size; it is good here. Plex rescans part of a library when files change,
and Jellyfin and Emby monitor local disks in real time, though Linux watch
limits often need raising. Gunmetal's edge is in what the scan avoids: no helper process per
file (Jellyfin 10.9's ffprobe processes, about 700 MB each, exhausted a
4 GB Raspberry Pi 4, #11588), no whole-file reads on cloud drives, change
detection on shares that send no events, and a recorded reason for every
grouping. Scan speed is a design goal until the benchmark against Jellyfin
exists (ADR 1).

**Other releases.**

- R2: film and TV libraries (LIB-002); cancelling and reprioritising jobs
  (LIB-023); opt-in, sandboxed lookups at MusicBrainz and the Cover Art
  Archive (LIB-107, LIB-111, LIB-112); editing in the app, with locks
  (LIB-172, LIB-173); a duplicates report (LIB-196); CUE sheets (LIB-071,
  MUS-041); more formats (MUS-033); ratings read from tags (MUS-045);
  remapping a missing file by hand (LIB-035); an I/O profile per root
  (ADM-087).
- Later: acoustic fingerprinting in the review queue (LIB-106); missing
  albums on artist pages (LIB-201).

**Depends on.** Open decision 8 (decoders for loudness); ADR 3 for the
curation log that keeps review-queue answers (open decision 1).

---

## F03. Signing in on a new device

**Release: R1 for browsers; native apps in R2.**

**Who and where.** Any member with an account, in a browser on a new laptop
or phone.

**Starts** with an account made in F01 or F10. **Ends** with the device in
the person's device list, holding a synced copy of what they may see, and
showing their home and queue.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Opens the server's address. | Sign-in page | Serves the sign-in page with the server's name and message and no list of users. Shows a notice if the browser is not on the supported list. Reports whether this is a secure context. | ADM-140, ACC-007, CLI-002, CLI-150 |
| 2 | Signs in with a passkey; or with a password and then a two-factor code; or through the household's identity provider. | Sign-in; second sign-in step; Sign-in > Continue with provider | Runs the WebAuthn ceremony, or checks the password and the TOTP code, or completes OIDC with state, nonce and redirect checks. Uses uniform errors and timing and the shared limiter. Makes no outbound call except to the household's own identity provider. | ACC-050, ACC-052, ACC-053, ACC-057, ACC-007, ACC-063, ACC-003 |
| 3 | Chooses whether this browser should stay signed in. | "Remember this browser" | Issues a cookie session that page scripts cannot read, with the lifetime policy allows for a remembered or a shared computer. Records the device and writes an audit entry. | ACC-079, ACC-124, ACC-068, ADM-110 |
| 4 | Sees a one-time banner if this connection cannot keep an offline copy or use passkeys, with the fix. | Banner on first sign-in; Settings > About this connection | Nothing beyond the secure-context report in step 1. | CLI-150 |
| 5 | Waits while the device syncs, then sees Home. | Home; sync status indicator; Settings > Storage | Sends a snapshot and then deltas of the synced library, filtered by the person's grants and restrictions as the payload is built. Sends the profile's preferences and home layout from the user log. | CLI-022, ACC-037, ACC-030, ACC-012, CLI-030, DIS-002, DIS-007, CLI-024 |
| 6 | Is offered the queue they left on another device. | Player bar prompt | Nothing more: the queue document arrived with the sync. | CLI-103, MUS-122 |
| 7 | Optionally installs the web app on the home screen. | Install prompt | Serves the manifest, icons and service worker. This works only in a secure context. | CLI-003 |
| 8 | Later finds this device in their list, with its last-seen time. | Account > Devices | Updates last-seen in the device registry. | ACC-068 |

**When it goes wrong.**

- The password is wrong. There is one uniform message and attempts slow
  down (ACC-063). Owners who want fail2ban get suitable log lines (ADM-122).
- A member is locked out. An admin issues a single-use, short-lived link,
  with a QR code, that leads straight to enrolling a new passkey (ACC-064).
- The owner has lost every device and passkey. `gunmetal admin recover` on
  the host prints a recovery link, and a banner and audit entry follow
  (ACC-004, ADM-034).
- The internet is down but the home network is up. Sign-in and playback
  still work (ACC-003).
- The server becomes unreachable after sign-in. The device keeps browsing
  and playing what it holds (CLI-025). A device that never finished its
  first sync has nothing to fall back on and must say so
  ([G12](#gaps-and-questions-the-flows-expose)).
- The server was upgraded and this client is older. Protocol negotiation
  keeps it working, or it shows "update needed" (CLI-032).
- The page is plain HTTP. Sign-in is by password, and there is no offline
  copy (CLI-150).

**Rivals today.** Plex's sign-in is easy and has two-factor, and one
plex.tv account lists every server; that is convenient but central, and
plex.tv sign-in was down for about two hours on 2026-07-14. Jellyfin has no
built-in two-factor (#26, 1,103 votes), and its main single sign-on plugin
was archived on 2026-05-12. Jellyfin's Quick Connect is a good local way to
sign in a second device. No media server in the research offers passkeys.

**Other releases.**

- R2: native apps find the server on the home network (ACC-104), accept a
  custom address and proxy headers (CLI-028), and keep a device key in
  secure storage instead of a cookie (ACC-051). A new browser can be
  approved from a signed-in device instead of typing a password, using the
  same pairing protocol as F04 (ACC-062). The person's other devices get a
  "new device signed in" notice (ACC-071). One app can hold several servers
  with a key per server (CLI-018, ACC-014). Shared devices get a profile
  picker (ACC-019). Small devices sync partially (CLI-023). The app says
  whether it is local, direct or relayed (ACC-105) and switches between home
  and away by itself (CLI-029). Admins can require strong sign-in (ACC-054).
- Later: one sign-in across several Gunmetal servers (ACC-133); moving to a
  new phone in one step (ACC-015).

**Depends on.** ADR 3 (identity store and user log); open decision 7
(HTTPS names); open decision 16 (initial sync of 100,000 tracks in under
2 minutes, proposed).

---

## F04. Pairing a TV by scanning a code with a phone

**Release: R2.**

**Who and where.** A member with a signed-in phone, and an Android TV,
Google TV or Fire OS device with the Gunmetal app.

**Starts** with the TV app installed and signed out. **Ends** with the TV
holding its own device key, listed as the person's device or as a household
device, and opening the right profile.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Installs and opens the TV app. | TV app first launch | Answers LAN discovery, so the TV finds the server with no address typed. Away from home the person picks or types it. | CLI-006, CLI-008, CLI-017, ACC-104 |
| 2 | Sees a QR code, a short typed code as a fallback, and the TV's name. | TV sign-in screen | On the device, the TV creates a key pair in secure storage. The server opens a pairing request holding the TV's public key and issues a short-lived code under the guessing limiter. | ACC-061, CLI-027, ACC-051, ACC-063 |
| 3 | Scans the QR code with the phone. | Phone approval sheet | Resolves the request and shows which TV is asking and which profile it wants. The code identifies the request only, so scanning it grants nothing. | ACC-061, CLI-034 |
| 4 | Chooses "my TV" or "household TV", picks the profiles it may open, and approves. | Phone approval sheet | Checks the approver's session and rights. Enrols the TV's public key against the person or the household, with the allowed profiles. Writes an audit entry and sends a new-device notice to the person's other devices. | ACC-021, ACC-016, ACC-071, ADM-110 |
| 5 | Watches the TV move on by itself. | TV | Completes a challenge-response sign-in with the device key. No password or token is typed on, or shown by, the TV. | ACC-051 |
| 6 | On a household TV, picks a profile; adult profiles ask for a PIN with hidden digits. | TV profile picker; PIN pad | Lists the profiles this TV may open and checks the PIN hash under an attempt limiter. | ACC-019, CLI-050, ACC-020, DIS-142 |
| 7 | Waits while the TV syncs, then sees Home with the left rail. | TV Home; TV rail | Sends the profile's filtered synced library: all metadata, and artwork in TV sizes within the device's budget. | CLI-023, LIB-142, DIS-144, CLI-035, CLI-038 |
| 8 | Later finds or removes the TV. | Account > Devices; Admin > Household > Devices | On revocation, fails the TV's new requests at once and cuts its open responses. | ACC-068, ACC-069, ACC-122 |

**When it goes wrong.**

- The code expires before anyone scans it. The map sets an expiry but not
  what the TV does next; it should show a fresh code without a key press.
- The wrong person scans it. The sheet names the TV, so they decline, and
  the request lapses unapproved.
- Someone tries codes by brute force. The limiter stops them (ACC-063).
- The phone is away from home. Approval works wherever the phone can reach
  the server, which in R2 includes iroh (ACC-096).
- The TV cannot reach the server. Connection status says why (ACC-105),
  and the owner's connectivity check helps (ADM-134).
- The household has iPhones and no Android phone. There is no native iPhone
  app in R2 (CLI-005 is Later), so the approval sheet must also work in the
  web client ([G9](#gaps-and-questions-the-flows-expose)).

**Rivals today.** Jellyfin's Quick Connect already signs in a TV with a
six-digit code, locally and for free, once the admin enables it; a QR
version is planned (#2642, 225 votes). Plex and Emby use link codes or PINs
through their account services. Gunmetal's differences are that the phone
names the TV before approving, the TV gets its own revocable key rather
than a copied session, and a household TV can open several profiles with
nobody's password.

**Other releases.**

- R1 has no TV app. A TV's own browser can open the web client and sign in
  with a password (F03), which is awkward with a remote.
- R2 also gives command-line tools a token through the same pairing routes
  (INT-027).
- Later: Apple TV, if the App Store licence decision allows (CLI-007, open
  decision 3); Samsung and LG (CLI-010); Roku through the Jellyfin adapter
  (CLI-012).

**Depends on.** ADR 3 (identity store); open decision 10 (relays) for
pairing from outside the home.

---

## F05. Playing an album and editing the queue

**Release: R1.**

**Who and where.** A listener in the web client, in a desktop browser or
the installed web app on a phone.

**Starts** with the library synced to the device. **Ends** with the album
playing gaplessly at an even level, the queue holding the listener's edits,
and any of their signed-in devices able to pick it up.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Opens an album from Home, search or the artist page. | Album page | Nothing: the page renders on the device from the synced library, with discs and disc titles, other editions and credits. | MUS-054, MUS-012, MUS-008, MUS-001, MUS-208 |
| 2 | Presses Play, or Play disc. | Album page; now-playing bar | On the device, the client builds the queue with the album in the "From" lane and a context ID, and the core marks any track this browser cannot decode. The server applies the queue operation, assigns a version and stores the queue document in the user log. | MUS-116, LAT-009, MUS-229, MUS-122 |
| 3 | Hears the first track start, and sees a badge such as "Original FLAC, 24-bit, played directly". | Now-playing bar; quality badge | Signs a short-lived URL bound to the session, the item and an expiry. Serves byte ranges of the original file, or copies its audio frames into fragmented MP4 without re-encoding when the browser's Media Source Extensions need that. Records the decision and its reason. | ACC-122, MUS-066, MUS-230, MUS-099, ADM-100 |
| 4 | Hears the album play without gaps and at a steady level. | Player; Track info "gain source" | Supplies encoder delay and padding, a seek index and gain values from the scan. On the device, the player fetches the next track early and applies album gain while the album plays in order. | MUS-067, MUS-069, MUS-070, MUS-071, MUS-084, MUS-085, MUS-087, MUS-088, MUS-089, MUS-090 |
| 5 | Uses the lock screen, notification or media keys, and opens the full-screen player and the lyrics. | OS media panel; now-playing bar; full-screen player; lyrics view | Supplies embedded and sidecar lyrics inside the synced library. | MUS-073, CLI-070, MUS-108, MUS-110, MUS-154, MUS-155, MUS-156, MUS-158 |
| 6 | From another page, picks three songs one after another with "Play next". They will play in the order chosen. | Context menu | Each pick is a small operation against the last queue version the client saw. The server orders the operations and assigns versions. The picks go into the "Up next" lane in order. | MUS-117, MUS-062, DIS-111, MUS-122 |
| 7 | Adds a playlist with "Add to queue" and drags an album to the end. | Context menu; queue drop zones | Applies the same kind of operation. A multi-item drop is one operation. | MUS-118, MUS-063, CLI-062 |
| 8 | Opens the queue to reorder, remove and clear, and sees where each item is playing from. | Queue panel (full height on desktop, a sheet on phones) | Applies the same kind of operation. | MUS-119, MUS-123, CLI-060, CLI-149 |
| 9 | Turns on shuffle (random, spread out, or by album), reorders while shuffled, then reshuffles the rest. | Shuffle button and menu; queue menu | Stores the shuffle seed and order on the queue, so every device sees the same order. | MUS-126, MUS-127, MUS-120, MUS-128 |
| 10 | Switches on "Continue with" so music carries on after the queue, and sees what will play and why. | Queue panel toggle | Supplies the neighbour table. On the device, radio rules in the core pick the tracks. The lane is off by default. | MUS-129, DIS-070, DIS-062, DIS-060 |
| 11 | Loves a track from the bar, sets a sleep timer and saves the session as a playlist. | Now-playing bar; player menu; queue menu | Writes love events to the user log. Saving the queue creates a playlist (F06). | MUS-109, MUS-180, MUS-076, MUS-077, MUS-125 |
| 12 | Closes the tab, and later opens the client on the phone. | Player bar prompt "Continue on this device" | Nothing more: the queue and position are already in the phone's copy. | CLI-103, MUS-122 |
| All | Listens. | None | Records each play with its real timestamp, plus counts and skips, unless a private session is on. | MUS-182, MUS-183, ACC-117 |

**When it goes wrong.**

- Two devices edit the queue at once. The server rejects an operation
  built on a stale version, and the client rebases and shows the result
  (MUS-122). This rule has to be written and tested before handoff (F08) is
  built.
- A track cannot be decoded in this browser, or is damaged. It is dimmed
  before play, skipped with a reason and listed in the health report
  (MUS-229, MUS-079). F12 covers the rest.
- The signed URL expires during a long pause or a three-hour mix. The
  client should fetch a fresh one without the listener noticing
  ([G10](#gaps-and-questions-the-flows-expose)).
- Two browser tabs on one profile both try to play the queue (G11).
- The page is plain HTTP on a LAN address. Everything works online, but
  nothing plays once the server is unreachable (CLI-150).
- iPhone browsers restrict background audio, so R1's lock-screen and
  background behaviour on iOS is best effort (clients map, dependencies).

**Rivals today.** Plexamp already does gapless playback, loudness levelling
and a good queue, and it is the bar to clear; it runs only in Plex's own
apps, and some features need Plex Pass. Spotify is the reference for queue
lanes. Navidrome's web player will not do gapless (#745), and Jellyfin's
web gapless depends on the browser; gapless is Jellyfin's most-voted music
request (647 votes, open since 2019). Apple Music plays several "play next"
picks in reverse order, which is what MUS-117 fixes.

**Other releases.**

- R2: undoing queue edits (MUS-121); queue history (MUS-124); several
  saved queues (MUS-131); crossfade and album-aware fades (MUS-091,
  MUS-092); an equaliser (MUS-094); gapless and background playback in the
  native apps (MUS-068, MUS-074); the landscape player and info cards
  (MUS-112, MUS-111); handoff (F08).
- Later: a visualiser (MUS-083); listening together (MUS-202).

**Depends on.** ADR 3 (the queue lives in the user log); open decision 9
(amend ADR 2 to name the audio packager); open decision 8 (loudness
measurement). Which containers each browser accepts for the packager is
unverified.

---

## F06. Building a playlist

**Release: R1.**

**Who and where.** A listener in the web client. The flow covers a
hand-built playlist, a smart playlist and an imported one.

**Starts** with a synced library. **Ends** with playlists that follow the
person to every device and survive moves, upgrades and cache rebuilds.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Creates a playlist from the sidebar. | Sidebar; playlist page | Writes a playlist-created event to the user log with a random public ID. | MUS-132, INT-008 |
| 2 | Adds a track with "Add to playlist", searching their playlists, seeing where the track already is and adding it to several at once. | Add-to-playlist sheet | Writes playlist events that refer to tracks by content identity, not by path. | MUS-133, LIB-028 |
| 3 | Is asked before adding a track that is already there. | Add sheet prompt | Nothing: the check runs on the device. | MUS-134 |
| 4 | Selects twenty tracks across an album and search results and drags them onto the playlist in the sidebar. | Selection bar; sidebar drop target | Applies the batch as one operation. | DIS-110, MUS-063, CLI-062 |
| 5 | Reorders, removes and renames; finds a song inside a long playlist; sorts by date added. | Playlist page search and sort | Writes reorder, remove and rename events. Finding and sorting run on the device. | MUS-132, MUS-135, DIS-091 |
| 6 | Sees a cover made from the albums inside, pins the playlist, and adds it to the Home shortcuts. | Playlist tiles; sidebar; top of Home | Writes pin events. The client composes the cover from artwork it already holds. | MUS-137, MUS-139, DIS-013 |
| 7 | Exports the playlist as M3U8 to use elsewhere. | Playlist menu: export | Writes an M3U8 with paths relative to the library root. | MUS-140 |
| 8 | Starts a smart playlist. | Smart playlist editor | Nothing yet. | MUS-143, DIS-121 |
| 9 | Builds rules from menus, such as "genre is jazz, not played in a year, rating at least four, 50 at random", and watches the matches update. | Rule editor sheet | Nothing while editing: on the device, the core evaluates the rules against the synced library, with seeded randomness so the list is the same on every device. | MUS-144, MUS-145, MUS-146, DIS-119, DIS-120, DIS-122 |
| 10 | Saves, and the playlist keeps itself up to date. | Playlist page | Stores the rule tree in the rule store and syncs it. Evaluates rules on the server only for adapters and sync filters, and again when the library changes. | DIS-119, DIS-121 |
| 11 | Alternatively saves a library filter as a smart playlist, or plays "Loved tracks", which already is one. | Filter sheet "Save as"; Library > Loved | Uses the same rule store. | DIS-105, MUS-149, DIS-046 |
| 12 | Imports an M3U from another player and reads the match report. | Playlist menu: import; Admin > Migration > Review | Parses the file. Matches each entry by path relative to a library root, then by tags and duration, records the reason, and sends misses to the review queue. | MUS-140, ADM-043, ADM-044 |

**When it goes wrong.**

- An imported entry matches nothing. It waits in the unmatched queue with
  its reason (ADM-044).
- A track in a playlist is moved or upgraded. The entry follows its
  identity (LIB-029, LIB-030).
- A track is deleted. The entry survives the trash grace period and comes
  back with the file (LIB-033). What it shows after the file is purged is
  not settled ([G6](#gaps-and-questions-the-flows-expose)).
- A rule matches nothing. The editor shows that live, before saving
  (DIS-120).
- An `.m3u` found in a music folder became a playlist (LIB-192), but media
  is read-only (LIB-007), so an edit cannot be written back (G5).
- The playlist is edited on two devices. Both are online in R1, so the
  server orders the events; offline edits arrive in R2 (CLI-094).

**Rivals today.** Plex's smart playlists are good and free. Navidrome
offers more than 80 rule fields but makes people write JSON, and Jellyfin
has no smart playlists in its core (588 votes). Spotify's add-to-playlist
sheet and duplicate prompt are the reference for steps 2 and 3. Gunmetal's
edge is one free rule language with a live preview that drives home rows
now and download rules in R2.

**Other releases.**

- R1 also has a playlist write API, so tools can do steps 1 to 5 with a
  scoped token (INT-138, ACC-049).
- R2: playlist folders (MUS-136); a custom image (MUS-138); shared and
  collaborative playlists (ACC-091); offline edits that merge later
  (CLI-094); stable daily or weekly snapshots (MUS-147); keeping a smart
  playlist downloaded (CLI-080); pinning a list as a Home row (DIS-126);
  Navidrome smart playlist import (MUS-148); a share link to a playlist
  (ACC-086).
- Later: a playlist from a prompt (MUS-153); streaming-service playlist
  import (INT-153).
- No: suggestions inserted into a person's own playlists (MUS-130,
  DIS-182).

**Depends on.** ADR 3 (playlists and rules live in the user log).

---

## F07. Downloading for offline and playing on a plane

**Release: R2.** R1 has no downloads (see below).

**Who and where.** A listener with the Android app (CLI-004), downloading
music and, in this flow, a film and some episodes too.

**Starts** at home with a signed-in phone holding a device key. **Ends**
with downloads that play with no signal, and with plays, ratings and edits
made in the air merged when the phone reconnects.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Taps the download switch on an album, an artist and a playlist. | Download toggle on album, artist and playlist pages | Checks the person's download right. Issues signed byte-range URLs and an offline grant: the device key plus a signed record of what the device may play and until when. | CLI-078, ACC-044, CLI-095 |
| 2 | Sets rules: keep "Loved tracks" and one smart playlist downloaded, keep what was played recently, all within a 20 GB cap. | Downloads > Rules; "Keep downloaded" on any smart playlist; Settings > Storage | Supplies the rules in the shared rule language. On the device, eviction makes room for automatic downloads and never removes a hand-picked one. | CLI-080, CLI-081, CLI-082, DIS-119 |
| 3 | Chooses Opus copies to save space, Wi-Fi only, and the SD card. | Download quality setting; Settings > Downloads > Network; Settings > Downloads | Encodes each Opus copy once in the sandboxed worker and caches it for the next device. | CLI-083, MUS-213, CLI-091, CLI-059 |
| 4 | Downloads a film and keeps the next three episodes of a show. | Download button on films and episodes; Show page > Keep next episodes | Serves originals, or a remuxed copy where the device's player prefers another container. Skip markers, chapters, subtitles and fonts travel with each download. | CLI-084, CLI-085, CLI-087, CLI-088, VID-175 |
| 5 | Closes the app. Downloads continue and resume after a dropped connection. | Notification; Downloads screen | Serves resumable ranges and refreshes URLs on request. Streams on other devices take priority over downloads. | CLI-090, ACC-110 |
| 6 | Checks the downloads manager before leaving. | Downloads screen | Returns typed errors that the app turns into plain reasons. | CLI-089, MUS-215 |
| 7 | On the plane, browses as usual; a "Downloaded" filter narrows any list. | All lists; "Downloaded" filter | Nothing: the synced library, the search index and the grant are on the phone. | CLI-026, MUS-209, CLI-025, DIS-084 |
| 8 | Plays an album with lock-screen controls and lyrics, then the film with subtitles. | Player; lock screen; lyrics view | Nothing. | CLI-069, MUS-074, MUS-160, VID-175 |
| 9 | Rates tracks, edits a playlist and stops the film halfway. | Context menu; playlist page; player | Nothing yet: on the device, the edits queue as operations. | CLI-094, MUS-214, VID-118 |
| 10 | Lands and reconnects. | Sync status; History, showing the device | Ingests the offline plays with their real timestamps and removes duplicates. Applies the queued edits under the stated conflict rules, updates resume points and renews the grant. | CLI-093, CLI-094, VID-118, CLI-095 |

**When it goes wrong.**

- Storage fills up. Automatic downloads make room, and hand-picked ones
  stay (CLI-082).
- The phone is on mobile data with "Wi-Fi only" set. Downloads wait and say
  so; the rule is never broken silently (CLI-091). Plexamp downloaded over
  cellular while showing "paused" in September 2026.
- The trip outlasts the grant. Downloads stop playing until the phone makes
  contact, and the Downloads screen shows the expiry beforehand (CLI-095).
  The clients map recommends an admin-set lifetime with a 30-day default
  (clients map, open decision 4).
- The owner revokes a lost phone's downloads. That takes effect only when
  the phone next makes contact (ACC-045, CLI-096); F15 says so plainly.
- An offline edit conflicts with a change made at home. A rare conflict
  notice says which change won (CLI-094).
- The person has no download right. The switch explains why (ACC-044).

**Rivals today.** Plexamp's offline music is good: "Keep Played Music"
arrived in September 2026 and its download cap is gone, but downloads need
Plex Pass. Finamp downloads for free, and Symfonium caches automatically.
Emby is best at letting an admin see and control synced media. Offline is
Jellyfin's most-voted request (1,820 votes). Gunmetal's edge is free,
rule-driven downloads under a cap, offline that uses the same screens, and a
grant the owner can revoke.

**What R1 offers instead.** No downloads. The web client fetches upcoming
tracks early on patchy signal (CLI-099). In a secure context, the installed
web app can browse and search its synced copy with no server (CLI-025), but
it plays only what it has already buffered. Downloads in the browser are
Later (MUS-217). Open decision 5 recommends shipping the Android music app
as an R1 point release, which would bring music downloads forward.

**Other releases.**

- R2 also lets a person start a phone download from another device
  (CLI-097).
- Later: smaller transcoded video downloads (CLI-086); offline music on
  Wear OS and Apple Watch (CLI-128, CLI-129); copying to a folder or drive
  (CLI-100).

**Depends on.** ADR 3 (grants, rules and queued edits are durable user
data); the remuxer and the sandbox (R2 infrastructure); clients map open
decisions 4 and 5.

---

## F08. Moving playback from the phone to another device

**Release: R2.** R1 offers "continue on this device" (see below).

**Who and where.** A listener playing on the Android app, with the desktop
app, a TV app or a browser tab signed in to the same profile, or a
Chromecast speaker on the network.

**Starts** with music playing on the phone. **Ends** with the same queue,
lanes and position playing on the chosen device, and the phone acting as
its remote.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Taps the device button. | Device picker in the player bar and Now Playing | Keeps a WebSocket control channel per signed-in session and reports which of the profile's players are present. | CLI-101, MUS-197 |
| 2 | Sees the living-room TV, the desktop and a Chromecast speaker, each with its state. | Device picker | Lists only the players this profile may control. | CLI-101, ACC-121 |
| 3 | Picks the desktop. | Device picker; accept prompt on the target where needed | Sends a transfer command over the control channel. The target loads the queue document at the current version, with its lanes, shuffle order and position, and starts. The phone stops. | CLI-101, MUS-122, MUS-116, LAT-009, VID-144 |
| 4 | Controls the desktop from the phone, including its volume. | Remote mode in Now Playing | Relays play, pause, skip, seek and volume commands, each authorised per profile. | CLI-102, MUS-198 |
| 5 | Edits the queue on the phone while the desktop plays. | Queue panel in remote mode | Applies queue operations to the shared, versioned document, and the target follows them. | MUS-122, MUS-119 |
| 6 | Alternatively casts to the Chromecast speaker. | Cast button; device picker | Signs refreshable URLs the receiver can fetch itself on the home network, and picks a format the receiver can decode. Away from home, the phone relays. | CLI-106, MUS-203, CLI-110, CLI-111 |
| 7 | Stops or adjusts casting from the notification. | Notification | Nothing beyond the control channel. | CLI-112 |
| 8 | Takes playback back to the phone when leaving the house. | Device picker on the phone | Runs the same transfer in the other direction. | CLI-101 |

**When it goes wrong.**

- Two devices send commands at once. The server orders them by queue
  version under a written conflict rule; CLI-101 requires that rule to be
  designed and tested before CLI-101 or CLI-102 is built.
- The target fails to start. The map does not say what happens
  ([G14](#gaps-and-questions-the-flows-expose)).
- The cast receiver cannot decode the file. The decision engine picks a
  path the receiver can play (CLI-111). URL lifetimes must cover a whole
  film or be refreshable, because receivers cannot join an iroh connection
  (clients map, risks).
- Someone tries to control another person's player. It is refused; control
  with the other person's grant is Later (ACC-047). Jellyfin had a
  broken-access-control advisory on session control in September 2026.

**Rivals today.** Spotify Connect is the reference and is excellent, but it
runs through Spotify's account service. Plexamp hands off between its
players through plex.tv. Jellyfin can already remote-control its sessions
without a vendor account. Gunmetal's edge is one versioned queue that any
of the profile's devices can take over with the same lanes and position,
with no central account.

**What R1 offers instead.** R1's only players are browser tabs, so there is
no device picker. The queue persists on the server, and opening the client
elsewhere offers "Continue on this device" (CLI-103, MUS-122). How a
second tab and the first one share a queue is not stated (G11).

**Other releases.**

- R2 also brings Sonos and whole-home audio through Music Assistant over
  the OpenSubsonic adapter (MUS-206).
- Later: AirPlay (CLI-109); headless and dedicated players (CLI-104);
  synchronised multi-room (CLI-105, a Spotify request with 8,800 votes);
  controlling another person's player with their grant (ACC-047); listening
  together (MUS-202).

**Depends on.** The control-channel design and its conflict rule (CLI-101);
open decision 10 (relays) for control away from home; clients map open
decision 7 (Chromecast receiver).

---

## F09. Searching across music and video

**Release: R1 for music; films, shows, episodes and people join in R2.**

**Who and where.** Anyone, in the web client in R1 and on every client
from R2.

**Starts** with a synced library. **Ends** with the person at, or playing,
the thing they wanted.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Focuses the search field from any screen and sees recent searches. | Global search field; Search screen | Nothing. | DIS-083, DIS-089 |
| 2 | Types "amelie", and results appear with each keystroke, grouped by type with chips and with no cap. | Search screen; type chips | Nothing at query time. On the device, the core builds the index from the synced library. If that build is too slow on the reference low-end device, the server ships a prebuilt index segment instead. | DIS-084, DIS-083, MUS-061 |
| 3 | Finds "Amélie" despite the missing accent, a typo and a straight apostrophe. | Search | Nothing. | DIS-085 |
| 4 | Types "christmas" and gets the tag and the genre as well as titles. | Search; genre chips | Sends tags, genres and moods in the sync feed. | DIS-086, MUS-017, MUS-019 |
| 5 | Types a composer's name and gets the person, with their roles. | Search; artist page role tabs | Sends credits with their roles in the sync feed. | DIS-087, MUS-005 |
| 6 | Narrows the search to one library, or to Music. | Scope selector | Nothing. | DIS-088 |
| 7 | Plays a result straight away, queues it, or opens it. | Context menu; album page | Nothing beyond F05. | DIS-111, CLI-034 |
| 8 | Goes back and finds the results and scroll position as they were. | Navigation | Nothing. | DIS-112 |
| All | Never sees anything this profile may not see. | Every result | Builds the sync payload from the person's grants and restrictions, so the index on the device never holds a blocked item. | ACC-030, ACC-037, DIS-140 |

**When it goes wrong.**

- The person is on a plane or the server is down. Search still works from
  the local copy over HTTPS or localhost (DIS-084, CLI-025). Over plain
  HTTP, the web client cannot keep its copy across reloads (CLI-150).
- Nothing matches. The empty state should offer to widen the scope.
- The library is large and the device cheap. Index size and build time are
  measured against the budget: under 50 ms per search at 100,000 tracks, a
  proposed goal (open decision 16).

**Rivals today.** Plex's search has been relevance-ranked since 2023 and
has no per-source cap; it is reasonably good. Spotify is fast and
typo-tolerant (unverified). Jellyfin has no typo tolerance (46 votes),
broke accent matching in 10.11.0, and does not search tags (256 votes since
2019). No rival server searches on the device; Plexamp's offline mode,
since September 2026, includes search.

**Other releases.**

- R2: one box covers films, shows, episodes and people (actor, director,
  character) beside music, as the R1 rows DIS-083 and DIS-087 already
  promise for R2, with kind chips (LAT-013). A
  title held in two libraries or editions shows once with its versions
  (DIS-095). Music and Watch get separate homes (DIS-017), so search needs
  a rule for which kinds it covers
  ([G16](#gaps-and-questions-the-flows-expose)). R2 also adds search by
  lyrics (DIS-093), transliteration and CJK (DIS-092), voice search on TV
  and phone (DIS-094, CLI-044), and a keyboard command palette (DIS-090).
- R3: programmes on air or coming up appear in search (DIS-096, LIV-072).
- Later: results outside the library (DIS-097); several servers at once
  (DIS-099); search plugins (DIS-098); the operating system's own search
  (DIS-174).

**Depends on.** Open decision 16 (performance budgets).

---

## F10. Sharing a library with a friend

**Release: R1, when the owner gives the server an outside address; the
no-open-port path arrives in R2.**

**Who and where.** The owner in the web client's admin section, and a
friend in another house with a browser.

**Starts** with the owner's library and, in R1, a server the owner has made
reachable from outside with their own domain, reverse proxy or VPN.
**Ends** with the friend holding an account limited to chosen libraries,
signed in on their own device.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | The owner makes the server reachable from outside: a domain with HTTPS behind a reverse proxy, a VPN, or a path prefix next to other services. | Admin > Network; Admin > Network > HTTPS | Honours a trusted-proxy list when resolving each client's address, serves under the configured base path, and serves TLS with the owner's certificate. | ACC-097, ACC-134, ACC-098, ADM-022 |
| 2 | The owner creates an invite: which libraries, how many uses, when it expires. | Admin > Invitations | Stores the invite as a capability with an expiry, a use count and a policy that carries the library grants. | ACC-080, ACC-037 |
| 3 | The owner sends the link or shows the QR code. | Admin > Invitations | Puts the configured public address in the link. | ACC-080 |
| 4 | The friend opens the link. | Invite landing in the web client | Checks the invite is valid and unexpired, and reveals nothing else about the server. | ACC-080, ACC-120 |
| 5 | The friend creates an account and enrols a passkey on the spot, or a password and two-factor where passkeys cannot work. | Invite landing; Account > Sign-in methods | Creates a local account in the identity store from the invite's policy and logs the redemption. | ACC-080, ACC-006, ACC-050, ACC-052, ACC-053 |
| 6 | The friend sees only the shared libraries. | Home; library views | Builds the friend's synced library from their grants and checks authorisation on every object fetch. | ACC-037, ACC-121, ACC-030, MUS-027 |
| 7 | The owner sees the friend listening. | Admin > Dashboard (Now playing) | Lists live sessions from the session registry. | ADM-099 |
| 8 | The owner changes the friend's libraries, pauses the account, or removes it. | Admin > Users | Updates grants, so the next sync removes what the friend may no longer see. Disabling bumps the account's session epoch, which ends open sessions and cuts streams in flight. | ACC-037, ACC-008, ACC-122 |

**When it goes wrong.**

- The owner has no outside address. The link carries an address the friend
  cannot reach, and the map has no warning for this
  ([G7](#gaps-and-questions-the-flows-expose)).
- The link leaks. It expires, has a use count and can be revoked, and every
  redemption is logged (ACC-080, ADM-110).
- Someone guesses invite codes or passwords. The shared limiter applies
  (ACC-063).
- The friend wants to know what the owner can see. In R1 there is no page
  for that, because ACC-115 is R2 (G8).

**Rivals today.** Plex is genuinely best here: a friend in another house
gets chosen libraries in the Plex app they already have, with no address to
type. Since 29 April 2025, though, remote video needs Plex Pass for the
owner or a pass for each friend; music is exempt. Jellyfin has no
invitations (its sign-up request #494 has 227 votes), so owners add Wizarr
or jfa-go and run their own proxy. Navidrome and Immich already have
public share links for people with no account.

**Other releases.**

- R2: native apps reach home over iroh with no open port, no account and no
  fee, and the invite carries the server's node address and key, so the
  friend types nothing (ACC-096, ACC-083, ACC-100, ACC-101). An onboarding
  page says "install this app, then tap here" (ACC-082). Membership can end
  on a date (ACC-081). Remote access can be on or off per person (ACC-103).
  Quality caps never transcode silently (ACC-107, ACC-108). Stream limits
  count playing, not browsing (ACC-075). The house's upload is shared
  fairly (ACC-109). Policies can be named and shared (ACC-038). Each person
  can see what admins see about them, and the owner chooses that (ACC-115,
  ACC-116). Share links for an album or playlist, with a password, an
  expiry and a download switch, reach people with no account (ACC-086,
  ACC-087, ACC-088, ACC-089), as does a single-item guest link (ACC-135).
  Playlists can be collaborative (ACC-091). People can be re-invited from
  an old server (ACC-084, ADM-048).
- Later: remote access in the browser over iroh (ACC-102); asking to join
  (ACC-085); link previews in chat apps (ACC-090).

**Depends on.** Open decision 10 (who runs relays); open decision 13 (what
admins can see).

---

## F11. Migrating from Plex or Jellyfin

**Release: R1 for listening-service export files and M3U playlists;
importing from the rival servers' own databases arrives in R2.**

**Who and where.** The owner, in the admin section, with the old server's
files still to hand.

**Starts** with Gunmetal set up (F01) and the same music scanned (F02).
**Ends** with history and playlists attached to the right files, and a list
of what did not match.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Answers "Coming from another server?" at setup, or opens Migration later. | Welcome > Import; Admin > Migration | Lists the importers this release has. | ADM-030 |
| 2 | Uploads Last.fm or ListenBrainz export files. | Admin > Migration > Listening services | Parses the files locally, with no network grant. Marks each listen as imported, so a scrobbler never sends it back. | ADM-042, INT-107, MUS-189 |
| 3 | Uploads a folder of M3U playlists exported from the old server, where it can export them. | Admin > Migration > Playlists | Parses M3U and M3U8. | ADM-043, MUS-140 |
| 4 | Waits for matching. | Admin > Tasks; Admin > Activity | Runs the one matcher every importer shares: path relative to a remapped root, then MusicBrainz IDs, then tags and duration. Records a reason and a confidence for each match. | ADM-044 |
| 5 | Reads the match report and works through the unmatched queue. | Admin > Migration > Review | Holds low-confidence matches for a yes. | ADM-044, LIB-099 |
| 6 | Sees years of plays in history and the playlists in the sidebar. | History page; sidebar | Writes the imported plays and playlists to the user log, keyed by content identity. | MUS-183, MUS-132, LIB-028 |

**When it goes wrong.**

- Paths changed between servers. The matcher remaps roots and falls back to
  tags and duration (ADM-044).
- An import goes badly. In R1, imported events are marked and can be removed
  as a batch (INT-107); a full undo is R2 (ADM-046).
- The owner is leaving Plex. Plex has no built-in music playlist export
  (one forum user spent weeks rebuilding 614 items), so in R1 a Plex
  owner's playlists depend on third-party tools. Play counts and ratings
  held only in Plex's database cannot come across until R2. Whether
  Jellyfin exports playlists is unverified.

**Rivals today.** No media server imports from another one out of the box.
People use third-party tools such as WatchState (about 1.6k stars) and
JellyPlex-Watched, which has a dry-run mode. Moving between Jellyfin
servers is good, because users travel inside its backup. Plex's own
cross-server sync excludes music.

**Other releases.**

- R2: import music history, ratings and playlists from a copy of Plex's
  database (ADM-036, MUS-141), and video watch state (ADM-037); music and
  video state from Jellyfin and Emby (ADM-038, ADM-039); stars, ratings and
  playlists from Navidrome and other Subsonic servers (ADM-040); an iTunes
  library file (ADM-041). Every importer gets a dry run (ADM-045) and an
  undo (ADM-046), can read a rival's API instead of a database copy
  (ADM-047), brings users over as invitations with their old library access
  (ADM-048, ACC-084), and can sync again while both servers run (ADM-049,
  INT-116). NFO files are read as they are (LIB-126) and ratings come from
  tags (MUS-045). Friends' existing apps keep working through the Jellyfin
  adapter's music subset (INT-098) and the OpenSubsonic adapter (INT-086,
  F19).
- Later: collections and metadata edits (ADM-050, LIB-182); the Jellyfin
  adapter's video subset (INT-099).
- No: Plex API compatibility (INT-101).

**Depends on.** ADR 3; fixtures from several versions of each rival's
database schema (ADM-036).

---

## F12. Recovering from a failed playback

**Release: R1 for music; the video cases arrive in R2.**

**Who and where.** The listener first, in the web client; then the owner,
in the admin section.

**Starts** with something that will not play, or stops. **Ends** with
either music playing again, or a clear reason and a next step for both the
listener and the owner.

This flow is a set of cases rather than one path. Each row is one way
playback can fail, with what the listener should see.

**Listener cases in R1.**

| Case | What the listener sees | Surface | The server | Features |
|---|---|---|---|---|
| A. This browser cannot decode the format, for example ALAC in some browsers (unverified) | The track is dimmed with a reason before anyone presses play, and the queue skips it with a note. | Track rows; queue; Library health | Sends codec and container per file in the sync feed. On the device, the core's capability probe decides. The health report counts such files by format and browser. | MUS-229, MUS-032 |
| B. The file is damaged | The player skips it with a notice that links to the health report, and never plays noise. | Notice; Library health > Problems | Flags the damage at scan time. The player keeps a true-peak ceiling. | MUS-079, LIB-193 |
| C. The connection drops | Music carries on from what was fetched ahead, a quiet banner says the server is unreachable, and playback resumes when it returns. | Quiet status banner; player | Serves byte ranges, so the player resumes exactly where it stopped. | CLI-099, MUS-070, CLI-025 |
| D. The stream URL expired during a long pause | Nothing visible: the player gets a fresh URL. | None | Signs a new URL while the session is valid. | ACC-122 |
| E. An admin ended the session, the device was revoked, or the account was disabled | Playback stops with the admin's message, followed by the sign-in page. | Client banner; sign-in page | Rejects new requests by session epoch and cuts responses in flight. | ADM-102, ACC-069, ACC-008, ACC-122 |
| F. The drive holding the file is offline | The item is greyed out with an offline badge, and pressing play says why. | Greyed-out items | Checks root health, marks the root offline and alerts the admin. | LIB-032, ADM-108, ADM-116 |
| G. The file was moved or deleted | A move inside the library roots changes nothing. A missing file is listed, and it returns with its history if restored within the grace period. | Library health > Missing files | Detects moves and keeps deleted entries in a trash with a grace period. | LIB-029, LIB-034, LIB-033 |
| H. The server is restarting or upgrading | A status page instead of a refused connection, and the queue intact afterwards. | Startup page | Opens its listener before the database. | ADM-032, MUS-122 |

**The owner investigates (R1).**

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Sees which sessions failed and the reason for each. | Admin > Sessions; session detail | Stores the structured reason from the core's decision engine per session, and exposes it in the API. | ADM-100, INT-134 |
| 2 | Inspects the file. | Admin > Diagnostics > Inspect a file; Item menu > File info | Runs the core's parsers through the inspect API and shows raw tags, structure, and any error with its location. | ADM-125, LIB-195, MUS-114 |
| 3 | Asks the listener for a report. | Settings > Help > Diagnostics, on the listener's device | On the device, builds the report, redacted by default and saved as a file the listener chooses to send. Adds a server log excerpt on request. | CLI-033 |
| 4 | Runs the doctor and builds a bundle when asking the project for help. | Admin > Diagnostics; CLI | Runs the check registry, builds a bundle with a preview and masking, and includes local crash records. | ADM-123, ADM-124, ADM-130 |

**Video cases in R2.**

| Case | What the viewer sees | Surface | The server | Features |
|---|---|---|---|---|
| I. The device cannot open the container | The film plays anyway, and the info overlay says it is being remuxed and why. | Player info overlay | The core's decision engine picks a remux, and the in-process remuxer repackages without touching picture or sound. | VID-002, VID-003, VID-168, VID-169 |
| J. The codec is unsupported and transcoding is off, unavailable, or not allowed for this person | An error card offers another version, a download for later, or another device. | Player error card | Treats policy as an input to the decision, so the refusal comes with its reason and the alternatives. The sandbox self-test says plainly when transcoding cannot run. | VID-010, ACC-043, VID-009, ADM-133, VID-013, CLI-084, CLI-101 |
| K. The link is too slow for the original | A short choice: the original with a bigger buffer, another version, an audio-only conversion, or a download. | Pre-play sheet | Compares the measured link with the file's peak bitrate per segment from the segment map. | VID-024, VID-014 |
| L. Too many streams | A card that names the limit. | Player limit card | Counts playing leases, not open apps. | VID-173, ACC-075 |
| M. A Dolby Vision file on a non-Dolby Vision device | The HDR10 base layer plays, and the overlay names the fallback. | Player info overlay | The remuxer handles the HEVC NAL units. | VID-040 |
| N. A stall nobody can explain | One tap sends the owner a redacted report, and the owner opens the session trace. | Player error card; Admin > Sessions > Trace | Joins the server's decision and bytes sent with the client's buffering and error reports for a short time. | VID-174, ADM-126 |

Before play, a "plays directly here" badge warns of a remux or transcode
in advance (VID-015).

**Rivals today.** Plex and Jellyfin both show playback information, though
how much is unverified; mpv's statistics page and Infuse's overlay are the
best. Jellyfin's Android TV app added a media capability report in 0.19.
The research found no rival that marks an unplayable track before play.
Plexamp sends download telemetry, including titles and the account name,
with no off switch; Gunmetal's reports are read before they are sent.

**Gaps.** In R1 the only fix for case A is another client, and R1 has none
(G13). Case D depends on silent URL refresh (G10).

**Depends on.** Open decision 9 (the audio packager); the decision engine
in the core (ADR 1, decision 2); the remuxer and sandbox for the R2 cases.

---

## F13. Upgrading the server and rolling back

**Release: R1.**

**Who and where.** The owner, at the host and in the admin section.

**Starts** with a running server and a new release available. **Ends** with
the new version serving, or the old one back, with nothing lost either way.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Learns that a new version exists, if the update check was allowed. | Admin > Updates; dashboard update card | Fetches a signed feed daily through the egress gate and verifies it against a pinned key. Contacts nobody when the check is off. | ADM-053, ADM-028 |
| 2 | Is told plainly if the running version has a known security problem. | Admin banner; Admin > Updates | Compares the running version with the advisory ranges in the feed. | ADM-054, ACC-126 |
| 3 | Reads whether this release can be rolled back. | Admin > Updates; release notes | Shows the rollback-safety field from the feed. | ADM-060 |
| 4 | Replaces the binary, or pulls the new container tag, and restarts. | Host | Nothing until it starts. | ADM-001, ADM-003 |
| 5 | Sees the startup page while the server migrates. | Startup page | Takes an automatic snapshot, runs migrations on a copy, reports the result and the time taken, and serves only after they pass. Upgrades from any older version in one step. | ADM-056, ADM-057, ADM-058, ADM-032 |
| 6 | Is back in, with no full rescan. | Admin > Activity, for example "Re-reading MP4 files after a parser update" | Rereads only files whose parser version changed and keeps derived data such as loudness. | LIB-025, ADM-141 |
| 7 | If the new version misbehaves, starts the previous binary. | Startup page ("rebuilding: data was written by a newer version") | An older binary that finds a newer cache discards it and rebuilds from the files plus the log, skipping event types it does not know. The pre-upgrade snapshot is the fast path when its version matches. | ADM-059, ADM-077, ADM-056 |

**When it goes wrong.**

- A migration fails. It ran on a copy, the old data is untouched, and the
  startup page says what failed (ADM-057).
- The power goes during a write. The user log recovers from a torn write
  (ADM-078).
- The main client will not load. The emergency page still offers status,
  logs, a backup and a restart (ADM-113).
- A TV app cannot be updated yet. Old clients keep working (CLI-032).

**Rivals today.** Jellyfin is good at the safety net: it copies the old
database before migrating, 10.11 added a startup page that shows migration
progress, and 12.0 added a mode that runs migrations and exits. Its 10.11
migrations still failed for many people, and it cannot skip versions (12.0
needs 10.10.7 or any 10.11 first). Plex keeps only its routine three-day
database backup, but in practice it is the easiest to roll back, because
reinstalling the older build has fixed past regressions. Plex can also
email owners of a vulnerable version, which Gunmetal cannot do without a
central account; the advisory banner in step 2 is the substitute.

**Other releases.** R2: release channels with free release candidates
(ADM-055), withdrawn releases flagged (ADM-061), and a plugin check before
upgrading (ADM-063). Later: opt-in automatic updates with a health-check
rollback (ADM-064).

**Depends on.** ADR 3 (a forward-compatible user log); open decision 25
(release signing keys).

---

## F14. Rebuilding a dead server from a backup

**Release: R1.**

**Who and where.** The owner, at a fresh install on new hardware, a new
operating system or a container.

**Starts** with the old machine dead or retired and a backup file to hand.
**Ends** with the same accounts, history, playlists and settings on the new
machine, and every library root pointed at its new path.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Before anything breaks, has daily backups that are verified, explained and downloadable. | Admin > Backups; backup detail; dashboard health card | Backs up a consistent snapshot of the user log, the identity store and settings, with a manifest. Verifies each backup after writing and alerts on failure. | ADM-065, ADM-066, ADM-072, ADM-069, ADM-116 |
| 2 | Installs Gunmetal on the new machine, starts it, and enters the setup code. | Console; Welcome > Setup code | As in F01, steps 1 to 5. | ADM-001, ACC-001 |
| 3 | Chooses "Restore from a backup" and uploads or picks the file. | Welcome > Restore | Verifies the archive while still in setup mode. | ADM-029 |
| 4 | Sees where each library root used to be and points it at its new path. | Welcome > Restore > "Where are your libraries now?" | Checks every root is reachable and offers to remap moved roots, with a dry run. | ADM-051, LIB-031 |
| 5 | Watches the restore. | Restore progress page | Restores the log and the identity store, then rebuilds the cache from the files, reusing derived data when the backup includes it. | ADM-029, ADM-077, ADM-141 |
| 6 | Signs in. | Sign-in page | Uses the restored identity store. | ACC-050, ACC-052 |

**When it goes wrong.**

- The server is now reached at a different domain. Every passkey breaks,
  because a passkey belongs to the domain it was made for
  ([G3](#gaps-and-questions-the-flows-expose)). Members fall back to a
  password or a one-time link (ACC-064).
- A root cannot be found. The restore names it and offers the remap
  (ADM-051).
- A restore on a running server fails. The UI restore takes a restore point
  first and shows a preview (ADM-070).
- The owner prefers a shell. `gunmetal restore` does the same (ADM-071).

**Rivals today.** Immich's restore button on the welcome screen, with
checks that the library folders are readable, is the model, and Gunmetal
is at parity with it. Jellyfin restores from the command line only
(`--restore-archive`) and needs identical media paths. Plex documents a
manual move on the same operating system only, and does not officially
support moving between systems. Navidrome can remap missing files, but the
remap cannot be undone.

**Other releases.** R2: encrypted backups (ADM-068); off-site destinations
(ADM-073); an optional cache snapshot for a faster restore (ADM-067).
Later: point-in-time recovery from the log (ADM-076).

**Depends on.** ADR 3 (the backup is the user log and the identity store).

---

## F15. Losing a phone

**Release: R1 for sessions; revoking downloads arrives in R2.**

**Who and where.** A member on another device, or the owner acting for
them.

**Starts** with a lost or stolen phone that is signed in. **Ends** with the
phone unable to reach the server, and in R2 its downloads revoked on next
contact.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Signs in elsewhere and opens their devices. | Account > Devices | Lists every device, app password and token with its last-seen time. | ACC-068 |
| 2 | Removes the lost phone, or signs out of everything. | Account > Devices > Remove; Account > Devices (sign out of all sessions) | Revokes the device so its new requests fail at once and its open responses are cut. "All" bumps the session epoch that every session and signed URL carries. | ACC-069, ACC-070, ACC-122 |
| 3 | Changes the password if the phone could reveal it, confirming with a fresh passkey touch where one exists. | Password change dialog; confirmation prompt | Bumps the session epoch, ending every other session. | ACC-065, ACC-055, ACC-056 |
| 4 | Checks their sign-in history. | Account > Sign-in history | Shows the person's own audit entries. | ACC-078, ADM-110 |
| 5 | The owner sees the event and, if needed, pauses the account. | Admin > Activity; Admin > Users | Applies the same epoch mechanism. | ADM-110, ACC-008 |

**When it goes wrong.**

- The lost phone was the only device. An admin issues a one-time sign-in
  link (ACC-064); the owner uses host recovery (ACC-004).
- In R2 the phone is offline. Its downloads keep playing until the grant
  expires, because revocation takes effect only on next contact (CLI-095,
  CLI-096). The docs must say this plainly (clients map, risks).

**Rivals today.** Plex offers device revocation and "sign out of all
sessions" at plex.tv, recommended after its 2025 breach; that is good.
Jellyfin revokes devices and needed a 12.1 fix to end their open sessions.
Emby is best at controlling synced media. Navidrome's shared stream URLs
kept working after deletion until September 2026, which is the failure
ACC-122 is designed against.

**Other releases.** R2: revoke a device's downloads (ACC-045, CLI-096);
device keys that can be revoked one by one (ACC-051); new-device alerts
(ACC-071). Later: spotting shared passwords (ACC-077).

**Depends on.** ADR 3 (identity store); clients map open decision 4 (grant
lifetime).

---

## F16. Keeping your own record

**Release: R1.**

**Who and where.** A listener in the web client.

**Starts** with a history that is the person's own. **Ends** with the
person having played something unrecorded, removed a play, dismissed an
item and taken a full export.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Turns on a private session, then plays something. | Player > Private session; private indicator | Plays through the "not recorded" path: no history, no recommendations, and from R2 no scrobbles. | ACC-117, MUS-185, DIS-053 |
| 2 | Turns it off, and plays are recorded again. | Private indicator | Returns to normal recording. | ACC-117 |
| 3 | Opens history by date and finds last March. | History page | Nothing at read time: history is in the profile's log on the device. | MUS-183, DIS-050 |
| 4 | Removes a play. | History row menu | Writes a removal event, and counts, statistics and recommendations follow. | MUS-184, ACC-118, DIS-052 |
| 5 | Dismisses an album from Continue listening, then undoes it. | Card context menu; undo toast; Hidden page | Writes a dismiss event, then its reversal. | DIS-022, DIS-023 |
| 6 | Exports everything they have told the server. | Account > Your data | Builds an export from the user log in a documented format: history, loves, ratings, playlists, hides, layouts and rules. | ACC-010, DIS-058, MUS-188, INT-151, LAT-007 |

**When it goes wrong.** The person forgets the private session is on. Open
decision 14 recommends it stays off by default, one tap from the player,
and clearly shown while on.

**Rivals today.** These are streaming-app habits that media servers lack.
Removing a play has 5,458 votes on Spotify's board and browsing history by
date 2,035; Jellyfin's dismiss-with-undo request has 1,725. No media server
the research checked documents private sessions (unverified).

**Other releases.** R2: scrobble filters that honour private listening
(INT-105); a page showing what admins can see (ACC-115); charts and a year
in review (MUS-186, MUS-187); hide and snooze (DIS-054).

**Depends on.** ADR 3; open decisions 13 and 14.

---

## F17. Watching a film on the TV

**Release: R2.**

**Who and where.** A viewer on the Android TV app paired in F04.

**Starts** with a film library and a signed-in TV. **Ends** with the film
played as the original, resumed later on another device.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Picks a row from Continue Watching or Next Up, or browses Films with the left rail. | TV Home; TV rail | Builds the rows from the watch log and the synced library. | DIS-024, DIS-025, CLI-035, CLI-036 |
| 2 | Opens a film and sees a badge saying it plays directly here, its versions and its tracks. | Title detail page; pre-play sheet | Nothing: on the device, the core runs the playback decision against the synced stream index and the TV's capability report. | VID-015, VID-013, VID-014, VID-053, CLI-047, VID-002 |
| 3 | Presses play, and the picture appears at the film's own frame rate. | Player | Signs a session-bound URL and serves byte ranges of the original. The TV's libmpv player decodes it. | VID-001, VID-011, VID-176, CLI-046, VID-012 |
| 4 | Sees styled subtitles exactly as authored, in the preferred language. | Player subtitle quick menu | Serves the subtitle stream and the fonts attached to the file through signed URLs. | VID-069, VID-068, VID-071, VID-051, VID-052 |
| 5 | Skips the intro. | Player skip button | Supplies skip markers from chapter titles or analysis, stored as logged data. | VID-110, VID-111, VID-112, VID-114 |
| 6 | Checks how it is playing. | Player info overlay | Supplies the decision trace, redacted by role. | VID-168, VID-169 |
| 7 | Stops halfway, then resumes later on the phone. | Resume prompt | Records the position as an event, so resume works on any device, offline included. | VID-118, VID-119 |
| 8 | At the credits, sees the next item or a post-play screen. | Post-play screen; countdown card | Counts the film as watched when its credits start. | VID-122, VID-123 |

**When it goes wrong.** F12 covers cases I to N.

**Rivals today.** Plex, Jellyfin and Emby all direct play on capable
clients and all resume and suggest the next episode; mpv-based players,
Kodi and Infuse already play nearly any file as it is. Gunmetal is at
parity on native direct play. Its edges are the badge before play,
subtitles rendered on the device that never force a transcode (Plex's
Android TV player cannot render ASS), and a plain reason whenever anything
changes. Jellyfin's Android TV app has had HDR and audio passthrough issues
open for years (127 and 50 comments).

**Other releases.** Later: true Dolby Vision output on Dolby Vision
televisions (VID-184, open decision 19); hardware transcoding (VID-006);
watching together (VID-153, open decision 18); Apple TV (CLI-007).

**Depends on.** The remuxer, sandbox and segment map (R2 infrastructure);
open decisions 19 and 20.

---

## F18. Setting up a child's profile

**Release: R2.**

**Who and where.** A parent in the web client, then a child on the
living-room TV.

**Starts** with a household of adults. **Ends** with a child profile that
can see only what it is allowed, on every device, with nothing leaking
through search, artwork or screensavers.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Creates a household and adds a child with a name and picture, and no email or password. | Admin > Household > Add a child | Creates a managed profile linked to a policy. | ACC-016, ACC-018, ACC-011 |
| 2 | Picks a preset: younger child, older child or teen. | Add a child > preset chooser | Copies a built-in policy template. | ACC-023 |
| 3 | Adjusts it: no explicit music, a film and TV rating ceiling for their country, only items labelled for the children, and listening hours. | Child-profile settings, with the rule editor | Stores the rules in the shared rule language, the ceiling per country system, the explicit flag from tags, and the schedule. | ACC-024, ACC-026, ACC-027, ACC-028, ACC-025, ACC-032 |
| 4 | Locks the adult profiles with PINs. | Profile settings; PIN pad | Stores a PIN hash per profile behind an attempt limiter. | ACC-020 |
| 5 | The child picks their profile on the TV. | TV profile picker; Kids home | Builds the child's sync payload with blocked items removed before it reaches the device, and checks signed URLs again. | ACC-019, DIS-144, ACC-030, DIS-152 |
| 6 | Outside the allowed hours, the child sees a clear message. | Blocked-time message | Enforces the schedule when URLs are issued and in offline grants. | ACC-032 |
| 7 | A parent allows one album without loosening the profile. | Item menu > Allow for a child | Adds the item to the policy's exception list. | ACC-029 |
| 8 | A parent sees what the child played. | Household > child > History | Lets household adults read each child's history. | ACC-034 |

**When it goes wrong.** A restricted item reaches the device through a side
path. ACC-030 makes "every path obeys the restrictions" an R1 rule with a
cross-profile test on every route, so the R2 rows only add rules to an
enforcement point that already exists.

**Rivals today.** Plex is good here: managed users, rating presets and
PIN-protected profiles, though custom rules need Plex Pass and a request to
hide PIN entry has 111 votes. Jellyfin's rating limits are free and support
sub-ratings, but it has no households (#493, 202 votes) and no managed
profiles. Plex removes folder view while restrictions are on, and Jellyfin
filters home artwork but declined a screensaver filter.

**Other releases.** R1 has per-person library access (ACC-037, MUS-027)
and reads the explicit flag (MUS-047), so a separate children's library is
possible, but there are no managed profiles, presets or PINs. Later: a
daily time allowance (ACC-033), asking a parent (ACC-035), guest mode
(ACC-022), encrypted per-profile stores on shared devices (ACC-132). R3:
live TV channel limits (ACC-036).

**Depends on.** ADR 3; the rule language (DIS-119).

---

## F19. Connecting an existing music app

**Release: R2.**

**Who and where.** The owner enables the adapter; a listener connects
Symfonium or another Subsonic app. This is the interim answer for iPhone
owners, whose native app is Later (CLI-130).

**Starts** with the adapter off, as it is by default. **Ends** with the
third-party app browsing and playing, its plays counted in the same
history, and a key the listener can revoke.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | The owner turns on the OpenSubsonic adapter. | Admin > Compatibility | Registers the adapter routes behind the same policy layer, limiter and cross-user tests as the native API. | INT-086, ACC-130, INT-094 |
| 2 | The listener creates a key for the app. | Account > Apps and tokens ("Connect a music app", with a QR code) | Mints an expiring, revocable API key scoped to read and play. | INT-087, ACC-129, INT-024 |
| 3 | Enters the server address and key in the app. | Third-party app | Validates the key. Serves browsing, search, streaming, cover art, playlists, stars, ratings, scrobbles and the play queue from the native music model through an ID translation layer. Advertises only the extensions it implements. | INT-086, INT-089, INT-090 |
| 4 | Plays music. | Third-party app; History (shows which app) | Writes adapter plays to the user log with the device and app. | INT-093 |
| 5 | Sees, and later revokes, the connected app. | Account > Apps and tokens; Admin > Compatibility | Lists connected apps and revokes keys one by one or all at once. | INT-096, INT-021 |

**When it goes wrong.** The app supports only the old password-derived
sign-in. Which Subsonic apps support API keys is unverified (open decision
4), and legacy password sign-in is Later and off by default (INT-088).

**Rivals today.** Navidrome, gonic, Ampache and LMS already serve Subsonic
apps well, and the apps themselves are mature; borrowing that ecosystem is
the point. Jellyfin has no Subsonic API, and Plex has none.

**Other releases.** R2 also brings the Jellyfin adapter's music subset for
Finamp, Jellify and Feishin (INT-098), and Music Assistant through
OpenSubsonic (INT-097). Later: OpenSubsonic podcasts and bookmarks
(INT-092).

**Depends on.** Open decision 4 (adapter timing and API keys).

---

## F20. Adding an IPTV playlist and watching a channel

**Release: R3.**

**Who and where.** The owner in the admin section, then a viewer on the TV
app.

**Starts** with Live TV switched off, as it is for households without TV.
**Ends** with filtered channels matched to a guide, playing as broadcast on
the TV, and a recording scheduled.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | The owner switches on the Live TV module. | Admin > Modules toggle | Registers live TV routes and jobs only when the module is on. | LIV-001 |
| 2 | Starts the guided set-up and chooses an IPTV playlist. | Admin > Live TV > Set-up wizard | Probes sources and computes guide coverage, saving everything in one transaction at the end. | LIV-002 |
| 3 | Adds an M3U by URL, with headers if the provider needs them. | Admin > Live TV > Sources > Add playlist; Source > Advanced | Fetches through the egress gate with connect-time address checks, so the URL cannot reach the LAN or local files. Keeps provider credentials where clients never see them. | LIV-003, LIV-007, LIV-015, LIV-014 |
| 4 | Tests before saving: channel count, a test tune and any HTTP error. | Add-source wizard: test results panel | Probes with the demuxer and writes nothing until confirmed. | LIV-016 |
| 5 | Filters a 20,000-line playlist down to the groups wanted. | Source > Filters, with a live match count | Applies ordered include and exclude rules, with a preview. | LIV-008, LIV-009 |
| 6 | Adds an XMLTV guide and reviews the automatic matches. | Admin > Live TV > Guide sources; Admin > Channels > Mapping | Parses the guide and matches channels in the core, with a confidence for each. | LIV-035, LIV-051, LIV-050 |
| 7 | Decides who may watch and who may record. | Admin > Users > Live TV | Adds live TV capabilities to each policy. | ACC-048 |
| 8 | The viewer opens the guide on the TV. | Guide grid (TV) | Syncs a compact guide to the device, so it browses instantly even when the server is unreachable. | LIV-064, LIV-065 |
| 9 | Picks a channel, and it plays as broadcast. | Live player | Serves the transport stream from a fan-out buffer. The native client decodes it, so the server does not transcode. | LIV-078, LIV-080 |
| 10 | Records a programme from the guide, free. | Guide and details "Record" | Creates a recording object for the scheduler and recorder. | LIV-102 |

**When it goes wrong.** Watching in a browser costs the server CPU, because
browsers need a remux or a sandboxed conversion, and the docs must say so
(LIV-079). When every tuner is busy, the viewer sees who holds each one and
chooses (LIV-087).

**Rivals today.** Jellyfin and Emby take M3U playlists directly, and
Channels DVR advertises channel changes under a second. Plex needs an
HDHomeRun emulator for M3U (119 votes, open since 2018), and its own
documentation says Live TV needs a server that can transcode. Jellyfin had
a server-side request forgery and file-read hole in its M3U tuner, fixed in
10.11.7 in April 2026, which is why step 3 guards every fetch. Dispatcharr's
ordered regex filters are the model for step 5.

**Depends on.** Open decision 23 (do not start R3 until the remuxer and
sandbox are stable).

---

## Journeys not designed here

These journeys come up in the research but are deliberately not walked
through, because the map places them in Later or No.

| Journey | Release | Why | Features |
|---|---|---|---|
| Watching together with a guest link | Later | The second most-voted Plex suggestion (2,878 votes). The guest capability ships in R2 as its base, and open decision 18 recommends watch together as the first feature after R2. | VID-153, VID-154, ACC-093, ACC-135 |
| Listening together on one queue | Later | It shares its design with watching together, which comes first. | MUS-202, ACC-094 |
| The same music in several rooms, in step | Later | It needs clock sync and group sessions. | CLI-105, MUS-201 |
| Saving music for offline inside a browser | Later | No rival does it, and browser storage can be evicted. | MUS-217 |
| Using an iPhone app, CarPlay, Apple TV or AirPlay | Later; R2 if the App Store licence decision allows | The blocker is the App Store terms and the AGPL with no contributor agreement, not engineering (open decision 3). | CLI-005, CLI-007, CLI-117, CLI-109 |
| Remote access from a browser with no proxy | Later | Browsers on iroh are relay-only, so every byte costs relay bandwidth: affordable for music, not for video. | ACC-102 |
| Deleting a file from the UI | Later | Media is read-only; deletion needs an opt-in writable root (open decision 17). | ADM-139 |
| Signing in with Google or Apple | No | It needs a central account. | ACC-067 |
| Skipping sign-in because you are at home | No | Header spoofing let remote attackers look local in Emby's 2023 compromise. | ACC-066 |
| Opening a router port automatically | No | Open ports were the risk behind the 2023 Emby compromise, and iroh needs none. | ACC-106 |
| Using Plex's apps against Gunmetal | No | Plex's apps sign in through plex.tv, so emulating Plex would mean depending on Plex's cloud. | INT-101 |
| Fixing tags in the app and writing them to the files | No | Media folders stay read-only. | MUS-048, LIB-133 |
| Requesting new titles inside Gunmetal | No | Seerr already does it; Gunmetal links to it in R2. | INT-131, INT-126 |
| Asking a voice assistant run by the project | No | It needs a vendor cloud. Casting to Nest and Google speakers in R2 (CLI-106) is the partial answer. | CLI-131 |

## Surfaces the flows pass through

This is the screen list the flows imply, grouped by where it lives, with
the release that first needs it. Names follow the map's UI surfaces column.

| Surface | First needed | Flows |
|---|---|---|
| **Host and recovery** | | |
| Console and log message (setup code, startup errors) | R1 | F01, F14 |
| Command line: `gunmetal doctor`, `admin recover`, `restore`, `rebuild` | R1 | F01, F03, F12, F13, F14 |
| Startup page | R1 | F01, F12, F13 |
| Emergency page | R1 | F13 |
| **First run** | | |
| Welcome > Language; Welcome > Setup code; the new-or-restore choice | R1 | F01, F14 |
| Welcome > Create admin, with the secure-context panel and Welcome > HTTPS | R1 | F01 |
| Welcome > Privacy | R1 | F01 |
| Welcome > Import | R1 | F01, F11 |
| Welcome > Libraries (folder picker with live checks) | R1 | F01, F02 |
| Welcome > Restore, with root remapping and a progress page | R1 | F14 |
| Welcome > Done | R1 | F01 |
| **Sign-in and account** | | |
| Sign-in page and second sign-in step | R1 | F03, F10, F14 |
| Invite landing | R1 | F10 |
| Secure-context banner; Settings > About this connection | R1 | F01, F03 |
| Account > Sign-in methods | R1 | F01, F03, F10, F15 |
| Account > Devices; Account > Sign-in history | R1 | F03, F04, F15 |
| Confirmation prompt for sensitive changes | R1 | F15 |
| Account > Your data | R1 | F16 |
| Account > Apps and tokens | R1 for tokens; app keys in R2 | F19 |
| **Listening and browsing** | | |
| Home, including the scanning empty state, shortcuts and Continue listening | R1 | F02, F03, F05, F06, F16 |
| Library views with filter sheet, sort, view toggle and folder view | R1 | F02, F06, F09 |
| Artist page; album page | R1 | F02, F05, F09 |
| Playlist page; sidebar | R1 | F06 |
| Add-to-playlist sheet | R1 | F05, F06 |
| Rule editor sheet | R1 | F06; download rules in F07 and restrictions in F18 from R2 |
| Context menu; selection bar | R1 | F05, F06, F09 |
| Search screen and global search field | R1 | F09 |
| Now-playing bar; full-screen player; queue panel or sheet; lyrics view | R1 | F05, F08 |
| Track info sheet | R1 | F05, F12 |
| "Continue on this device" prompt | R1 | F03, F05, F08 |
| History page; Hidden page; undo toast | R1 | F11, F16 |
| Quiet status banner; dimmed and greyed-out items; client banner | R1 | F12 |
| Settings > Storage; Settings > Help > Diagnostics; install prompt | R1 | F03, F07, F12 |
| **Admin** | | |
| Admin > Dashboard (scan card, health summary, now playing, update card) | R1 | F01, F02, F10, F13 |
| Admin > Libraries and library settings | R1 | F02, F14 |
| Library health (problems, tag problems, missing files, sidecar problems) | R1 | F02, F12 |
| Admin > Review queue | R1 | F02, F06, F11 |
| Admin > Activity; Admin > Tasks | R1 | F02, F11, F13, F15 |
| Admin > Users; Admin > Invitations | R1 | F02, F10, F15 |
| Admin > Network | R1 | F01, F10 |
| Admin > Sessions | R1 | F12 |
| Admin > Diagnostics (doctor, bundle, inspect a file) | R1 | F12 |
| Admin > Backups | R1 | F14 |
| Admin > Updates; admin banner | R1 | F13 |
| Admin > Migration | R1 | F11 |
| Admin > Alerts | R1 | F02, F12, F14 |
| **R2 additions** | | |
| TV sign-in screen; phone approval sheet | R2 | F03, F04 |
| Profile picker; PIN pad; Kids home | R2 | F04, F18 |
| TV Home and rail | R2 | F04, F17 |
| Downloads screen and rules; download toggles; Settings > Downloads | R2 | F07, F15 |
| Device picker; remote mode; cast button | R2 | F08 |
| Title detail page; pre-play sheet; video player with info overlay, subtitle menu, skip button, error and limit cards; post-play screen | R2 | F12, F17 |
| Admin > Household; child-profile settings | R2 | F18 |
| Admin > Remote access | R2 | F10 |
| Admin > Compatibility | R2 | F19 |
| **R3 additions** | | |
| Admin > Live TV (set-up wizard, sources, guide sources, mapping) | R3 | F20 |
| Guide grid; live player | R3 | F20 |

## Server work the flows share

Most of the server column repeats across flows. These are the shared pieces
of work, which flows lean on each, and what blocks them.

| Server capability | Features | Flows | First release | Blocked by |
|---|---|---|---|---|
| Setup state machine and the unclaimed-server guard | ACC-001, ADM-020 | F01, F14 | R1 | |
| Identity store and sessions: accounts, credentials, cookie sessions, the session epoch, the device registry, the shared limiter | ACC-002, ACC-050, ACC-052, ACC-053, ACC-057, ACC-063, ACC-065, ACC-068, ACC-079, ACC-124 | F01, F03, F10, F14, F15 | R1 | ADR 3 |
| The user log, with typed, versioned, exportable events | LAT-007, ADM-078, MUS-122, ACC-010 | F05, F06, F07, F11, F13, F14, F16 | R1 | ADR 3 |
| Scan pipeline, job registry and progress events | LIB-019, LIB-020, LIB-021, LIB-022, ADM-093, ADM-095 | F02, F11, F13 | R1 | Open decision 8, for loudness |
| Change feed and sync endpoints, filtered by grants and restrictions | LIB-018, CLI-022, INT-006, ACC-030, ACC-037 | F02, F03, F04, F05, F09, F10, F18 | R1 | |
| Byte serving with short-lived, session-bound, revocable URLs | MUS-066, ACC-122 | F05, F07, F08, F12, F15, F17 | R1 | |
| The audio-only packager for browsers | MUS-230 | F05, F12 | R1 | Open decision 9 |
| The queue document and its versioned operations | MUS-122, MUS-116, LAT-009 | F05, F06, F08 | R1 | ADR 3 |
| The playback decision engine with structured reasons, in the core | MUS-099, ADM-100, INT-134, VID-002 | F05, F08, F12, F17 | R1 for audio; R2 for video | |
| The rule engine, in the core | DIS-119 | F06, F07, F18 | R1 | |
| The matcher, in the core | ADM-044, MUS-140 | F06, F11 | R1 | |
| The egress gate and its activity log | ADM-028, ADM-129, LIV-015 | F01, F13, F20 | R1 | |
| Backup, restore and rebuild | ADM-065, ADM-070, ADM-077, ADM-141 | F13, F14 | R1 | ADR 3 |
| Health records, diagnostics and the inspect API | LIB-193, ADM-108, ADM-123, ADM-124, ADM-125 | F02, F12 | R1 | |
| Invitations as capabilities | ACC-080 | F10 | R1 | ADR 3 |
| One pairing protocol and device keys | ACC-061, ACC-062, ACC-051, CLI-027 | F03, F04 | R2 | |
| The control channel and its conflict rule | CLI-101, CLI-102 | F08 | R2 | Its own design |
| Offline grants | CLI-095, ACC-045 | F07, F15 | R2 | Clients map open decision 4 |
| The remuxer and the transcode sandbox | VID-003, VID-005, CLI-083 | F07, F12, F17 | R2 | |
| The iroh endpoint and relays | ACC-096, ACC-100 | F04, F08, F10 | R2 | Open decision 10 |
| The adapter layer | INT-086, INT-098, ACC-130 | F11, F19 | R2 | Open decision 4 |
| Live TV ingest, buffer and recorder | LIV-003, LIV-078, LIV-102 | F20 | R3 | Open decision 23 |

## Gaps and questions the flows expose

Walking the flows step by step turned up these points that the feature map
does not settle. Each has a recommendation and names the row that should
own the answer.

1. **G1. Half-finished setup and expired codes (F01; owner ACC-001).** The
   map closes setup once an admin exists (ADM-020), but does not say what
   happens if the browser closes after the code is accepted and before the
   owner exists, or how a new code is issued. *Recommendation:* bind the
   claim to the browser session that entered the code, let it lapse with
   the code, and print a fresh code on restart.
2. **G2. Passwords over plain HTTP (F01, F03; owner ADM-021).** Where
   passkeys cannot work, R1 accepts a password, and over plain HTTP it
   crosses the home network unencrypted. *Recommendation:* the
   secure-context panel says so in one line, and the R1 docs recommend
   HTTPS before anyone is invited (ACC-098, ADM-022).
3. **G3. Passkeys after a move (F14; owners ACC-050 and ADM-029).** A
   passkey belongs to the domain it was created for, so restoring under a
   new domain breaks every member's passkeys. *Recommendation:* the restore
   step shows the old domain from the backup, warns when the new address
   differs, and offers to issue one-time sign-in links (ACC-064).
4. **G4. Who sees a new library (F02; owner ACC-037).** The map does not
   say whether a new library is visible to everyone or only to admins.
   *Recommendation:* only the owner and admins until someone is granted
   access, and the add-library sheet asks.
5. **G5. Playlists that come from files (F06; owner LIB-192).** An `.m3u`
   file in a music folder becomes a playlist, but media is read-only
   (LIB-007), so edits cannot be written back. *Recommendation:* show such
   playlists as read-only with "Duplicate to edit", so a rescan never
   fights a person's edits.
6. **G6. Playlist entries after a purge (F06; owner MUS-132).** The map
   does not say what a playlist shows once a track is purged from the
   trash. *Recommendation:* keep the entry as "missing" with its last known
   title and artist, so a later copy rematches by identity (LIB-028).
7. **G7. The address an invite carries (F10; owner ACC-080).** In R1 an
   invite carries whatever public address the server knows, and a LAN-only
   address is useless outside. *Recommendation:* Admin > Invitations shows
   the address the link will carry and warns when it is private.
8. **G8. What the owner can see, in R1 (F10, F16; owner ACC-115).** A
   friend's live sessions are visible to the owner from R1 (ADM-099), but
   the page explaining that is R2. *Recommendation:* one plain line on the
   invite landing page and in account settings in R1, following open
   decision 13.
9. **G9. Approving a TV from an iPhone (F04; owner ACC-061).** R2 has no
   native iPhone app, so the phone approval sheet must also be a web client
   route, and the QR code should be a link that opens the Android app when
   it is installed and the web client otherwise (CLI-034). The web route
   needs a secure context to hold a session dependably (CLI-150).
   *Recommendation:* specify the pairing QR as a deep link and test both
   paths.
10. **G10. URL expiry during playback (F05, F12; owner ACC-122).** Stream
    URLs are short-lived and range responses are capped, so a long pause or
    a three-hour mix will outlive a URL. *Recommendation:* make "a URL
    expiring during playback is invisible to the listener" a named R1
    acceptance test, on both the byte path and the packager.
11. **G11. Two tabs and one queue in R1 (F05, F08; owner MUS-122).**
    Without the R2 control channel, two browser tabs on one profile can
    both play. *Recommendation:* the tab that presses play last takes the
    queue; the other sees the new queue version on its next sync and pauses
    with a "playing on another device" note. This needs only the R1 queue
    versions.
12. **G12. A device that never finished its first sync (F03; owner
    CLI-025).** It cannot fall back to a local copy it does not have.
    *Recommendation:* sync status shows progress and "not yet available
    offline" until the first snapshot lands.
13. **G13. Formats a browser cannot decode (F12; owners MUS-229 and
    MUS-033).** R1 can only explain. MUS-033 sends the R2 extra formats
    through the sandboxed transcoder, but the map does not say whether a
    core format that one browser cannot decode also gets a conversion in
    R2. *Recommendation:* yes, as an Opus stream through the MUS-106 path,
    governed by the same per-person rights as other conversions (VID-010).
14. **G14. A handoff that fails (F08; owner CLI-101).** The map does not
    say what happens when the target does not start. *Recommendation:* the
    source keeps playing until the target confirms, and the device picker
    reports the failure.
15. **G15. Restore needs the setup code (F14; owner ADM-029).** The flows
    assume a restore, like a new setup, needs the console's code first,
    because ACC-001 blocks every route while unclaimed. *Recommendation:*
    say so in ADM-029, so nobody on the LAN can restore their own backup
    onto a fresh install.
16. **G16. What search covers once music and video have separate homes
    (F09; owner DIS-083).** *Recommendation:* search covers everything the
    profile can see; opened from the Music or Watch home, it preselects
    that kind's chip, which one tap clears.
17. **G17. The R1 phone gap (F05, F07, F08).** Most R1 compromises in these
    flows come from phones having only the web app: no downloads, no
    dependable background audio on iPhone, no handoff. *Recommendation:*
    settle open decision 5 early. Shipping the native Android music app as
    an R1 point release changes F05, F07 and F08 more than any other single
    decision.

## How these flows feed the next steps

- **For the UI plan**, the surface table above is the screen list. Each
  step's "The person" cell is that screen's job, and each "When it goes
  wrong" branch is a state the screen must design: empty, loading, offline,
  refused or failed.
- **For the server**, each "The server" cell is a behaviour with an
  observable result, so it can be written as a failing test before any
  code. Following the project's testing rules, each behaviour is proved at
  the lowest layer that can observe it: unit tests in the core for the
  matcher, the rule engine, the decision engine and queue operations; API
  tests for authorisation, sessions, signed URLs and sync filtering; and one
  end-to-end test per flow's main path, with its failure branches covered
  below that level.
- **For parallel work**, the shared server table splits into pieces that
  can be built side by side. Nearly every R1 piece writes to the user log or
  the identity store, so ADR 3 has to be accepted first (open decision 1).
- **For order**, the map's R1 build order still applies: the music model
  and scan (F02), the player (F05, F12), the queue and playlists (F05,
  F06), lyrics, search and home (F09), then sign-in, backups and the
  security baseline (F01, F03, F10, F13 to F16). The flows add one
  constraint: F01's claim and owner account are needed before any flow with
  more than one person can be tested end to end.
